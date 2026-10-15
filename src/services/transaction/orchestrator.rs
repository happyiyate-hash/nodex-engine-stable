use crate::adapters::orchestrator::{AdapterOrchestrator, AdapterRuntimeConfig};
use crate::common::errors::EngineError;
use crate::services::transaction::types::{TransactionRequest, TransactionResult};
use crate::services::wallet::WalletOrchestrator;
use serde_json::Value;

/// Generic transaction lifecycle coordinator.
///
/// This service is deliberately blind to the transaction message. It knows
/// only how to route an action to the selected adapter and return the adapter
/// response. Construction, signing, approval, permissions, batching and
/// broadcasting belong to the adapter.
pub struct TransactionOrchestrator {
    adapters: AdapterOrchestrator,
}

impl TransactionOrchestrator {
    pub fn new(adapters: AdapterOrchestrator) -> Self {
        Self { adapters }
    }

    pub fn with_default_rpc() -> Result<Self, EngineError> {
        Ok(Self::new(AdapterOrchestrator::with_default_rpc()?))
    }

    pub async fn execute(&self, request: &TransactionRequest) -> Result<TransactionResult, EngineError> {
        request.validate()?;
        let response = self.call_adapter(request).await?;
        Ok(TransactionResult {
            chain_type: request.chain_type.clone(),
            response,
        })
    }

    pub async fn sign(&self, request: &TransactionRequest) -> Result<Value, EngineError> {
        request.validate()?;
        self.call_adapter(request).await
    }

    async fn call_adapter(&self, request: &TransactionRequest) -> Result<Value, EngineError> {
        let pin = request.pin.as_deref().filter(|value| !value.trim().is_empty())
            .ok_or_else(|| EngineError::Validation("transaction signing requires wallet PIN".into()))?;
        let wallet_id = request.message.get("wallet_id").and_then(Value::as_str)
            .map(str::to_owned)
            .or(WalletOrchestrator::active_child_id()?);
        let wallet_id = wallet_id.ok_or_else(|| EngineError::Validation("no active wallet child is available for signing".into()))?;
        let signing_key = WalletOrchestrator::signing_key(&wallet_id, &request.chain_type, pin)?;
        let mut message = request.message.clone();
        let object = message.as_object_mut()
            .ok_or_else(|| EngineError::Validation("sign_transaction message must be a JSON object".into()))?;
        object.insert("_wallet_child_id".into(), Value::String(wallet_id));
        object.insert("_signing_key_hex".into(), Value::String(hex::encode(signing_key)));

        let body = serde_json::json!({
            "action": request.action,
            "message": message,
        });

        let bytes = serde_json::to_vec(&body).map_err(|e| {
            EngineError::Unexpected(format!("failed to encode adapter request: {e}"))
        })?;

        let response = self
            .adapters
            .call(
                &request.chain_type,
                &bytes,
                AdapterRuntimeConfig {
                    rpc_provider: request.runtime.rpc_provider.clone(),
                    rpc_url: request.runtime.rpc_url.clone(),
                    chain_name: request.runtime.chain_name.clone(),
                },
            )
            .await?;

        serde_json::from_slice(&response)
            .map_err(|e| EngineError::Unexpected(format!("invalid adapter response: {e}")))
    }
}
