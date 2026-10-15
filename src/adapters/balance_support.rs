//! Shared adapter balance orchestration. Chain adapters supply normalization;
//! configured balance providers supply the actual network transport and provider policy.
use std::sync::Arc;
use crate::common::errors::EngineError;
use crate::services::balance_service::providers::{BalanceProvider, BalanceProviderError, BalanceProviderRequest, BalanceProviderResponse};
use crate::services::balance_service::service::ChainBalance;

pub async fn fetch_batch(
    provider: Arc<dyn BalanceProvider>,
    requests: Vec<BalanceProviderRequest>,
    normalize: fn(ChainBalance) -> Result<ChainBalance, EngineError>,
) -> Result<Vec<BalanceProviderResponse>, BalanceProviderError> {
    let responses = provider.fetch_batch(requests).await?;
    Ok(responses.into_iter().map(|response| {
        let wallet_id = response.wallet_id.clone();
        let chain_id = response.chain_id.clone();
        match response.result {
            Ok(balance) => match normalize(balance) {
                Ok(balance) => BalanceProviderResponse::success(balance),
                Err(error) => BalanceProviderResponse {
                    wallet_id,
                    chain_id,
                    result: Err(BalanceProviderError::Message(error.to_string())),
                },
            },
            Err(error) => BalanceProviderResponse {
                wallet_id,
                chain_id,
                result: Err(error),
            },
        }
    }).collect())
}
