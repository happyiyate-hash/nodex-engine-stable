use crate::services::balance_service::service::{BalanceAsset, ChainBalance};
use crate::adapters::evm::AssetRow as EvmAssetRow;
use crate::adapters::solana::AssetRow as SolanaAssetRow;
use crate::adapters::xrp::AssetRow as XrpAssetRow;
use crate::adapters::cosmos::AssetRow as CosmosAssetRow;
use crate::adapters::near::AssetRow as NearAssetRow;
use crate::adapters::sui::AssetRow as SuiAssetRow;
use crate::adapters::substrate::AssetRow as SubstrateAssetRow;
use crate::adapters::tezos::AssetRow as TezosAssetRow;

use crate::adapters::adapter_trait::WalletAdapter;
use crate::adapters::algorand::AlgorandAdapter;
use crate::adapters::aptos::AptosAdapter;
use crate::adapters::bep2::Bep2Adapter;
use crate::adapters::cardano::CardanoAdapter;
use crate::adapters::cosmos::CosmosAdapter;
use crate::adapters::evm::EvmAdapter;
use crate::adapters::hedera::HederaAdapter;
use crate::adapters::near::NearAdapter;
use crate::adapters::substrate::SubstrateAdapter;
use crate::adapters::solana::SolanaAdapter;
use crate::adapters::starknet::StarknetAdapter;
use crate::adapters::sui::SuiAdapter;
use crate::adapters::tezos::TezosAdapter;
use crate::adapters::tron::TronAdapter;
use crate::adapters::utxo::UtxoAdapter;
use crate::adapters::xrp::XrpAdapter;

use crate::common::errors::EngineError;
use crate::types::chain::{ChainFamily, ChainInfo, ChainRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    Evm,
    Utxo,
    Solana,
    Xrp,
    Cosmos,

    Substrate,
    Algorand,
    Near,
    Aptos,
    Sui,
    Tezos,
    Tron,
    Starknet,
    Hedera,
    Cardano,
    Bep2,
}

pub struct AdapterRegistry;

impl AdapterRegistry {
    /// Normalize provider output through the chain-specific adapter selected by
    /// the central adapter registry. Balance Service does not maintain its own
    /// adapter registry.
    /// Route normalization to the balance module owned by the selected adapter.
    /// Provider-specific transport remains in balance_service::providers.
    pub fn normalize_balance(
        wallet_id: String,
        chain_id: String,
        chain_type: String,
        address: String,
        native: crate::services::balance_service::service::BalanceAsset,
        tokens: Vec<crate::services::balance_service::service::BalanceAsset>,
    ) -> Result<crate::services::balance_service::service::ChainBalance, EngineError> {
        let balance = crate::services::balance_service::service::ChainBalance {
            wallet_id, chain_id, chain_type: chain_type.clone(), address,
            native: Some(native), tokens,
        };
        match Self::resolve(&chain_type) {
            Some(AdapterKind::Evm) => crate::adapters::evm_balance::normalize(balance),
            Some(AdapterKind::Utxo) => crate::adapters::utxo_balance::normalize(balance),
            Some(AdapterKind::Solana) => crate::adapters::solana_balance::normalize(balance),
            Some(AdapterKind::Xrp) => crate::adapters::xrp_balance::normalize(balance),
            Some(AdapterKind::Cosmos) => crate::adapters::cosmos_balance::normalize(balance),
            Some(AdapterKind::Substrate) => crate::adapters::substrate_balance::normalize(balance),
            Some(AdapterKind::Algorand) => crate::adapters::algorand_balance::normalize(balance),
            Some(AdapterKind::Near) => crate::adapters::near_balance::normalize(balance),
            Some(AdapterKind::Aptos) => crate::adapters::aptos_balance::normalize(balance),
            Some(AdapterKind::Sui) => crate::adapters::sui_balance::normalize(balance),
            Some(AdapterKind::Tezos) => crate::adapters::tezos_balance::normalize(balance),
            Some(AdapterKind::Tron) => crate::adapters::tron_balance::normalize(balance),
            Some(AdapterKind::Starknet) => crate::adapters::starknet_balance::normalize(balance),
            Some(AdapterKind::Hedera) => crate::adapters::hedera_balance::normalize(balance),
            Some(AdapterKind::Cardano) => crate::adapters::cardano_balance::normalize(balance),
            Some(AdapterKind::Bep2) => crate::adapters::bep2_balance::normalize(balance),
            None => Err(EngineError::Unsupported(format!("no adapter for chain_type {}", chain_type))),
        }
    }

    /// Route balance retrieval to the adapter-owned balance module. The adapter
    /// builds/normalizes the balance job while the selected balance provider
    /// owns endpoint communication, authentication, and transport policy.
    pub async fn fetch_balance_batch(
        chain_type: &str,
        provider: std::sync::Arc<dyn crate::services::balance_service::providers::BalanceProvider>,
        requests: Vec<crate::services::balance_service::providers::BalanceProviderRequest>,
    ) -> Result<Vec<crate::services::balance_service::providers::BalanceProviderResponse>, crate::services::balance_service::providers::BalanceProviderError> {
        match Self::resolve(chain_type) {
            Some(AdapterKind::Evm) => crate::adapters::evm_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Utxo) => crate::adapters::utxo_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Solana) => crate::adapters::solana_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Xrp) => crate::adapters::xrp_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Cosmos) => crate::adapters::cosmos_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Substrate) => crate::adapters::substrate_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Algorand) => crate::adapters::algorand_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Near) => crate::adapters::near_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Aptos) => crate::adapters::aptos_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Sui) => crate::adapters::sui_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Tezos) => crate::adapters::tezos_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Tron) => crate::adapters::tron_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Starknet) => crate::adapters::starknet_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Hedera) => crate::adapters::hedera_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Cardano) => crate::adapters::cardano_balance::fetch_batch(provider, requests).await,
            Some(AdapterKind::Bep2) => crate::adapters::bep2_balance::fetch_batch(provider, requests).await,
            None => Err(crate::services::balance_service::providers::BalanceProviderError::Message(format!("no adapter for chain_type {}", chain_type))),
        }
    }

    /// Dispatch an adapter-specific JSON body to the adapter selected by chain_type.
    /// WalletEngine must not interpret the body; this registry is the adapter boundary.
    pub async fn communicate(
        chain_type: &str,
        rpc: &crate::services::rpc::RpcOrchestrator,
        body: &[u8],
        config: crate::adapters::orchestrator::AdapterRuntimeConfig,
    ) -> Result<Vec<u8>, EngineError> {
        match Self::resolve(chain_type) {
            Some(AdapterKind::Tron) => {
                let adapter = TronAdapter::new(crate::adapters::tron::ChainConfig {
                    rpc_url: config.rpc_url,
                    rpc_provider: config.rpc_provider,
                    name: config.chain_name,
                });
                adapter.permission_json(rpc, body).await
            }
            Some(kind) => Err(EngineError::Unsupported(format!(
                "adapter_call is not implemented for adapter {:?} yet",
                kind
            ))),
            None => Err(EngineError::Unsupported(format!("no adapter for chain_type {}", chain_type))),
        }
    }

}

impl AdapterRegistry {
    pub fn resolve(chain_type: &str) -> Option<AdapterKind> {
        match chain_type.trim().to_ascii_lowercase().as_str() {
            "evm" | "ethereum" | "polygon" | "base" | "arbitrum" | "optimism" | "avalanche" | "bnb" | "bsc" => Some(AdapterKind::Evm),
            "btc" | "bitcoin" | "bitcoin-cash" | "bch" | "litecoin" | "ltc" | "doge" | "dogecoin" | "utxo" => Some(AdapterKind::Utxo),
            "solana" => Some(AdapterKind::Solana),
            "xrp" => Some(AdapterKind::Xrp),
            "cosmos" | "atom" | "osmosis" | "osmo" | "secret" | "scrt" | "injective" | "inj" | "celestia" | "tia" => Some(AdapterKind::Cosmos),
            "substrate" | "polkadot" | "dot" | "kusama" | "ksm" | "westend" | "asset-hub" => Some(AdapterKind::Substrate),
            "algorand" | "algo" => Some(AdapterKind::Algorand),
            "near" => Some(AdapterKind::Near),
            "aptos" | "apt" => Some(AdapterKind::Aptos),
            "sui" => Some(AdapterKind::Sui),
            "tezos" | "xtz" => Some(AdapterKind::Tezos),
            "tron" | "trx" => Some(AdapterKind::Tron),
            "starknet" | "strk" => Some(AdapterKind::Starknet),
            "hedera" | "hbar" => Some(AdapterKind::Hedera),
            "cardano" | "ada" => Some(AdapterKind::Cardano),
            "bep2" => Some(AdapterKind::Bep2),
            _ => None,
        }
    }

    pub fn supported_chain_types() -> &'static [&'static str] {
        &[
            "evm", "btc", "utxo", "solana", "xrp",
            "cosmos", "substrate",
            "algorand", "near", "aptos", "sui", "tezos", "tron", "starknet",
            "hedera", "cardano", "bep2",
        ]
    }

    pub fn derive_address(chain_type: &str, seed: &[u8], path: &str) -> Result<String, EngineError> {
        match Self::resolve(chain_type) {
            Some(AdapterKind::Evm) => EvmAdapter::derive_address_from_path(seed, path),
            Some(AdapterKind::Utxo) => <UtxoAdapter as WalletAdapter>::derive_address_from_path(seed, path),
            Some(AdapterKind::Solana) => SolanaAdapter::derive_address_from_path(seed, path),
            Some(AdapterKind::Xrp) => XrpAdapter::derive_address_from_path(seed, path),
            Some(AdapterKind::Cosmos) => CosmosAdapter::derive_address_for_chain(seed, path, chain_type),
            Some(AdapterKind::Substrate) => SubstrateAdapter::derive_address_from_path(seed, path),
            Some(AdapterKind::Algorand) => AlgorandAdapter::derive_address_from_path(seed, path),
            Some(AdapterKind::Near) => NearAdapter::derive_address_from_path(seed, path),
            Some(AdapterKind::Aptos) => AptosAdapter::derive_address_from_path(seed, path),
            Some(AdapterKind::Sui) => SuiAdapter::derive_address_from_path(seed, path),
            Some(AdapterKind::Tezos) => TezosAdapter::derive_address_from_path(seed, path),
            Some(AdapterKind::Tron) => TronAdapter::derive_address_from_path(seed, path),
            Some(AdapterKind::Starknet) => StarknetAdapter::derive_address_from_path(seed, path),
            Some(AdapterKind::Hedera) => HederaAdapter::derive_address_from_path(seed, path),
            Some(AdapterKind::Cardano) => CardanoAdapter::derive_address_from_path(seed, path),
            Some(AdapterKind::Bep2) => Bep2Adapter::derive_address_from_path(seed, path),
            None => Err(EngineError::Unsupported(format!("no adapter for chain_type {}", chain_type))),
        }
    }

    /// Route a complete derivation batch to adapters. The registry only
    /// groups requests by their registered chain_type; it does not implement
    /// derivation rules or decide whether addresses should be shared. Each
    /// adapter receives its complete batch and owns those decisions.
    pub fn derive_addresses(
        seed: &[u8],
        chains: &[ChainRequest],
    ) -> Result<std::collections::HashMap<String, String>, EngineError> {
        let mut grouped: std::collections::HashMap<String, Vec<ChainRequest>> =
            std::collections::HashMap::new();

        for chain in chains {
            let normalized = chain.clone();
            grouped.entry(normalized.chain_type.clone()).or_default().push(normalized);
        }

        let mut addresses = std::collections::HashMap::new();

        for (chain_type, group) in grouped {
            let kind = Self::resolve(&chain_type)
                .ok_or_else(|| EngineError::Unsupported(format!("no adapter for chain_type {}", chain_type)))?;

            let derived = match kind {
                AdapterKind::Evm => EvmAdapter::derive_addresses(seed, &group)?,
                AdapterKind::Utxo => UtxoAdapter::derive_addresses(seed, &group)?,
                AdapterKind::Cosmos => CosmosAdapter::derive_addresses(seed, &group)?,
                _ => {
                    let mut out = std::collections::HashMap::with_capacity(group.len());
                    for chain in &group {
                        let path = match chain.derivation_path.as_deref() {
                            Some(path) if !path.trim().is_empty() => path.to_owned(),
                            Some(_) => return Err(EngineError::Validation(
                                format!("derivation_path cannot be empty for {} address derivation", chain_type)
                            )),
                            None => default_derivation_path(&chain_type).ok_or_else(|| EngineError::Validation(
                                format!("derivation_path is required for {} address derivation", chain_type)
                            ))?,
                        };
                        let address = Self::derive_address(&chain_type, seed, &path)?;
                        out.insert(chain.name.clone(), address);
                    }
                    out
                }
            };

            for (name, address) in derived {
                if addresses.insert(name.clone(), address).is_some() {
                    return Err(EngineError::Validation(format!(
                        "duplicate blockchain target: {}",
                        name
                    )));
                }
            }
        }

        Ok(addresses)
    }

    pub fn public_key_to_address(chain_type: &str, public_key: &[u8]) -> Result<String, EngineError> {
        match Self::resolve(chain_type) {
            Some(AdapterKind::Evm) => EvmAdapter::pub_key_to_address(public_key),
            Some(AdapterKind::Utxo) => UtxoAdapter::pub_key_to_address(public_key),
            Some(AdapterKind::Solana) => SolanaAdapter::pub_key_to_address(public_key),
            Some(AdapterKind::Xrp) => XrpAdapter::pub_key_to_address(public_key),
            Some(AdapterKind::Cosmos) => CosmosAdapter::pub_key_to_address_for_chain(public_key, chain_type),
            Some(AdapterKind::Substrate) => SubstrateAdapter::pub_key_to_address(public_key),
            Some(AdapterKind::Algorand) => AlgorandAdapter::pub_key_to_address(public_key),
            Some(AdapterKind::Near) => NearAdapter::pub_key_to_address(public_key),
            Some(AdapterKind::Aptos) => AptosAdapter::pub_key_to_address(public_key),
            Some(AdapterKind::Sui) => SuiAdapter::pub_key_to_address(public_key),
            Some(AdapterKind::Tezos) => TezosAdapter::pub_key_to_address(public_key),
            Some(AdapterKind::Tron) => TronAdapter::pub_key_to_address(public_key),
            Some(AdapterKind::Starknet) => StarknetAdapter::pub_key_to_address(public_key),
            Some(AdapterKind::Hedera) => HederaAdapter::pub_key_to_address(public_key),
            Some(AdapterKind::Cardano) => CardanoAdapter::pub_key_to_address(public_key),
            Some(AdapterKind::Bep2) => Bep2Adapter::pub_key_to_address(public_key),
            None => Err(EngineError::Unsupported(format!("no adapter for chain_type {}", chain_type))),
        }
    }

    pub fn address_from_private_key(chain_type: &str, private_key: &[u8]) -> Result<String, EngineError> {
        match Self::resolve(chain_type) {
            Some(AdapterKind::Evm) => EvmAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Utxo) => UtxoAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Solana) => SolanaAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Xrp) => XrpAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Cosmos) => CosmosAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Substrate) => SubstrateAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Algorand) => AlgorandAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Near) => NearAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Aptos) => AptosAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Sui) => SuiAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Tezos) => TezosAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Tron) => TronAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Starknet) => StarknetAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Hedera) => HederaAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Cardano) => CardanoAdapter::address_from_private_key(private_key),
            Some(AdapterKind::Bep2) => Bep2Adapter::address_from_private_key(private_key),
            None => Err(EngineError::Unsupported(format!("no adapter for chain_type {}", chain_type))),
        }
    }

    pub fn sign_transaction(chain_type: &str, private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        match Self::resolve(chain_type) {
            Some(AdapterKind::Evm) => EvmAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Utxo) => UtxoAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Solana) => SolanaAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Xrp) => XrpAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Cosmos) => CosmosAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Substrate) => SubstrateAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Algorand) => AlgorandAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Near) => NearAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Aptos) => AptosAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Sui) => SuiAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Tezos) => TezosAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Tron) => TronAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Starknet) => StarknetAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Hedera) => HederaAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Cardano) => CardanoAdapter::sign_transaction(private_key, tx_data),
            Some(AdapterKind::Bep2) => Bep2Adapter::sign_transaction(private_key, tx_data),
            None => Err(EngineError::Unsupported(format!("no adapter for chain_type {}", chain_type))),
        }
    }
}

pub fn default_derivation_path(chain_type: &str) -> Option<String> {
    match chain_type.trim().to_ascii_lowercase().as_str() {
        "evm" => Some(EvmAdapter::CANONICAL_DERIVATION_PATH.into()),
        "solana" => Some("m/44'/501'/0'/0'".into()),
        "tron" | "trx" => <TronAdapter as WalletAdapter>::default_derivation_path().map(str::to_owned),
        _ => None,
    }
}

pub fn normalize_chain(request: &ChainRequest) -> Result<ChainRequest, EngineError> {
    let family = request.family();
    if family == ChainFamily::Other { return Err(EngineError::Unsupported(format!("unsupported chain_type: {}", request.chain_type))); }
    if request.name.trim().is_empty() { return Err(EngineError::Validation("chain name cannot be empty".into())); }
    let mut normalized=request.clone(); normalized.chain_type=family.as_str().to_string(); let _=normalized.path()?; Ok(normalized)
}

/// Resolve the numeric EVM chain IDs supported by the engine's network registry.
pub fn get_chain_info(chain_id: u64) -> Result<Option<ChainInfo>, EngineError> {
    let (name, symbol) = match chain_id {
        1 => ("Ethereum", "ETH"),
        10 => ("Optimism", "OP"),
        56 => ("BNB Smart Chain", "BNB"),
        137 => ("Polygon", "POL"),
        8453 => ("Base", "ETH"),
        42161 => ("Arbitrum One", "ETH"),
        43114 => ("Avalanche C-Chain", "AVAX"),
        _ => return Ok(None),
    };
    Ok(Some(ChainInfo {
        name: name.into(),
        chain_id: chain_id.to_string(),
        chain_type: ChainFamily::Evm.as_str().into(),
        derivation_path: default_derivation_path("evm").expect("EVM has a default path"),
    }))
}

pub fn supported_chains() -> Result<Vec<ChainInfo>, EngineError> {
    [1u64, 10, 56, 137, 8453, 42161, 43114]
        .into_iter()
        .map(|id| get_chain_info(id).map(|info| info.expect("registered chain must resolve")))
        .collect()
}

pub fn supported_chain_ids() -> Result<Vec<u64>, EngineError> {
    Ok(vec![1, 10, 56, 137, 8453, 42161, 43114])
}

pub fn is_chain_supported(chain_id: u64) -> Result<bool, EngineError> {
    Ok(get_chain_info(chain_id)?.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_resolves_known_evm_chain() {
        let chain = get_chain_info(1).unwrap().unwrap();
        assert_eq!(chain.name, "Ethereum");
        assert_eq!(chain.chain_type, "evm");
        assert_eq!(chain.chain_id, "1");
    }

    #[test]
    fn registry_rejects_unknown_chain_without_error() {
        assert!(get_chain_info(999_999).unwrap().is_none());
        assert!(!is_chain_supported(999_999).unwrap());
    }

    #[test]
    fn batch_derivation_uses_tron_adapter_default_path_when_omitted() {
        let chain = ChainRequest::new("TRON", "tron");
        let addresses = AdapterRegistry::derive_addresses(&[7u8; 32], &[chain])
            .expect("TRON should derive without an explicit path");
        let address = addresses.get("TRON").expect("TRON address should be returned");
        assert_eq!(address.len(), 34);
        assert!(address.starts_with('T'));
    }

    #[test]
    fn tron_default_path_is_resolved_for_trx_alias() {
        assert_eq!(
            default_derivation_path("trx").as_deref(),
            Some("m/44'/195'/0'/0/0")
        );
    }
}

impl AdapterRegistry {
    /// Dispatch the same derivation action contract to the adapter selected by
    /// chain_type. The registry selects the implementation; adapter code owns
    /// the chain-specific derivation rules.
    /// Discover existing children using a chain adapter's network-aware policy.
    pub async fn discover_child_accounts(
        chain_type: &str,
        rpc: &crate::services::rpc::RpcOrchestrator,
        config: crate::adapters::orchestrator::AdapterRuntimeConfig,
        seed: &[u8],
        start_index: u32,
        max_count: u32,
        gap_limit: u32,
    ) -> Result<serde_json::Value, EngineError> {
        match Self::resolve(chain_type) {
            Some(AdapterKind::Tron) => {
                let adapter = TronAdapter::new(crate::adapters::tron::ChainConfig {
                    rpc_url: config.rpc_url,
                    rpc_provider: config.rpc_provider,
                    name: config.chain_name,
                });
                let children = adapter.discover_child_accounts(
                    rpc, seed, start_index, max_count, gap_limit
                ).await?;
                serde_json::to_value(children)
                    .map_err(|e| EngineError::Unexpected(format!("failed to encode discovered TRON children: {e}")))
            }
            Some(kind) => Err(EngineError::Unsupported(format!(
                "on-chain child discovery is not implemented for adapter {:?} yet",
                kind
            ))),
            None => Err(EngineError::Unsupported(format!("no adapter for chain_type {}", chain_type))),
        }
    }

    pub fn dispatch_derivation(
        chain_type: &str,
        seed: &[u8],
        action: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, EngineError> {
        match Self::resolve(chain_type) {
            Some(AdapterKind::Evm) => EvmAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Utxo) => UtxoAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Solana) => SolanaAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Xrp) => XrpAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Cosmos) => CosmosAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Substrate) => SubstrateAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Algorand) => AlgorandAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Near) => NearAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Aptos) => AptosAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Sui) => SuiAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Tezos) => TezosAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Tron) => TronAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Starknet) => StarknetAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Hedera) => HederaAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Cardano) => CardanoAdapter::handle_derivation_request(seed, action, params),
            Some(AdapterKind::Bep2) => Bep2Adapter::handle_derivation_request(seed, action, params),
            None => Err(EngineError::Unsupported(format!("no adapter for chain_type {}", chain_type))),
        }
    }
}
