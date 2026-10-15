use async_trait::async_trait;
use futures::future::join_all;

use super::{
    balance_method_config, fetch_configured_native_balance, fetch_evm_balances_batch,
    BalanceProvider, BalanceProviderError, BalanceProviderRequest, BalanceProviderResponse,
};

/// Public-provider adapter. The blockchain catalog defines each balance method;
/// this adapter only dispatches to the shared EVM executor or generic configured
/// JSON-RPC/HTTP executor. It contains no per-blockchain balance implementations.
pub struct PublicBalanceProvider;

impl PublicBalanceProvider {
    async fn fetch_configured_one(request: BalanceProviderRequest) -> BalanceProviderResponse {
        let wallet_id = request.request.wallet_id.clone();
        let chain_id = request.request.chain.chain_id.clone();
        let Some(config) = balance_method_config(&request.request.chain) else {
            return BalanceProviderResponse::error(
                &request.request,
                BalanceProviderError::Message(format!(
                    "blockchain catalog has no balance_method for {} ({})",
                    request.request.chain.name, request.request.chain.chain_id
                )),
            );
        };

        let executor = config.get("executor").and_then(serde_json::Value::as_str).unwrap_or("");
        match executor {
            "evm" => match fetch_evm_balances_batch(vec![request.clone()]).await {
                Ok(mut responses) => responses.pop().unwrap_or_else(|| BalanceProviderResponse {
                    wallet_id,
                    chain_id,
                    result: Err(BalanceProviderError::Message(
                        "shared EVM balance executor returned no response".into()
                    )),
                }),
                Err(error) => BalanceProviderResponse {
                    wallet_id,
                    chain_id,
                    result: Err(BalanceProviderError::Message(error.to_string())),
                },
            },
            "json_rpc" | "http_post" | "http_get" => {
                match fetch_configured_native_balance(&request, &config).await {
                    Ok(balance) => BalanceProviderResponse::success(balance),
                    Err(error) => BalanceProviderResponse { wallet_id, chain_id, result: Err(error) },
                }
            }
            other => BalanceProviderResponse {
                wallet_id,
                chain_id,
                result: Err(BalanceProviderError::Message(format!(
                    "unsupported balance_method executor '{}' for {}",
                    other, request.request.chain.name
                ))),
            },
        }
    }
}

#[async_trait]
impl BalanceProvider for PublicBalanceProvider {
    async fn fetch_batch(
        &self,
        requests: Vec<BalanceProviderRequest>,
    ) -> Result<Vec<BalanceProviderResponse>, BalanceProviderError> {
        Ok(join_all(requests.into_iter().map(Self::fetch_configured_one)).await)
    }
}
