use std::{convert::Infallible, env, sync::Arc};
use tokio::sync::Mutex;
use warp::{Filter, Rejection, Reply};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use nodex_wallet_engine::crypto::mnemonic::generate_mnemonic;
use nodex_wallet_engine::events::bus::EventBus;
use nodex_wallet_engine::wallet_engine::{WalletProvider, WalletRequest, WalletResponse};

type SharedProvider = Arc<Mutex<Option<WalletProvider>>>;

#[derive(Debug, Deserialize)]
struct ProviderRequest {
    action: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct ApiResponse {
    success: bool,
    data: Option<Value>,
    error: Option<String>,
}

fn redact_secret_fields(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for key in ["mnemonic", "seed", "private_key", "secret_phrase"] {
                object.remove(key);
            }
            for nested in object.values_mut() {
                redact_secret_fields(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_secret_fields(item);
            }
        }
        _ => {}
    }
}

fn summarize_wallet_response(response: &WalletResponse, include_secret_phrase: bool) -> Value {
    // Never echo plaintext recovery material through the raw response object.
    // The UI must explicitly opt in to the one-time recovery phrase field.
    let mut raw = response.data.clone().unwrap_or_default();
    redact_secret_fields(&mut raw);

    let mut summary = serde_json::json!({
        "status": if response.success { "success" } else { "error" },
        "success": response.success,
        "message": response.error.as_deref().unwrap_or("wallet operation completed"),
        "secret_phrase": null,
        "addresses": null,
        "raw": raw,
    });

    if let Some(data) = &response.data {
        if include_secret_phrase {
            if let Some(mnemonic) = data.get("mnemonic").and_then(|value| value.as_str()) {
                summary["secret_phrase"] = Value::String(mnemonic.to_string());
            }
        }
        if let Some(addresses) = data.get("addresses") {
            summary["addresses"] = addresses.clone();
        }
    }

    summary
}

#[tokio::main]
async fn main() {
    let state: SharedProvider = Arc::new(Mutex::new(None));
    // Set NODEX_LOCAL_MODE=true only for an embedded/on-device engine. In this
    // mode the HTTP server binds to loopback and local UI clients need no token.
    let local_mode = env::var("NODEX_LOCAL_MODE")
        .map(|value| value.eq_ignore_ascii_case("true") || value == "1")
        .unwrap_or(false);

    let api_route = warp::path("api")
        .and(warp::path("request"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(handle_request);

    // Generic Server-Sent Events endpoint. Clients subscribe once and receive
    // engine events regardless of which subsystem published them. Never publish
    // private keys, seed phrases, PINs, or other secret material to this bus.
    let events_route = warp::path("api")
        .and(warp::path("events"))
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::header::optional::<String>("authorization"))
        .map(move |authorization: Option<String>| {
            // Local packaged mode binds only to loopback and can use the event
            // stream without a manually configured token. Remote mode remains
            // protected by NODEX_EVENT_STREAM_TOKEN.
            let expected = env::var("NODEX_EVENT_STREAM_TOKEN")
                .ok()
                .filter(|token| !token.trim().is_empty());
            let authorized = if local_mode {
                true
            } else {
                match (expected.as_deref(), authorization.as_deref()) {
                    (Some(token), Some(header)) => header.strip_prefix("Bearer ") == Some(token),
                    _ => false,
                }
            };
            if !authorized {
                return warp::reply::with_status(
                    "event stream unauthorized",
                    warp::http::StatusCode::UNAUTHORIZED,
                ).into_response();
            }

            let receiver = EventBus::subscribe();
            let stream = futures::stream::unfold(receiver, |mut receiver| async move {
                loop {
                    match receiver.recv().await {
                        Ok(event) => {
                            let event_id = event.event_id.to_string();
                            let event_type = event.event_type.clone();
                            let payload = serde_json::to_string(&event)
                                .unwrap_or_else(|_| "{}".to_string());
                            let message = warp::sse::Event::default()
                                .id(event_id)
                                .event(event_type)
                                .data(payload);
                            return Some((Ok::<_, Infallible>(message), receiver));
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
                    }
                }
            });
            warp::sse::reply(warp::sse::keep_alive().stream(stream)).into_response()
        });

    let health_route = warp::path("health")
        .and(warp::path::end())
        .map(|| warp::reply::json(&json!({
            "status": "ok",
            "service": "nodex-wallet-engine",
            "mode": "web-test"
        })));

    let root_route = warp::path::end().map(|| warp::redirect::temporary(warp::http::Uri::from_static("https://sable-wallet-frontend.onrender.com")));
    let static_route = warp::path("static").and(warp::fs::dir("static/dev_ui"));

    let routes = root_route
        .or(health_route)
        .or(events_route)
        .or(api_route)
        .or(static_route)
        .with(warp::cors().allow_any_origin().allow_methods(vec!["GET", "POST", "OPTIONS"]).allow_headers(vec!["content-type", "authorization"]).max_age(86400))
        .recover(handle_rejection);

    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3030);

    let bind_address = if local_mode { [127, 0, 0, 1] } else { [0, 0, 0, 0] };
    println!("NodeX Wallet Engine UI available on {}:{port}", if local_mode { "127.0.0.1" } else { "0.0.0.0" });
    warp::serve(routes).run((bind_address, port)).await;
}

fn with_state(
    state: SharedProvider,
) -> impl Filter<Extract = (SharedProvider,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn handle_request(
    payload: ProviderRequest,
    state: SharedProvider,
) -> Result<impl Reply, Infallible> {
    let wallet_request = WalletRequest {
        action: payload.action,
        params: payload.params,
    };

    let action = wallet_request.action.clone();
    let include_secret_phrase = wallet_request.params
        .get("include_secret_phrase")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let api_response = match action.as_str() {
        "create_wallet" | "generate_wallet" => {
            let word_count = wallet_request
                .params
                .get("word_count")
                .and_then(|value| value.as_u64())
                .map(|count| count as usize)
                .unwrap_or(12);
            let _ = word_count;

            match generate_mnemonic(12).and_then(|mnemonic| WalletProvider::new(&mnemonic)) {
                Ok(mut provider) => match provider.handle_request(wallet_request) {
                    Ok(wallet_response) => {
                        *state.lock().await = Some(provider);
                        build_api_response(wallet_response, include_secret_phrase)
                    }
                    Err(error) => error_response(error.to_string()),
                },
                Err(error) => error_response(error.to_string()),
            }
        }
        "import_wallet" => {
            let mnemonic = wallet_request
                .params
                .get("mnemonic")
                .and_then(|value| value.as_str())
                .unwrap_or_default();

            if mnemonic.is_empty() {
                error_response("missing mnemonic".to_string())
            } else {
                match WalletProvider::new(mnemonic) {
                    Ok(mut provider) => match provider.handle_request(wallet_request) {
                        Ok(wallet_response) => {
                            *state.lock().await = Some(provider);
                            build_api_response(wallet_response, include_secret_phrase)
                        }
                        Err(error) => error_response(error.to_string()),
                    },
                    Err(error) => error_response(error.to_string()),
                }
            }
        }
        "import_private_key" => {
            match generate_mnemonic(12).and_then(|mnemonic| WalletProvider::new(&mnemonic)) {
                Ok(mut provider) => match provider.handle_request(wallet_request) {
                    Ok(wallet_response) => {
                        *state.lock().await = Some(provider);
                        build_api_response(wallet_response, include_secret_phrase)
                    }
                    Err(error) => error_response(error.to_string()),
                },
                Err(error) => error_response(error.to_string()),
            }
        }
        "restore_wallet" => {
            let mnemonic = wallet_request
                .params
                .get("mnemonic")
                .and_then(|value| value.as_str());

            match mnemonic {
                None => error_response("missing mnemonic".to_string()),
                Some(mnemonic) => {
                    let mut provider_guard = state.lock().await;

                    if provider_guard.is_none() {
                        match WalletProvider::new(mnemonic) {
                            Ok(provider) => *provider_guard = Some(provider),
                            Err(error) => return Ok(warp::reply::json(&error_response(error.to_string()))),
                        }
                    }

                    match provider_guard.as_mut().unwrap().handle_request(wallet_request) {
                        Ok(wallet_response) => build_api_response(wallet_response, include_secret_phrase),
                        Err(error) => error_response(error.to_string()),
                    }
                }
            }
        }
        _ => {
            let mut provider_guard = state.lock().await;

            if provider_guard.is_none() {
                match generate_mnemonic(12).and_then(|mnemonic| WalletProvider::new(&mnemonic)) {
                    Ok(provider) => *provider_guard = Some(provider),
                    Err(error) => return Ok(warp::reply::json(&error_response(error.to_string()))),
                }
            }

            match provider_guard.as_mut().unwrap().handle_request_async(wallet_request).await {
                Ok(wallet_response) => build_api_response(wallet_response, include_secret_phrase),
                Err(error) => error_response(format!("{action}: {error}")),
            }
        }
    };

    Ok(warp::reply::json(&api_response))
}

fn build_api_response(wallet_response: WalletResponse, include_secret_phrase: bool) -> ApiResponse {
    ApiResponse {
        success: wallet_response.success,
        data: Some(summarize_wallet_response(&wallet_response, include_secret_phrase)),
        error: wallet_response.error,
    }
}

fn error_response(message: String) -> ApiResponse {
    ApiResponse {
        success: false,
        data: Some(json!({
            "status": "error",
            "message": message
        })),
        error: Some(message),
    }
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let message = if err.is_not_found() {
        "not found".to_string()
    } else {
        format!("request rejected: {:?}", err)
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&error_response(message)),
        warp::http::StatusCode::BAD_REQUEST,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn recovery_phrase_is_not_returned_unless_explicitly_requested() {
        let response = WalletResponse::ok(json!({
            "mnemonic": "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            "seed": "must-not-be-returned",
            "private_key": "must-not-be-returned",
            "addresses": {
                "evm": "0x1234",
                "bitcoin": "bc1qtest",
                "solana": "SolanaAddress"
            },
            "nested": { "mnemonic": "nested-secret", "private_key": "nested-key" }
        }));

        let summary = summarize_wallet_response(&response, false);
        assert!(summary["secret_phrase"].is_null());
        assert!(summary["raw"].get("mnemonic").is_none());
        assert!(summary["raw"].get("seed").is_none());
        assert!(summary["raw"].get("private_key").is_none());
        assert!(summary["raw"]["nested"].get("mnemonic").is_none());
        assert!(summary["raw"]["nested"].get("private_key").is_none());
        assert_eq!(summary["addresses"]["evm"], "0x1234");

        let opted_in = summarize_wallet_response(&response, true);
        assert_eq!(opted_in["secret_phrase"], response.data.as_ref().unwrap()["mnemonic"]);
        assert!(opted_in["raw"].get("mnemonic").is_none());
    }
}
