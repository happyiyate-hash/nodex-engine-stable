//! Chain-specific balance normalization for the cosmos adapter.
//!
//! Transport and provider authentication remain in balance_service providers;
//! this module owns the adapter-specific conversion into the shared balance model.
use crate::common::errors::EngineError;
use crate::services::balance_service::service::{BalanceAsset, ChainBalance};

pub fn normalize(mut balance: ChainBalance) -> Result<ChainBalance, EngineError> {
    let native = balance.native.take().ok_or_else(|| EngineError::Validation("cosmos balance response has no native asset".into()))?;
    let rows: Vec<crate::adapters::cosmos::AssetRow> = std::iter::once(native.clone()).chain(balance.tokens.iter().cloned()).map(|a| crate::adapters::cosmos::AssetRow {
        address: a.address, symbol: a.symbol.unwrap_or_else(|| String::new()), decimals: a.decimals, is_native: a.is_native, balance: a.balance
    }).collect();
    let rows = crate::adapters::cosmos::CosmosAdapter::normalize_balances(rows);
    let mut iter = rows.into_iter();
    let first = iter.next().ok_or_else(|| EngineError::Validation("cosmos adapter returned no normalized native balance".into()))?;
    balance.native = Some(BalanceAsset { address: first.address, symbol: if first.symbol.is_empty() { native.symbol.clone() } else { Some(first.symbol) }, name: native.name, decimals: first.decimals, balance: first.balance, is_native: true });
    balance.tokens = iter.map(|r| BalanceAsset { address: r.address, symbol: if r.symbol.is_empty() { None } else { Some(r.symbol) }, name: None, decimals: r.decimals, balance: r.balance, is_native: false }).collect();
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
