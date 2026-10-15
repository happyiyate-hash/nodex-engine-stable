use async_trait::async_trait;
use crate::common::errors::EngineError;
use super::{execute_json_rpc_batch, RpcProvider, RpcRequest, RpcResponse};

pub struct XrpProvider;

impl XrpProvider {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl RpcProvider for XrpProvider {
    fn provider_name(&self) -> &'static str { "xrp" }

    async fn execute_batch(&self, requests: Vec<RpcRequest>) -> Vec<Result<RpcResponse, EngineError>> {
        let endpoint = requests.first().map(|r| r.endpoint.clone()).unwrap_or_default();
        execute_json_rpc_batch("xrp", &endpoint, requests, false, "").await
    }
}
