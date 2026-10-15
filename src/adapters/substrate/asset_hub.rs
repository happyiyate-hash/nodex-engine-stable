use super::super::substrate_adapter::ChainConfig;

/// Asset Hub uses the shared Substrate adapter for common account/balance
/// behavior; runtime-specific asset handling is supplied separately.
pub fn config(rpc_url: impl Into<String>) -> ChainConfig {
    let mut config = ChainConfig::polkadot(rpc_url.into());
    config.name = Some("asset-hub".into());
    config
}
