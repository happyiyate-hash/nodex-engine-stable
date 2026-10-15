pub struct WalletState {
    pub mnemonic_backed_up: bool,
}

pub struct WalletService {
    mnemonic: String,
}

impl WalletService {
    pub fn new(mnemonic: &str) -> Result<Self, crate::common::errors::EngineError> {
        Ok(Self { mnemonic: mnemonic.to_string() })
    }

    pub fn backup_wallet(&self) -> Result<String, crate::common::errors::EngineError> {
        Ok(format!("backup:{}", self.mnemonic))
    }

    pub fn restore_wallet(&mut self, mnemonic: &str) -> Result<(), crate::common::errors::EngineError> {
        self.mnemonic = mnemonic.to_string();
        Ok(())
    }

    pub fn state(&self) -> WalletState {
        WalletState { mnemonic_backed_up: !self.mnemonic.is_empty() }
    }
}


pub mod registry;
pub mod orchestrator;
pub use registry::{WalletChoice, WalletGroup, WalletRecord, WalletRegistry, WalletType, WalletBlockchain, WalletToken, WalletRegistrySnapshot};
pub use orchestrator::WalletOrchestrator;
