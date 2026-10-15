//! Chain-specific balance normalization for the bep2 adapter.
//!
//! Transport and provider authentication remain in balance_service providers;
//! this module owns the adapter-specific conversion into the shared balance model.
use crate::common::errors::EngineError;
use crate::services::balance_service::service::{BalanceAsset, ChainBalance};

pub fn normalize(mut balance: ChainBalance) -> Result<ChainBalance, EngineError> {
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
