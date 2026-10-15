use super::super::substrate_adapter::ChainConfig;

pub fn config(rpc_url: impl Into<String>) -> ChainConfig {
    ChainConfig::polkadot(rpc_url.into())
}
