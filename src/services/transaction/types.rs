use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRuntime {
    pub rpc_provider: String,
    pub rpc_url: String,
    #[serde(default)]
    pub chain_name: Option<String>,
}

/// Generic transaction envelope.
/// The message is intentionally opaque to the transaction service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRequest {
    pub action: String,
    pub chain_type: String,
    /// PIN is consumed by the trusted wallet layer and never forwarded to adapters.
    #[serde(default)]
    pub pin: Option<String>,
    pub runtime: TransactionRuntime,
    pub message: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResult {
    pub chain_type: String,
    /// Adapter response returned without transaction-service interpretation.
    pub response: Value,
}

impl TransactionRequest {
    pub fn validate(&self) -> Result<(), crate::common::errors::EngineError> {
        if self.action.trim().is_empty() {
            return Err(crate::common::errors::EngineError::Validation("transaction action is required".into()));
        }
        if self.action != "sign_transaction" {
            return Err(crate::common::errors::EngineError::Validation("transaction action must be sign_transaction".into()));
        }
        if self.chain_type.trim().is_empty() {
            return Err(crate::common::errors::EngineError::Validation("chain_type is required".into()));
        }
        if self.runtime.rpc_provider.trim().is_empty() {
            return Err(crate::common::errors::EngineError::Validation("rpc_provider is required".into()));
        }
        if self.runtime.rpc_url.trim().is_empty() {
            return Err(crate::common::errors::EngineError::Validation("rpc_url is required".into()));
        }
        Ok(())
    }
}
