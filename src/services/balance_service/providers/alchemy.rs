use async_trait::async_trait;
use futures::future::join_all;

use super::{
    balance_method_config, fetch_configured_native_balance, fetch_evm_balances_batch,
    BalanceProvider, BalanceProviderError, BalanceProviderRequest, BalanceProviderResponse,
};

pub struct AlchemyBalanceProvider;

#[async_trait]
impl BalanceProvider for AlchemyBalanceProvider {
    async fn fetch_batch(
        &self,
        requests: Vec<BalanceProviderRequest>,
    ) -> Result<Vec<BalanceProviderResponse>, BalanceProviderError> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }

        // Balance semantics come from static/dev_ui/blockchains.json. The
        // provider adapter handles only provider routing, not chain-specific rules.
        let mut evm_requests = Vec::new();
        let mut configured_requests = Vec::new();
        for request in requests {
            let config = balance_method_config(&request.request.chain);
            let executor = config.as_ref().and_then(|value| value.get("executor"))
                .and_then(serde_json::Value::as_str).map(str::to_owned);
            match executor.as_deref() {
                Some("evm") => evm_requests.push(request),
                Some("json_rpc") | Some("http_post") | Some("http_get") => {
                    if let Some(config) = config {
                        configured_requests.push((request, config));
                    }
                },
                _ => {
                    let error = BalanceProviderError::Message(format!(
                        "blockchain catalog has no supported balance_method for {} ({})",
                        request.request.chain.name, request.request.chain.chain_id
                    ));
                    return Ok(vec![BalanceProviderResponse::error(&request.request, error)]);
                }
            }
        }

        let mut output = Vec::new();
        if !evm_requests.is_empty() {
            output.extend(fetch_evm_balances_batch(evm_requests).await?);
        }

        let configured_results = join_all(configured_requests.into_iter().map(|(request, config)| async move {
            let wallet_id = request.request.wallet_id.clone();
            let chain_id = request.request.chain.chain_id.clone();
            match fetch_configured_native_balance(&request, &config).await {
                Ok(balance) => BalanceProviderResponse::success(balance),
                Err(error) => BalanceProviderResponse { wallet_id, chain_id, result: Err(error) },
            }
        })).await;
        output.extend(configured_results);
        Ok(output)
    }
}
