use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::common::errors::EngineError;
use crate::services::rpc::providers::{RpcProvider, RpcRequest, RpcResponse};

/// Provider for TRON's native HTTP API.
///
/// TRON permission/account endpoints are native HTTP JSON endpoints
/// (for example /wallet/getaccount), not Ethereum-style JSON-RPC methods.
/// The endpoint supplied in each RpcRequest is therefore used as the
/// provider base URL, while RpcRequest::method is the TRON HTTP path.
pub struct TronProvider;

impl TronProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RpcProvider for TronProvider {
    fn provider_name(&self) -> &'static str {
        "tron"
    }

    async fn execute_batch(
        &self,
        requests: Vec<RpcRequest>,
    ) -> Vec<Result<RpcResponse, EngineError>> {
        if requests.is_empty() {
            return Vec::new();
        }

        let client = Client::new();
        let mut results = Vec::with_capacity(requests.len());

        for request in requests {
            let base = request.endpoint.trim_end_matches('/');
            let path = request.method.trim();
            let url = if path.starts_with("http://") || path.starts_with("https://") {
                path.to_owned()
            } else {
                format!("{}/{}", base, path.trim_start_matches('/'))
            };

            let response = match client.post(&url).json(&request.params).send().await {
                Ok(response) => response,
                Err(error) => {
                    results.push(Err(EngineError::Validation(format!(
                        "TRON RPC request failed: {}",
                        error
                    ))));
                    continue;
                }
            };

            let status = response.status();
            let body: Value = match response.json().await {
                Ok(body) => body,
                Err(error) => {
                    results.push(Err(EngineError::Validation(format!(
                        "TRON returned invalid JSON (HTTP {}): {}",
                        status, error
                    ))));
                    continue;
                }
            };

            if !status.is_success() {
                let message = body
                    .get("Error")
                    .or_else(|| body.get("error"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("TRON provider returned an HTTP error");

                results.push(Err(EngineError::Validation(format!(
                    "TRON RPC HTTP {}: {}",
                    status, message
                ))));
                continue;
            }

            if let Some(error) = body.get("Error").and_then(Value::as_str) {
                results.push(Err(EngineError::Validation(format!(
                    "TRON RPC error: {}",
                    error
                ))));
                continue;
            }

            results.push(Ok(RpcResponse {
                provider: request.provider,
                chain_id: request.chain_id,
                result: body,
            }));
        }

        results
    }
}
