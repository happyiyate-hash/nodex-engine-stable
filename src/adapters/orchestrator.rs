use crate::adapters::registry::AdapterRegistry;
use crate::types::chain::ChainRequest;
use crate::common::errors::EngineError;
use crate::services::rpc::RpcOrchestrator;
use crate::services::balance_service::service::ChainBalance;

#[derive(Clone, Debug, Default)]
pub struct AdapterRuntimeConfig {
    pub rpc_provider: String,
    pub rpc_url: String,
    pub chain_name: Option<String>,
}

/// Generic adapter boundary. It routes by chain_type and deliberately treats
/// the request body as opaque so WalletEngine stays chain-agnostic.
#[derive(Debug)]
pub enum AdapterTargetRequest<'a> {
    Derive { seed: &'a [u8], chains: &'a [ChainRequest] },
    Normalize { balances: &'a [ChainBalance] },
}

#[derive(Debug)]
pub enum AdapterTargetResponse {
    Addresses(std::collections::HashMap<String, String>),
    Balances(Vec<ChainBalance>),
}

pub struct AdapterOrchestrator {
    rpc: RpcOrchestrator,
}

impl AdapterOrchestrator {
    pub fn new(rpc: RpcOrchestrator) -> Self {
        Self { rpc }
    }

    pub fn with_default_rpc() -> Result<Self, EngineError> {
        Ok(Self::new(RpcOrchestrator::with_defaults()?))
    }


    /// Single generic adapter boundary. Wallet/service orchestrators use this
    /// operation for adapter-owned operations. chain_type selects the adapter;
    /// the operation body remains outside the wallet engine.
    pub fn adapter_target(
        &self,
        request: AdapterTargetRequest<'_>,
    ) -> Result<AdapterTargetResponse, EngineError> {
        match request {
            AdapterTargetRequest::Derive { seed, chains } => {
                // This layer is deliberately chain-agnostic. The registry only
                // routes the complete batch to the adapters selected by
                // chain_type; adapter-specific grouping and derivation policy
                // live inside the adapter itself.
                let addresses = AdapterRegistry::derive_addresses(seed, chains)?;
                Ok(AdapterTargetResponse::Addresses(addresses))
            }

            AdapterTargetRequest::Normalize { balances } => {
                let mut grouped: std::collections::HashMap<String, Vec<ChainBalance>> =
                    std::collections::HashMap::new();

                for balance in balances {
                    grouped.entry(balance.chain_type.trim().to_ascii_lowercase())
                        .or_default()
                        .push(balance.clone());
                }

                let mut normalized = Vec::with_capacity(balances.len());
                for (chain_type, group) in grouped {
                    for balance in group {
                        let native = balance.native.ok_or_else(|| EngineError::Validation(
                            format!("{} balance response has no native asset", chain_type)
                        ))?;
                        normalized.push(AdapterRegistry::normalize_balance(
                            balance.wallet_id,
                            balance.chain_id,
                            chain_type.clone(),
                            balance.address,
                            native,
                            balance.tokens,
                        )?);
                    }
                }

                Ok(AdapterTargetResponse::Balances(normalized))
            }
        }
    }

    /// Uniform derivation boundary used by wallet services. The caller sends
    /// one action contract; chain_type is the only selector needed by the
    /// registry, while the selected adapter owns the implementation.
    pub fn derive(
        &self,
        seed: &[u8],
        action: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, EngineError> {
        let chain_type = params
            .get("chain_type")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| EngineError::Validation("derivation request requires chain_type".into()))?;

        AdapterRegistry::dispatch_derivation(chain_type, seed, action, params)
    }

    pub fn rpc(&self) -> &RpcOrchestrator {
        &self.rpc
    }

    pub async fn call(
        &self,
        chain_type: &str,
        body: &[u8],
        config: AdapterRuntimeConfig,
    ) -> Result<Vec<u8>, EngineError> {
        if chain_type.trim().is_empty() {
            return Err(EngineError::Validation("chain_type is required for adapter_call".into()));
        }
        AdapterRegistry::communicate(chain_type, &self.rpc, body, config).await
    }
}
