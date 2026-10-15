use crate::common::errors::EngineError;
use super::{WalletRecord, WalletRegistry};

/// Wallet-domain orchestrator.
///
/// This is the application-facing coordinator for wallet lifecycle operations.
/// The registry remains the single source of truth; this layer owns the flow
/// around switching, hierarchy lookup, and signing-key resolution.
pub struct WalletOrchestrator;

impl WalletOrchestrator {
    pub fn switch_wallet(wallet_group_id: &str, pin: &str) -> Result<WalletRecord, EngineError> {
        WalletRegistry::switch(wallet_group_id, pin)?;
        WalletRegistry::active()?.ok_or_else(|| EngineError::Unexpected("wallet switched but no active child exists".into()))
    }

    pub fn switch_child(child_id: &str) -> Result<WalletRecord, EngineError> {
        WalletRegistry::switch_child(child_id)
    }

    pub fn active_group_id() -> Result<Option<String>, EngineError> {
        WalletRegistry::active_group_id()
    }

    pub fn active_child_id() -> Result<Option<String>, EngineError> {
        WalletRegistry::active_child_id()
    }

    pub fn active_wallet() -> Result<Option<WalletRecord>, EngineError> {
        WalletRegistry::active()
    }

    pub fn get_wallet(wallet_id: &str) -> Result<WalletRecord, EngineError> {
        WalletRegistry::get(wallet_id)
    }

    pub fn children(wallet_id: &str) -> Result<Vec<WalletRecord>, EngineError> {
        WalletRegistry::children(wallet_id)
    }

    pub fn signing_key(wallet_id: &str, blockchain: &str, pin: &str) -> Result<Vec<u8>, EngineError> {
        WalletRegistry::resolve_signing_key(wallet_id, blockchain, pin)
    }
}
