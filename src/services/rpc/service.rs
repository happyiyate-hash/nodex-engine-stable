use std::collections::HashMap;

use crate::services::rpc::providers::{RpcRequest, RpcResponse};
use crate::services::rpc::registry::RpcProviderRegistry;
use crate::services::rpc::providers::RpcProvider;

pub struct RpcOrchestrator {
    registry: RpcProviderRegistry,
}

impl RpcOrchestrator {
    pub fn new(registry: RpcProviderRegistry) -> Self {
        Self { registry }
    }

    pub fn with_defaults() -> Result<Self, crate::common::errors::EngineError> {
        Ok(Self::new(RpcProviderRegistry::with_defaults()?))
    }

    pub async fn execute_one(&self, request: RpcRequest) -> Result<RpcResponse, String> {
        self.execute(vec![request]).await.into_iter().next().unwrap_or_else(|| Err("RPC result missing".into()))
    }

    pub fn registry(&self) -> &RpcProviderRegistry {
        &self.registry
    }

    pub async fn execute(
        &self,
        requests: Vec<RpcRequest>,
    ) -> Vec<Result<RpcResponse, String>> {
        let mut grouped: HashMap<String, Vec<(usize, RpcRequest)>> = HashMap::new();

        for (index, request) in requests.into_iter().enumerate() {
            let provider = request.provider.trim().to_ascii_lowercase();
            let group_key = format!("{}\u{1f}{}", provider, request.endpoint.trim());
            grouped.entry(group_key).or_default().push((index, request));
        }

        let total = grouped.values().map(Vec::len).sum();
        let mut output: Vec<Option<Result<RpcResponse, String>>> =
            (0..total).map(|_| None).collect();

        for (group_key, entries) in grouped {
            let provider_name = group_key.split_once("\u{1f}").map(|(name, _)| name).unwrap_or(group_key.as_str());
            let Some(provider) = self.registry.get(&provider_name) else {
                for (index, _) in entries {
                    output[index] = Some(Err(format!(
                        "no RPC provider registered: {}",
                        provider_name
                    )));
                }
                continue;
            };

            let expected = entries.len();
            let provider_requests: Vec<RpcRequest> =
                entries.iter().map(|(_, request)| request.clone()).collect();

            let results = provider.execute_batch(provider_requests).await;

            if results.len() != expected {
                for (index, _) in entries {
                    output[index] = Some(Err(format!(
                        "RPC provider returned {} results for {} requests",
                        results.len(),
                        expected
                    )));
                }
                continue;
            }

            for ((index, _), result) in entries.into_iter().zip(results) {
                output[index] = Some(result.map_err(|error| error.to_string()));
            }
        }

        output
            .into_iter()
            .map(|result| result.unwrap_or_else(|| Err("RPC result slot was not populated".into())))
            .collect()
    }
}
