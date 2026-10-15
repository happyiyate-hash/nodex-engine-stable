use super::super::substrate_adapter::ChainConfig;

pub fn config(rpc_url: impl Into<String>) -> ChainConfig {
    ChainConfig::kusama(rpc_url.into())
}
