//! Chain-specific balance normalization for the utxo adapter.
//!
//! Transport and provider authentication remain in balance_service providers;
//! this module owns the adapter-specific conversion into the shared balance model.
use crate::common::errors::EngineError;
use crate::services::balance_service::service::{BalanceAsset, ChainBalance};

pub fn normalize(mut balance: ChainBalance) -> Result<ChainBalance, EngineError> {
    if let Some(mut native) = balance.native.take() {
        let decimals = native.decimals.unwrap_or(8);
        native.balance = crate::adapters::utxo::UtxoAdapter::normalize_balance(&native.balance, decimals)?;
        native.decimals = Some(decimals);
        balance.native = Some(native);
    }
    Ok(balance)
}

/// Fetch a batch through the selected balance-service provider, then normalize
/// each successful result with this adapter's chain-specific policy.
pub async fn fetch_batch(
    provider: std::sync::Arc<dyn crate::services::balance_service::providers::BalanceProvider>,
    requests: Vec<crate::services::balance_service::providers::BalanceProviderRequest>,
) -> Result<Vec<crate::services::balance_service::providers::BalanceProviderResponse>, crate::services::balance_service::providers::BalanceProviderError> {
    crate::adapters::balance_support::fetch_batch(provider, requests, normalize).await
}
