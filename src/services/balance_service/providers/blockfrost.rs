use async_trait::async_trait;
use futures::future::join_all;

use super::{
    balance_method_config, fetch_configured_native_balance, BalanceProvider,
    BalanceProviderError, BalanceProviderRequest, BalanceProviderResponse,
};

/// Blockfrost-backed balance provider. It intentionally contains no API key
/// or Cardano URL: the catalog defines the endpoint and maps the project_id
/// HTTP header to BLOCKFROST_PROJECT_ID at runtime.
pub struct BlockfrostBalanceProvider;

#[async_trait]
impl BalanceProvider for BlockfrostBalanceProvider {
    async fn fetch_batch(
        &self,
        requests: Vec<BalanceProviderRequest>,
    ) -> Result<Vec<BalanceProviderResponse>, BalanceProviderError> {
        Ok(join_all(requests.into_iter().map(|request| async move {
            let wallet_id = request.request.wallet_id.clone();
            let chain_id = request.request.chain.chain_id.clone();
            let config = balance_method_config(&request.request.chain);
            match config {
                Some(config) if matches!(
                    config.get("executor").and_then(serde_json::Value::as_str),
                    Some("http_get") | Some("http_post")
                ) => match fetch_configured_native_balance(&request, &config).await {
                    Ok(balance) => BalanceProviderResponse::success(balance),
                    Err(error) => BalanceProviderResponse { wallet_id, chain_id, result: Err(error) },
                },
                _ => BalanceProviderResponse::error(
                    &request.request,
                    BalanceProviderError::Message(format!(
                        "Blockfrost has no supported HTTP balance_method for {} ({})",
                        request.request.chain.name, request.request.chain.chain_id
                    )),
                ),
            }
        })).await)
    }
}
