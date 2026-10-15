use async_trait::async_trait;
use tokio::time::{sleep, Duration};

use crate::common::errors::EngineError;
use crate::services::rpc::providers::{
    configured_keys, execute_json_rpc_batch, RpcProvider, RpcRequest, RpcResponse,
};

pub struct InfuraProvider;

impl InfuraProvider { pub fn new() -> Self { Self } }

#[async_trait]
impl RpcProvider for InfuraProvider {
    fn provider_name(&self) -> &'static str { "infura" }

    async fn execute_batch(
        &self,
        requests: Vec<RpcRequest>,
    ) -> Vec<Result<RpcResponse, EngineError>> {
        let keys = match configured_keys("INFURA_API_KEY") {
            Ok(keys) => keys,
            Err(error) => return requests.into_iter().map(|_| Err(error.clone())).collect(),
        };

        let endpoint = requests.first().map(|r| r.endpoint.trim()).unwrap_or("");
        let mut last = None;

        for key in keys {
            let resolved = if endpoint.contains("{api_key}") || endpoint.contains("{API_KEY}") || endpoint.ends_with(&key) {
                endpoint.to_owned()
            } else if endpoint.trim_end_matches('/').ends_with("/v3") {
                format!("{}/{}", endpoint.trim_end_matches('/'), key)
            } else {
                endpoint.to_owned()
            };

            for attempt in 0..3 {
                let results = execute_json_rpc_batch("infura", &resolved, requests.clone(), false, &key).await;
                if results.iter().all(Result::is_ok) {
                    return results;
                }
                last = Some(results);
                if attempt < 2 {
                    sleep(Duration::from_millis(250 * (attempt + 1) as u64)).await;
                }
            }
        }

        last.unwrap_or_else(|| requests.into_iter().map(|_| Err(
            EngineError::Validation("Infura RPC failed for all configured API keys and retries".into())
        )).collect())
    }
}
