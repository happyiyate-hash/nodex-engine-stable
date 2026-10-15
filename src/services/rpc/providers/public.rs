use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use crate::common::errors::EngineError;
use crate::services::rpc::providers::{execute_json_rpc_batch, RpcProvider, RpcRequest, RpcResponse};

/// Public endpoints use the same RPC orchestrator as credentialed providers.
/// In addition to JSON-RPC, this provider supports explicit HTTP GET/POST
/// operations for chains whose public nodes expose REST APIs.
pub struct PublicProvider;

impl PublicProvider {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl RpcProvider for PublicProvider {
    fn provider_name(&self) -> &'static str { "public" }

    async fn execute_batch(&self, requests: Vec<RpcRequest>) -> Vec<Result<RpcResponse, EngineError>> {
        if requests.is_empty() { return Vec::new(); }
        let endpoint = requests.first().map(|r| r.endpoint.clone()).unwrap_or_default();
        if endpoint.trim().is_empty() {
            return requests.into_iter().map(|_| Err(EngineError::Validation("public RPC endpoint cannot be empty".into()))).collect();
        }

        // JSON-RPC batches are efficient and remain handled by the shared
        // transport. REST operations use the request URL as endpoint and body
        // as params, preserving a single provider/transport boundary.
        if requests.iter().all(|r| r.method == "__http_get" || r.method == "__http_post") {
            let client = match reqwest::Client::builder().timeout(Duration::from_secs(30)).build() {
                Ok(client) => client,
                Err(error) => return requests.into_iter().map(|_| Err(EngineError::Validation(format!("failed to initialize public HTTP client: {}", error)))).collect(),
            };
            let mut results = Vec::with_capacity(requests.len());
            for request in requests {
                let body = request.params.get("__http_body").unwrap_or(&request.params);
                let mut builder = if request.method == "__http_get" {
                    client.get(&request.endpoint)
                } else {
                    client.post(&request.endpoint).json(body)
                };
                if let Some(headers) = request.params.get("__headers").and_then(Value::as_object) {
                    for (name, value) in headers {
                        if let Some(value) = value.as_str() {
                            builder = builder.header(name, value);
                        }
                    }
                }
                let response = match builder.send().await {
                    Ok(response) => response,
                    Err(error) => {
                        results.push(Err(EngineError::Validation(format!("public HTTP request failed: {}", error))));
                        continue;
                    }
                };
                let status = response.status();
                let body_text = match response.text().await {
                    Ok(body) => body,
                    Err(error) => {
                        results.push(Err(EngineError::Validation(format!("public endpoint response read failed (HTTP {}): {}", status, error))));
                        continue;
                    }
                };
                let value: Value = match serde_json::from_str(&body_text) {
                    Ok(value) => value,
                    Err(error) if !status.is_success() => {
                        let excerpt: String = body_text.chars().take(240).collect();
                        results.push(Err(EngineError::Validation(format!("public endpoint HTTP {} returned non-JSON content: {} [{}]", status, error, excerpt))));
                        continue;
                    }
                    Err(error) => {
                        results.push(Err(EngineError::Validation(format!("public endpoint returned invalid JSON (HTTP {}): {}", status, error))));
                        continue;
                    }
                };
                if !status.is_success() {
                    results.push(Err(EngineError::Validation(format!("public endpoint HTTP {}: {}", status, value))));
                } else if let Some(error) = value.get("error") {
                    results.push(Err(EngineError::Validation(format!("public endpoint error: {}", error))));
                } else {
                    results.push(Ok(RpcResponse { provider: "public".into(), chain_id: request.chain_id, result: value }));
                }
            }
            return results;
        }

        execute_json_rpc_batch("public", &endpoint, requests, false, "").await
    }
}
