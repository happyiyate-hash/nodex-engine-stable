use std::collections::HashMap;
use std::sync::Arc;

use crate::common::errors::EngineError;
use crate::services::rpc::providers::{AlchemyProvider, BlockfrostProvider, InfuraProvider, PublicProvider, RpcProvider, TronProvider, XrpProvider};

#[derive(Default)]
pub struct RpcProviderRegistry {
    providers: HashMap<String, Arc<dyn RpcProvider>>,
}

impl RpcProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates the built-in provider set used by the RPC orchestrator.
    pub fn with_defaults() -> Result<Self, EngineError> {
        let mut registry = Self::new();
        registry.register(AlchemyProvider::new())?;
        registry.register(BlockfrostProvider::new())?;
        registry.register(InfuraProvider::new())?;
        registry.register(PublicProvider::new())?;
        registry.register(TronProvider::new())?;
        registry.register(XrpProvider::new())?;
        Ok(registry)
    }

    pub fn register<P>(&mut self, provider: P) -> Result<(), EngineError>
    where
        P: RpcProvider + 'static,
    {
        let name = provider.provider_name().trim().to_ascii_lowercase();

        if name.is_empty() {
            return Err(EngineError::Validation(
                "RPC provider name cannot be empty".into(),
            ));
        }

        if self.providers.contains_key(&name) {
            return Err(EngineError::Validation(format!(
                "RPC provider already registered: {}",
                name
            )));
        }

        self.providers.insert(name, Arc::new(provider));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn RpcProvider>> {
        self.providers
            .get(&name.trim().to_ascii_lowercase())
            .cloned()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub fn provider_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.providers.keys().cloned().collect();
        names.sort();
        names
    }
}
