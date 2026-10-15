use crate::adapters::registry::AdapterRegistry;
use crate::common::errors::EngineError;
use crate::services::transaction::orchestrator::TransactionOrchestrator;
use crate::services::transaction::types::{TransactionRequest, TransactionResult};

/// Public transaction execution service.
///
/// It owns orchestration only. Adapter-specific construction, signing,
/// approval, permissions and broadcasting are not implemented here.
pub struct TransactionService;

impl TransactionService {
    pub fn orchestrator() -> Result<TransactionOrchestrator, EngineError> {
        TransactionOrchestrator::with_default_rpc()
    }

    pub async fn sign(request: &TransactionRequest) -> Result<serde_json::Value, EngineError> {
        Self::orchestrator()?.sign(request).await
    }

    pub async fn execute(request: &TransactionRequest) -> Result<TransactionResult, EngineError> {
        Self::orchestrator()?.execute(request).await
    }

    pub fn sign_transaction(
        chain_type: &str,
        private_key: &[u8],
        tx_data: &[u8],
    ) -> Result<Vec<u8>, EngineError> {
        AdapterRegistry::sign_transaction(chain_type, private_key, tx_data)
    }
}
