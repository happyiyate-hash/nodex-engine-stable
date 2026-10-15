//! Chain-specific balance normalization for the solana adapter.
//!
//! Transport and provider authentication remain in balance_service providers;
//! this module owns the adapter-specific conversion into the shared balance model.
use crate::common::errors::EngineError;
use crate::services::balance_service::service::{BalanceAsset, ChainBalance};

pub fn normalize(mut balance: ChainBalance) -> Result<ChainBalance, EngineError> {
    let native = balance.native.take().ok_or_else(|| EngineError::Validation("solana balance response has no native asset".into()))?;
    let rows: Vec<crate::adapters::solana::AssetRow> = std::iter::once(native.clone()).chain(balance.tokens.iter().cloned()).map(|a| crate::adapters::solana::AssetRow {
        address: a.address, symbol: a.symbol.unwrap_or_else(|| if a.is_native { "SOL".into() } else { String::new() }), decimals: a.decimals .or(if a.is_native { Some(9) } else { None }), is_native: a.is_native, balance: a.balance
    }).collect();
    let rows = crate::adapters::solana::SolanaAdapter::normalize_balances(rows);
    let mut iter = rows.into_iter();
    let first = iter.next().ok_or_else(|| EngineError::Validation("solana adapter returned no normalized native balance".into()))?;
    balance.native = Some(BalanceAsset { address: first.address, symbol: if first.symbol.is_empty() { native.symbol.clone() } else { Some(first.symbol) }, name: native.name, decimals: first.decimals, balance: first.balance, is_native: true });
    // The shared AssetRow normalization type has no metadata-name field.
    // Preserve provider-enriched token names/symbols by joining normalized rows
    // back to the original assets using the mint address.
    balance.tokens = iter.map(|r| {
        let original = balance.tokens.iter().find(|asset| asset.address == r.address);
        BalanceAsset {
            address: r.address,
            symbol: if r.symbol.is_empty() {
                original.and_then(|asset| asset.symbol.clone())
            } else {
                Some(r.symbol)
            },
            name: original.and_then(|asset| asset.name.clone()),
            decimals: r.decimals.or_else(|| original.and_then(|asset| asset.decimals)),
            balance: r.balance,
            is_native: false,
        }
    }).collect();
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
