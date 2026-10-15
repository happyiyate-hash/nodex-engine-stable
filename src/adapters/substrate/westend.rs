use super::super::substrate_adapter::ChainConfig;

pub fn config(rpc_url: impl Into<String>) -> ChainConfig {
    let mut config = ChainConfig::polkadot(rpc_url.into());
    config.name = Some("westend".into());
    config.native_symbol = "WND".into();
    config
}
