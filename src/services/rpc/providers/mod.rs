use async_trait::async_trait;
use std::time::Duration;
use serde_json::{json, Value};

use crate::common::errors::EngineError;

#[derive(Debug, Clone)]
pub struct RpcRequest {
    pub provider: String,
    pub chain_type: String,
    pub chain_id: String,
    pub endpoint: String,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone)]
pub struct RpcResponse {
    pub provider: String,
    pub chain_id: String,
    pub result: Value,
}

#[async_trait]
pub trait RpcProvider: Send + Sync {
    fn provider_name(&self) -> &'static str;

    async fn execute_batch(
        &self,
        requests: Vec<RpcRequest>,
    ) -> Vec<Result<RpcResponse, EngineError>>;
}

pub(crate) fn configured_keys(env_name: &str) -> Result<Vec<String>, EngineError> {
    let plural = format!("{}S", env_name);
    let mut keys = Vec::new();

    if let Ok(value) = std::env::var(&plural) {
        keys.extend(value.split(',').map(str::trim).filter(|v| !v.is_empty()).map(str::to_owned));
    }

    for index in 1..=20 {
        let name = format!("{}_{}", env_name, index);
        if let Ok(value) = std::env::var(name) {
            let value = value.trim();
            if !value.is_empty() {
                keys.push(value.to_owned());
            }
        }
    }

    if keys.is_empty() {
        if let Ok(value) = std::env::var(env_name) {
            let value = value.trim();
            if !value.is_empty() {
                keys.push(value.to_owned());
            }
        }
    }

    keys.sort();
    keys.dedup();

    if keys.is_empty() {
        return Err(EngineError::Validation(format!(
            "missing required environment variable: {} (or numbered/multi-key variants)",
            env_name
        )));
    }

    Ok(keys)
}

pub(crate) fn resolve_endpoint(endpoint: &str, key: &str) -> String {
    let endpoint = endpoint.trim();
    if endpoint.contains("{api_key}") {
        return endpoint.replace("{api_key}", key);
    }
    if endpoint.contains("{API_KEY}") {
        return endpoint.replace("{API_KEY}", key);
    }
    endpoint.to_owned()
}

fn build_json_rpc_payload(requests: &[RpcRequest]) -> Value {
    if requests.len() == 1 {
        let request = &requests[0];
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": request.method,
            "params": request.params
        })
    } else {
        Value::Array(requests.iter().enumerate().map(|(id, request)| {
            json!({
                "jsonrpc": "2.0",
                "id": id as u64,
                "method": request.method,
                "params": request.params
            })
        }).collect())
    }
}

async fn execute_json_rpc_individually(
    provider_name: &str,
    endpoint: &str,
    requests: Vec<RpcRequest>,
    auth_header: bool,
    key: &str,
) -> Vec<Result<RpcResponse, EngineError>> {
    let client = match reqwest::Client::builder().timeout(Duration::from_secs(30)).build() {
        Ok(client) => client,
        Err(error) => return requests.into_iter().map(|_| Err(EngineError::Validation(format!("failed to initialize RPC HTTP client: {}", error)))).collect(),
    };
    let mut output = Vec::with_capacity(requests.len());
    for request in requests {
        let url = resolve_endpoint(endpoint, key);
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": request.method,
            "params": request.params
        });
        let mut builder = client.post(&url).json(&payload);
        if auth_header && !url.contains(key) {
            builder = builder.bearer_auth(key);
        }
        let response = match builder.send().await {
            Ok(response) => response,
            Err(error) => {
                output.push(Err(EngineError::Validation(format!("{} RPC request failed: {}", provider_name, error))));
                continue;
            }
        };
        let status = response.status();
        let body_text = match response.text().await {
            Ok(body) => body,
            Err(error) => {
                output.push(Err(EngineError::Validation(format!("{} failed reading RPC response (HTTP {}): {}", provider_name, status, error))));
                continue;
            }
        };
        let body: Value = match serde_json::from_str(&body_text) {
            Ok(body) => body,
            Err(error) if !status.is_success() => {
                let excerpt: String = body_text.chars().take(240).collect();
                output.push(Err(EngineError::Validation(format!("{} RPC HTTP {} (non-JSON response): {} [{}]", provider_name, status, error, excerpt))));
                continue;
            }
            Err(error) => {
                output.push(Err(EngineError::Validation(format!("{} returned invalid JSON (HTTP {}): {}", provider_name, status, error))));
                continue;
            }
        };
        if !status.is_success() {
            output.push(Err(EngineError::Validation(format!("{} RPC HTTP {}: {}", provider_name, status, body.get("error").and_then(|e| e.get("message")).and_then(Value::as_str).unwrap_or("provider returned an HTTP error")))));
            continue;
        }
        if let Some(error) = body.get("error") {
            output.push(Err(EngineError::Validation(format!("{} JSON-RPC error on {}: {}", provider_name, request.method, error.get("message").and_then(Value::as_str).unwrap_or("JSON-RPC provider error")))));
            continue;
        }
        match body.get("result") {
            Some(result) => output.push(Ok(RpcResponse { provider: provider_name.to_owned(), chain_id: request.chain_id, result: result.clone() })),
            None => output.push(Err(EngineError::Validation(format!("{} omitted JSON-RPC result for {}", provider_name, request.method)))),
        }
    }
    output
}

pub(crate) async fn execute_json_rpc_batch(
    provider_name: &str,
    endpoint: &str,
    requests: Vec<RpcRequest>,
    auth_header: bool,
    key: &str,
) -> Vec<Result<RpcResponse, EngineError>> {
    if requests.is_empty() {
        return Vec::new();
    }

    let url = resolve_endpoint(endpoint, key);
    // Several public endpoints reject batch arrays even for one request.
    let payload = build_json_rpc_payload(&requests);

    let client = match reqwest::Client::builder().timeout(Duration::from_secs(30)).build() {
        Ok(client) => client,
        Err(error) => return requests.into_iter().map(|_| Err(EngineError::Validation(format!("failed to initialize RPC HTTP client: {}", error)))).collect(),
    };
    let mut request_builder = client.post(&url).json(&payload);

    if auth_header && !url.contains(key) {
        request_builder = request_builder.bearer_auth(key);
    }

    let response = match request_builder.send().await {
        Ok(response) => response,
        Err(error) => {
            return requests.iter().map(|_| Err(EngineError::Validation(
                format!("{} RPC request failed: {}", provider_name, error)
            ))).collect();
        }
    };

    let status = response.status();
    let body_text = match response.text().await {
        Ok(body) => body,
        Err(error) => {
            return requests.iter().map(|_| Err(EngineError::Validation(
                format!("{} failed reading RPC response (HTTP {}): {}", provider_name, status, error)
            ))).collect();
        }
    };
    let body: Value = match serde_json::from_str(&body_text) {
        Ok(body) => body,
        Err(error) if !status.is_success() => {
            let excerpt: String = body_text.chars().take(240).collect();
            return requests.iter().map(|_| Err(EngineError::Validation(
                format!("{} RPC HTTP {} (non-JSON response): {} [{}]", provider_name, status, error, excerpt)
            ))).collect();
        }
        Err(error) => {
            return requests.iter().map(|_| Err(EngineError::Validation(
                format!("{} returned invalid JSON (HTTP {}): {}", provider_name, status, error)
            ))).collect();
        }
    };

    if !status.is_success() {
        if requests.len() > 1 {
            return execute_json_rpc_individually(provider_name, &url, requests, auth_header, key).await;
        }
        let message = body.get("error").and_then(|e| e.get("message")).and_then(Value::as_str)
            .unwrap_or("provider returned an HTTP error");
        return requests.iter().map(|_| Err(EngineError::Validation(
            format!("{} RPC HTTP {}: {}", provider_name, status, message)
        ))).collect();
    }

    let Some(items) = body.as_array() else {
        if requests.len() == 1 {
            let request = &requests[0];
            if let Some(error) = body.get("error") {
                return vec![Err(EngineError::Validation(format!("{} JSON-RPC error on {}: {}", provider_name, request.method, error.get("message").and_then(Value::as_str).unwrap_or("JSON-RPC provider error"))))];
            }
            return match body.get("result") {
                Some(result) => vec![Ok(RpcResponse { provider: provider_name.to_owned(), chain_id: request.chain_id.clone(), result: result.clone() })],
                None => vec![Err(EngineError::Validation(format!("{} did not return a JSON-RPC result", provider_name)))],
            };
        }
        return execute_json_rpc_individually(provider_name, &url, requests, auth_header, key).await;
    };

    let mut by_id = std::collections::HashMap::<u64, &Value>::new();
    for item in items {
        if let Some(id) = item.get("id").and_then(Value::as_u64) {
            by_id.insert(id, item);
        }
    }

    requests.iter().enumerate().map(|(id, request)| {
        let item = by_id.get(&(id as u64)).ok_or_else(|| EngineError::Validation(
            format!("{} omitted JSON-RPC response for request {}", provider_name, id)
        ))?;

        if let Some(error) = item.get("error") {
            let message = error.get("message").and_then(Value::as_str)
                .unwrap_or("JSON-RPC provider error");
            return Err(EngineError::Validation(format!(
                "{} JSON-RPC error on {}: {}", provider_name, request.method, message
            )));
        }

        Ok(RpcResponse {
            provider: provider_name.to_owned(),
            chain_id: request.chain_id.clone(),
            result: item.get("result").cloned().unwrap_or(Value::Null),
        })
    }).collect()
}

pub mod alchemy;
pub mod blockfrost;
pub mod infura;
pub mod tron;
pub mod public;

pub use alchemy::AlchemyProvider;
pub use blockfrost::BlockfrostProvider;
pub use infura::InfuraProvider;
pub use tron::TronProvider;
pub use public::PublicProvider;

pub mod xrp;
pub use xrp::XrpProvider;

#[cfg(test)]
mod json_rpc_payload_tests {
    use super::{build_json_rpc_payload, RpcRequest};
    use serde_json::json;

    fn request(method: &str) -> RpcRequest {
        RpcRequest {
            provider: "public".into(),
            chain_type: "near".into(),
            chain_id: "397".into(),
            endpoint: "https://rpc.mainnet.near.org".into(),
            method: method.into(),
            params: json!({"request_type": "view_account"}),
        }
    }

    #[test]
    fn one_json_rpc_call_is_an_object_not_a_batch_array() {
        let payload = build_json_rpc_payload(&[request("query")]);
        assert!(payload.is_object());
        assert_eq!(payload["method"], "query");
        assert_eq!(payload["id"], 1);
    }

    #[test]
    fn multiple_json_rpc_calls_remain_a_batch_array() {
        let payload = build_json_rpc_payload(&[request("query"), request("status")]);
        assert!(payload.is_array());
        assert_eq!(payload.as_array().unwrap().len(), 2);
        assert_eq!(payload[0]["id"], 0);
        assert_eq!(payload[1]["id"], 1);
    }
}
