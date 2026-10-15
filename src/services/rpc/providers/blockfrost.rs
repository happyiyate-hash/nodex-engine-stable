use async_trait::async_trait;

use crate::common::errors::EngineError;
use super::{public::PublicProvider, RpcProvider, RpcRequest, RpcResponse};

/// Blockfrost transport provider. Request details (URL path and project_id
/// header) are supplied by the blockchain catalog and shared HTTP executor;
/// credentials are read from the named runtime environment variable.
pub struct BlockfrostProvider;

impl BlockfrostProvider {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl RpcProvider for BlockfrostProvider {
    fn provider_name(&self) -> &'static str { "blockfrost" }

    async fn execute_batch(&self, requests: Vec<RpcRequest>) -> Vec<Result<RpcResponse, EngineError>> {
        let provider = PublicProvider::new();
        provider.execute_batch(requests).await.into_iter().map(|result| {
            result.map(|mut response| {
                response.provider = "blockfrost".into();
                response
            })
        }).collect()
    }
}
