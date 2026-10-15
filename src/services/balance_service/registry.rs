use std::collections::HashMap;
use std::sync::Arc;
use super::providers::{AlchemyBalanceProvider, BalanceProvider, BlockfrostBalanceProvider, InfuraBalanceProvider, PublicBalanceProvider};

#[derive(Default, Clone)]
pub struct BalanceProviderRegistry {
    providers: HashMap<String, Arc<dyn BalanceProvider>>,
}

impl BalanceProviderRegistry {
    pub fn new() -> Self { Self::default() }

    /// Creates the SDK defaults. Only providers with implemented adapters are registered.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register("alchemy", AlchemyBalanceProvider);
        registry.register("blockfrost", BlockfrostBalanceProvider);
        registry.register("infura", InfuraBalanceProvider);
        registry.register("public", PublicBalanceProvider);
        registry
    }

    pub fn register<P>(&mut self, name: impl Into<String>, provider: P)
    where P: BalanceProvider + 'static {
        self.providers.insert(name.into().trim().to_ascii_lowercase(), Arc::new(provider));
    }

    pub fn register_arc(&mut self, name: impl Into<String>, provider: Arc<dyn BalanceProvider>) {
        self.providers.insert(name.into().trim().to_ascii_lowercase(), provider);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn BalanceProvider>> {
        self.providers.get(&name.trim().to_ascii_lowercase()).cloned()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.providers.contains_key(&name.trim().to_ascii_lowercase())
    }

    pub fn names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}
