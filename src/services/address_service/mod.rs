use crate::adapters::orchestrator::{AdapterOrchestrator, AdapterTargetRequest, AdapterTargetResponse};
use crate::common::errors::EngineError;
use crate::types::chain::ChainRequest;

pub struct AddressService;

impl AddressService {
    pub fn public_key_to_address(chain_type: &str, public_key: &[u8]) -> Result<String, EngineError> {
        crate::adapters::registry::AdapterRegistry::public_key_to_address(chain_type, public_key)
    }

    pub fn derive_address_from_path(
        chain_type: &str,
        seed: &[u8],
        path: &str,
    ) -> Result<String, EngineError> {
        let chain = ChainRequest {
            name: chain_type.to_string(),
            chain_id: String::new(),
            symbol: None,
            network: None,
            chain_type: chain_type.to_string(),
            derivation_path: Some(path.to_string()),
            rpc_provider: None,
            rpc_url: None,
            address_type: None,
            p2pkh_version: None,
            p2sh_version: None,
            bech32_hrp: None,
        };
        let addresses = match AdapterOrchestrator::with_default_rpc()?.adapter_target(AdapterTargetRequest::Derive { seed, chains: std::slice::from_ref(&chain) })? { AdapterTargetResponse::Addresses(v) => v, _ => unreachable!() };
        let key = chain.name.trim().to_ascii_lowercase().replace([' ', '-', '/'], "_");
        addresses.get(&key)
            .cloned()
            .ok_or_else(|| EngineError::Unexpected(format!("adapter_target returned no address for {}", chain_type)))
    }

    pub fn derive_address(seed: &[u8], chain: &ChainRequest) -> Result<String, EngineError> {
        let orchestrator = AdapterOrchestrator::with_default_rpc()?;
        let addresses = match orchestrator.adapter_target(AdapterTargetRequest::Derive { seed, chains: std::slice::from_ref(chain) })? { AdapterTargetResponse::Addresses(v) => v, _ => unreachable!() };
        let key = chain.name.trim().to_ascii_lowercase().replace([' ', '-', '/'], "_");
        addresses.get(&key).cloned().ok_or_else(|| EngineError::Unexpected(format!("adapter_target returned no address for {}", chain.name)))
    }
}
