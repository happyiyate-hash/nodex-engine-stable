use async_trait::async_trait;
use std::time::Duration;

use crate::common::errors::EngineError;
use crate::services::rpc::providers::{
    configured_keys, execute_json_rpc_batch, resolve_endpoint, RpcProvider, RpcRequest, RpcResponse,
};

pub struct AlchemyProvider;

impl AlchemyProvider { pub fn new() -> Self { Self } }

impl AlchemyProvider {
    async fn execute_http_batch(
        requests: Vec<RpcRequest>,
        endpoint: &str,
        key: &str,
    ) -> Vec<Result<RpcResponse, EngineError>> {
        let client = match reqwest::Client::builder().timeout(Duration::from_secs(30)).build() {
            Ok(client) => client,
            Err(error) => return requests.into_iter().map(|_| Err(EngineError::Validation(
                format!("failed to initialize Alchemy HTTP client: {error}")
            ))).collect(),
        };
        let mut output = Vec::with_capacity(requests.len());
        for request in requests {
            let url = resolve_endpoint(if request.endpoint.trim().is_empty() { endpoint } else { &request.endpoint }, key);
            let body = request.params.get("__http_body").unwrap_or(&request.params);
            let mut builder = if request.method == "__http_get" {
                client.get(&url)
            } else {
                client.post(&url).json(body)
            };
            if let Some(headers) = request.params.get("__headers").and_then(serde_json::Value::as_object) {
                for (name, value) in headers {
                    if let Some(value) = value.as_str() {
                        builder = builder.header(name, value);
                    }
                }
            }
            let response = match builder.send().await {
                Ok(response) => response,
                Err(error) => {
                    output.push(Err(EngineError::Validation(format!("Alchemy HTTP request to {} failed: {}", url, error))));
                    continue;
                }
            };
            let status = response.status();
            let body: serde_json::Value = match response.json().await {
                Ok(body) => body,
                Err(error) => {
                    output.push(Err(EngineError::Validation(format!("Alchemy HTTP endpoint returned invalid JSON (HTTP {}): {}", status, error))));
                    continue;
                }
            };
            if !status.is_success() {
                output.push(Err(EngineError::Validation(format!("Alchemy HTTP {}: {}", status, body))));
            } else if let Some(error) = body.get("Error").and_then(serde_json::Value::as_str) {
                output.push(Err(EngineError::Validation(format!("Alchemy HTTP API error: {}", error))));
            } else {
                output.push(Ok(RpcResponse {
                    provider: "alchemy".into(),
                    chain_id: request.chain_id,
                    result: body,
                }));
            }
        }
        output
    }
}

#[async_trait]
impl RpcProvider for AlchemyProvider {
    fn provider_name(&self) -> &'static str { "alchemy" }

    async fn execute_batch(
        &self,
        requests: Vec<RpcRequest>,
    ) -> Vec<Result<RpcResponse, EngineError>> {
        if requests.is_empty() { return Vec::new(); }
        let keys = match configured_keys("ALCHEMY_API_KEY") {
            Ok(keys) => keys,
            Err(error) => return requests.into_iter().map(|_| Err(error.clone())).collect(),
        };

        let is_http = requests.iter().all(|request| request.method == "__http_post" || request.method == "__http_get");
        let endpoint = requests.first().map(|request| request.endpoint.as_str()).unwrap_or("");
        let mut last = None;

        for key in keys {
            let results = if is_http {
                Self::execute_http_batch(requests.clone(), endpoint, &key).await
            } else {
                execute_json_rpc_batch("alchemy", endpoint, requests.clone(), true, &key).await
            };
            if results.iter().all(Result::is_ok) {
                return results;
            }
            last = Some(results);
        }

        last.unwrap_or_else(|| requests.into_iter().map(|_| Err(
            EngineError::Validation("Alchemy RPC failed for all configured API keys".into())
        )).collect())
    }
}
