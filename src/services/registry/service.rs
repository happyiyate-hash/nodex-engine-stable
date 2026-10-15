use crate::adapters::orchestrator::AdapterOrchestrator;
use crate::adapters::registry::AdapterRegistry;
use crate::adapters::registry::normalize_chain;
use crate::common::errors::EngineError;
use crate::types::chain::{ChainInfo, ChainRequest};
use std::collections::HashMap;

pub struct RegistryService;

impl RegistryService {
    pub fn new() -> Self { Self }

    pub fn normalize_chain(&self, chain: &ChainRequest) -> Result<ChainInfo, EngineError> {
        let normalized = normalize_chain(chain)?;
        ChainInfo::from_request(&normalized)
    }

    pub fn derive_address(&self, seed: &[u8], chain: &ChainRequest) -> Result<String, EngineError> {
        let normalized = normalize_chain(chain)?;
        match AdapterOrchestrator::with_default_rpc()?.adapter_target(crate::adapters::orchestrator::AdapterTargetRequest::Derive { seed, chains: std::slice::from_ref(&normalized) })? { crate::adapters::orchestrator::AdapterTargetResponse::Addresses(v) => v.get(&normalized.name.trim().to_ascii_lowercase().replace([' ', '-', '/'], "_")).cloned(), _ => None }.ok_or_else(|| EngineError::Unexpected(format!("adapter_target returned no address for {}", normalized.name)))
    }

    pub fn derive_all_addresses(
        &self,
        seed: &[u8],
        chains: &[ChainRequest],
    ) -> Result<HashMap<String, String>, EngineError> {
        crate::services::address_derivation_service::AddressDerivationService::derive_addresses(seed, chains)
    }

    pub fn sign_transaction(
        &self,
        chain_type: &str,
        private_key: &[u8],
        tx_data: &[u8],
    ) -> Result<Vec<u8>, EngineError> {
        AdapterRegistry::sign_transaction(chain_type, private_key, tx_data)
    }
}
