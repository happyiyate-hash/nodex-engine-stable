use crate::adapters::tron::TronAdapter;
use crate::common::errors::EngineError;
use crate::services::rpc::RpcOrchestrator;
use super::{TronAccountPermissions, TronAccountResponse};

impl TronAdapter {
    pub async fn inspect_account_permissions_via_rpc(
        &self,
        rpc: &RpcOrchestrator,
        address: &str,
    ) -> Result<TronAccountPermissions, EngineError> {
        self.require_rpc_config()?;

        let result = self.execute_tron_rpc(
            rpc,
            "/wallet/getaccount",
            serde_json::json!({ "address": address, "visible": true }),
        ).await?;

        let account: TronAccountResponse = serde_json::from_value(result)
            .map_err(|e| EngineError::Unexpected(format!("invalid TRON account response: {e}")))?;

        if let Some(error) = account.error {
            return Err(EngineError::Unexpected(error));
        }

        Ok(TronAccountPermissions {
            owner: account.owner_permission,
            witness: account.witness_permission,
            active: account.active_permission.unwrap_or_default(),
        })
    }

    pub async fn inspect_account_permissions(&self, address: &str) -> Result<TronAccountPermissions, EngineError> {
        let client = self.client.as_ref()
            .ok_or_else(|| EngineError::Unexpected("TRON RPC client unavailable".into()))?;
        let url = format!("{}/wallet/getaccount", self.rpc_url.trim_end_matches('/'));
        let response = client.post(&url)
            .json(&serde_json::json!({ "address": address, "visible": true }))
            .send()
            .await
            .map_err(|e| EngineError::Unexpected(e.to_string()))?;
        if !response.status().is_success() {
            return Err(EngineError::Unexpected(format!("TRON RPC returned HTTP {}", response.status())));
        }
        let account: TronAccountResponse = response.json().await
            .map_err(|e| EngineError::Unexpected(e.to_string()))?;
        if let Some(error) = account.error {
            return Err(EngineError::Unexpected(error));
        }

        Ok(TronAccountPermissions {
            owner: account.owner_permission,
            witness: account.witness_permission,
            active: account.active_permission.unwrap_or_default(),
        })
    }

}
