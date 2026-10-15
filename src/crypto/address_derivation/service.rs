use crate::adapters::orchestrator::AdapterOrchestrator;
use crate::common::errors::EngineError;
use crate::types::chain::ChainRequest;
use std::collections::HashMap;

pub struct AddressDerivationService;

impl AddressDerivationService {
    pub fn derive_addresses(seed: &[u8], chains: &[ChainRequest]) -> Result<HashMap<String, String>, EngineError> {
        let orchestrator = AdapterOrchestrator::with_default_rpc()?;
        let mut addresses = HashMap::with_capacity(chains.len());

        for chain in chains {
            let path = chain.path()?;
            let params = serde_json::json!({
                "chain_type": chain.chain_type,
                "path": path
            });
            let result = orchestrator.derive(seed, "derive_address", &params)?;
            let address = result
                .get("address")
                .and_then(|value| value.as_str())
                .ok_or_else(|| EngineError::Unexpected(format!(
                    "adapter returned no address for {}",
                    chain.name
                )))?;
            let key = chain.name.trim().to_ascii_lowercase().replace([' ', '-', '/'], "_");
            if addresses.insert(key.clone(), address.to_string()).is_some() {
                return Err(EngineError::Validation(format!(
                    "duplicate blockchain target: {}",
                    chain.name
                )));
            }
        }

        Ok(addresses)
    }

    pub fn derive_all_addresses(seed: &[u8], chains: &[ChainRequest]) -> Result<HashMap<String, String>, EngineError> {
        Self::derive_addresses(seed, chains)
    }

    pub fn derive_one(seed: &[u8], chain: &ChainRequest) -> Result<String, EngineError> {
        let path = chain.path()?;
        let params = serde_json::json!({
            "chain_type": chain.chain_type,
            "path": path
        });
        let result = AdapterOrchestrator::with_default_rpc()?.derive(seed, "derive_address", &params)?;
        result
            .get("address")
            .and_then(|value| value.as_str())
            .map(str::to_owned)
            .ok_or_else(|| EngineError::Unexpected(format!(
                "adapter returned no address for {}",
                chain.name
            )))
    }
}
