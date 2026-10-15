use crate::adapters::tron::TronAdapter;
use crate::common::errors::EngineError;
use crate::services::rpc::RpcOrchestrator;
use sha3::{Digest, Keccak256};
use super::{TronAccountPermissions, TronPermission, TronPermissionKey, TronPermissionRequest};

impl TronAdapter {
    pub fn permission_allows_contract(permission: &TronPermission, contract_type: u16) -> bool {
        if permission.permission_type == Some(1) {
            return false;
        }
        let Some(bitmap) = permission.operations.as_deref() else {
            return true;
        };
        let bytes = match hex::decode(bitmap) {
            Ok(bytes) if bytes.len() == 32 => bytes,
            _ => return false,
        };
        let index = contract_type as usize;
        let byte = index / 8;
        let bit = index % 8;
        byte < bytes.len() && (bytes[byte] & (1u8 << bit)) != 0
    }

    pub fn signer_weight(permission: &TronPermission, signer_address: &str) -> u32 {
        permission.keys.iter()
            .filter(|key| key.address.eq_ignore_ascii_case(signer_address))
            .map(|key| key.weight)
            .sum()
    }

    pub fn validate_permission_for_signing(
        permissions: &TronAccountPermissions,
        permission_id: u32,
        contract_type: u16,
        signer_address: &str,
    ) -> Result<(), EngineError> {
        let permission = match permission_id {
            0 => permissions.owner.as_ref()
                .ok_or_else(|| EngineError::Validation("TRON owner permission is missing".into()))?,
            1 => return Err(EngineError::Validation("TRON witness permission cannot authorize normal transactions".into())),
            2..=9 => permissions.active.iter()
                .find(|p| p.id == Some(permission_id))
                .ok_or_else(|| EngineError::Validation(format!("TRON active permission {permission_id} not found")))?,
            _ => return Err(EngineError::Validation("invalid TRON permission_id".into())),
        };

        if !Self::permission_allows_contract(permission, contract_type) {
            return Err(EngineError::Validation(format!(
                "TRON permission {permission_id} does not allow contract type {contract_type}"
            )));
        }

        if Self::signer_weight(permission, signer_address) == 0 {
            return Err(EngineError::Validation(format!(
                "signer {signer_address} is not authorized by TRON permission {permission_id}"
            )));
        }

        Ok(())
    }

    pub fn has_reached_threshold(permission: &TronPermission, signer_addresses: &[String]) -> bool {
        use std::collections::HashSet;

        let mut seen = HashSet::new();
        let weight: u64 = signer_addresses.iter()
            .filter(|address| seen.insert(address.to_ascii_lowercase()))
            .map(|address| Self::signer_weight(permission, address) as u64)
            .sum();
        weight >= permission.threshold as u64
    }

    pub async fn permission_json(
        &self,
        rpc: &RpcOrchestrator,
        request: &[u8],
    ) -> Result<Vec<u8>, EngineError> {
        let request: TronPermissionRequest = serde_json::from_slice(request)
            .map_err(|e| EngineError::Validation(format!("invalid TRON permission JSON: {e}")))?;

        let response = match request.action.as_str() {
            "send" => {
                let from = if request.address.is_empty() {
                    return Err(EngineError::Validation("send requires a wallet/account address".into()));
                } else {
                    request.address.clone()
                };

                let outputs = request.outputs.clone().unwrap_or_else(|| {
                    request.to.clone().zip(request.amount_sun).map(|(to, amount_sun)| {
                        vec![super::TronTransferOutput { to, amount_sun }]
                    }).unwrap_or_default()
                });

                if outputs.is_empty() {
                    return Err(EngineError::Validation("send requires to/amount_sun or outputs".into()));
                }
                if outputs.len() > 1 {
                    return Err(EngineError::Unsupported(
                        "TRON native TRX does not support multiple TransferContract outputs in one transaction; use a configured smart-contract batch transfer instead".into()
                    ));
                }

                let output = &outputs[0];
                let permission_id = request.permission_id.ok_or_else(|| {
                    EngineError::Validation("permission_id is required for send".into())
                })?;

                Self::validate_tron_base58_address(&from)?;
                Self::validate_tron_base58_address(&output.to)?;

                if output.amount_sun == 0 {
                    return Err(EngineError::Validation("TRX amount_sun must be greater than zero".into()));
                }
                if permission_id > 9 || permission_id == 1 {
                    return Err(EngineError::Validation("invalid TRON permission_id for TRX transfer".into()));
                }

                let tx = self.build_trx_transaction(
                    rpc,
                    &from,
                    &output.to,
                    output.amount_sun,
                    permission_id,
                ).await?;

                serde_json::json!({
                    "ok": true,
                    "action": "send",
                    "from": from,
                    "outputs": outputs,
                    "permission_id": permission_id,
                    "unsigned_transaction": tx
                })
            }
            "get_permissions" => {
                let permissions = self.inspect_account_permissions_via_rpc(rpc, &request.address).await?;
                serde_json::json!({
                    "ok": true,
                    "action": "get_permissions",
                    "address": request.address,
                    "permissions": permissions
                })
            }
            "validate_permission_update" => {
                let update = request.update.ok_or_else(|| EngineError::Validation(
                    "update is required for validate_permission_update".into()
                ))?;
                self.validate_permission_update(&request.address, &update).await?;
                serde_json::json!({
                    "ok": true,
                    "action": "validate_permission_update",
                    "address": request.address,
                    "valid": true,
                    "update": update
                })
            }
            "add_owner_key" => {
                let key = request.key.ok_or_else(|| EngineError::Validation("key is required".into()))?;
                Self::validate_tron_base58_address(&key.address)?;
                let mut permissions = self.inspect_account_permissions_via_rpc(rpc, &request.address).await?;
                let owner = permissions.owner.as_mut().ok_or_else(|| EngineError::Validation("owner permission is missing".into()))?;
                Self::add_permission_key(owner, key)?;
                self.validate_account_permissions(&request.address, &permissions)?;
                serde_json::json!({
                    "ok": true,
                    "action": "add_owner_key",
                    "address": request.address,
                    "update": permissions
                })
            }
            "add_active_permission" => {
                let permission = request.permission.ok_or_else(|| EngineError::Validation("permission is required".into()))?;
                let mut permissions = self.inspect_account_permissions_via_rpc(rpc, &request.address).await?;
                if permissions.active.len() >= 8 {
                    return Err(EngineError::Validation("TRON allows at most 8 active permissions".into()));
                }
                let mut permission = permission;
                permission.permission_type = Some(2);
                Self::validate_tron_base58_address(&request.address)?;
                let used_ids: std::collections::HashSet<u32> = permissions.active.iter().filter_map(|p| p.id).collect();
                let next_id = (2..=9).find(|id| !used_ids.contains(id)).ok_or_else(|| EngineError::Validation("no available TRON active permission ID (2-9)".into()))?;
                permission.id = Some(next_id);
                permissions.active.push(permission);
                self.validate_account_permissions(&request.address, &permissions)?;
                serde_json::json!({
                    "ok": true,
                    "action": "add_active_permission",
                    "address": request.address,
                    "update": permissions
                })
            }
            "set_witness_permission" => {
                let permission = request.permission.ok_or_else(|| EngineError::Validation("permission is required".into()))?;
                let mut permissions = self.inspect_account_permissions_via_rpc(rpc, &request.address).await?;
                let mut permission = permission;
                permission.permission_type = Some(1);
                permission.id = Some(1);
                if permission.keys.len() != 1 {
                    return Err(EngineError::Validation("TRON witness permission must contain exactly one key".into()));
                }
                Self::validate_tron_base58_address(&permission.keys[0].address)?;
                permissions.witness = Some(permission);
                self.validate_account_permissions(&request.address, &permissions)?;
                serde_json::json!({
                    "ok": true,
                    "action": "set_witness_permission",
                    "address": request.address,
                    "update": permissions
                })
            }
            "prepare_permission_update" => {
                let update = request.update.ok_or_else(|| EngineError::Validation("update is required".into()))?;
                self.validate_permission_update(&request.address, &update).await?;
                let permission_id = request.permission_id.unwrap_or(0);
                if permission_id > 9 || permission_id == 1 {
                    return Err(EngineError::Validation("invalid TRON permission_id for permission update".into()));
                }
                let tx = self.build_permission_update_transaction(rpc, &request.address, &update, permission_id).await?;
                serde_json::json!({
                    "ok": true,
                    "action": "prepare_permission_update",
                    "address": request.address,
                    "unsigned_transaction": tx
                })
            }
            "send_trx" => {
                let to = request.to.clone().ok_or_else(|| EngineError::Validation("to is required for send_trx".into()))?;
                let amount_sun = request.amount_sun.ok_or_else(|| EngineError::Validation("amount_sun is required for send_trx".into()))?;
                let permission_id = request.permission_id.ok_or_else(|| EngineError::Validation(
                    "permission_id is required for send_trx".into()
                ))?;
                Self::validate_tron_base58_address(&request.address)?;
                Self::validate_tron_base58_address(&to)?;
                if amount_sun == 0 {
                    return Err(EngineError::Validation("TRX amount_sun must be greater than zero".into()));
                }
                if permission_id > 9 || permission_id == 1 {
                    return Err(EngineError::Validation("invalid TRON permission_id for TRX transfer".into()));
                }
                let tx = self.build_trx_transaction(rpc, &request.address, &to, amount_sun, permission_id).await?;
                serde_json::json!({
                    "ok": true,
                    "action": "send_trx",
                    "from": request.address,
                    "to": to,
                    "amount_sun": amount_sun,
                    "permission_id": permission_id,
                    "unsigned_transaction": tx
                })
            }
            "sign_transaction" => {
                let message = request.message.ok_or_else(|| EngineError::Validation(
                    "message is required for sign_transaction".into()
                ))?;

                // The complete transaction intent is opaque to this service.
                // The TRON adapter alone interprets its message and owns
                // construction, signing, approval/permission handling and
                // broadcasting.
                crate::adapters::tron::signing::execute_sign_transaction(
                    self,
                    rpc,
                    message,
                ).await?
            }
            "get_sign_weight" => {
                let transaction = request.transaction.ok_or_else(|| EngineError::Validation(
                    "transaction is required for get_sign_weight".into()
                ))?;
                let result = crate::adapters::tron::signing::get_sign_weight(self, rpc, transaction).await?;
                serde_json::json!({
                    "ok": true,
                    "action": "get_sign_weight",
                    "result": result
                })
            }
            "get_approved_list" => {
                let transaction = request.transaction.ok_or_else(|| EngineError::Validation(
                    "transaction is required for get_approved_list".into()
                ))?;
                let result = crate::adapters::tron::signing::get_approved_list(self, rpc, transaction).await?;
                serde_json::json!({
                    "ok": true,
                    "action": "get_approved_list",
                    "result": result
                })
            }
            "broadcast_signed_transaction" => {
                let transaction = request.transaction.ok_or_else(|| EngineError::Validation(
                    "transaction is required for broadcast_signed_transaction".into()
                ))?;
                let result = self.broadcast_signed_transaction(rpc, transaction).await?;
                serde_json::json!({
                    "ok": true,
                    "action": "broadcast_signed_transaction",
                    "result": result
                })
            }
            _ => return Err(EngineError::Validation(format!(
                "unsupported TRON permission action: {}",
                request.action
            ))),
        };

        serde_json::to_vec(&response)
            .map_err(|e| EngineError::Unexpected(format!("failed to encode TRON permission response: {e}")))
    }

    async fn validate_permission_update(
        &self,
        address: &str,
        update: &TronAccountPermissions,
    ) -> Result<(), EngineError> {
        Self::validate_tron_base58_address(address)?;
        self.validate_account_permissions(address, update)?;
        if update.owner.is_none() {
            return Err(EngineError::Validation("TRON owner permission is required".into()));
        }
        if update.active.is_empty() {
            return Err(EngineError::Validation("TRON requires at least one active permission".into()));
        }
        Ok(())
    }

    fn validate_account_permissions(
        &self,
        account_address: &str,
        permissions: &TronAccountPermissions,
    ) -> Result<(), EngineError> {
        Self::validate_tron_base58_address(account_address)?;
        let mut addresses = std::collections::HashSet::new();

        let owner = permissions.owner.as_ref().ok_or_else(|| EngineError::Validation("owner permission is required".into()))?;
        Self::validate_permission_object(owner, 0, false, account_address, &mut addresses)?;

        if let Some(witness) = permissions.witness.as_ref() {
            Self::validate_permission_object(witness, 1, true, account_address, &mut addresses)?;
        }

        let mut active_ids = std::collections::HashSet::new();
        for permission in &permissions.active {
            if let Some(id) = permission.id {
                if !(2..=9).contains(&id) {
                    return Err(EngineError::Validation("TRON active permission id must be 2 through 9".into()));
                }
                if !active_ids.insert(id) {
                    return Err(EngineError::Validation(format!("duplicate TRON active permission id {id}")));
                }
            }
            Self::validate_permission_object(permission, 2, false, account_address, &mut addresses)?;
        }

        Ok(())
    }

    async fn build_permission_update_transaction(
        &self,
        rpc: &RpcOrchestrator,
        address: &str,
        permissions: &TronAccountPermissions,
        permission_id: u32,
    ) -> Result<serde_json::Value, EngineError> {
        self.require_rpc_config()?;
        self.execute_tron_rpc(
            rpc,
            "/wallet/accountpermissionupdate",
            serde_json::json!({
                "owner_address": address,
                "owner": permissions.owner,
                "witness": permissions.witness,
                "actives": permissions.active,
                "visible": true,
                "Permission_id": permission_id
            }),
        ).await
    }

fn add_permission_key(permission: &mut TronPermission, key: TronPermissionKey) -> Result<(), EngineError> {
    if permission.keys.len() >= 5 {
        return Err(EngineError::Validation("TRON permissions allow at most 5 keys".into()));
    }
    if key.weight == 0 {
        return Err(EngineError::Validation("TRON signer weight must be greater than zero".into()));
    }
    if permission.keys.iter().any(|existing| existing.address.eq_ignore_ascii_case(&key.address)) {
        return Err(EngineError::Validation("duplicate TRON permission signer".into()));
    }
    permission.keys.push(key);
    Ok(())
}

fn validate_permission_object(
    permission: &TronPermission,
    expected_type: u32,
    witness: bool,
    account_address: &str,
    global_addresses: &mut std::collections::HashSet<String>,
) -> Result<(), EngineError> {
    if permission.permission_type.unwrap_or(expected_type) != expected_type {
        return Err(EngineError::Validation("TRON permission type is invalid".into()));
    }
    if permission.threshold == 0 {
        return Err(EngineError::Validation("TRON permission threshold must be greater than zero".into()));
    }
    if permission.keys.is_empty() || permission.keys.len() > 5 {
        return Err(EngineError::Validation("TRON permission must contain 1 to 5 keys".into()));
    }
    if witness && permission.keys.len() != 1 {
        return Err(EngineError::Validation("TRON witness permission must contain exactly one key".into()));
    }
    if permission.permission_name.as_deref().unwrap_or("").as_bytes().len() > 32 {
        return Err(EngineError::Validation("TRON permission_name must be at most 32 bytes".into()));
    }
    let mut total: u64 = 0;
    let mut local = std::collections::HashSet::new();

    for key in &permission.keys {
        if key.weight == 0 {
            return Err(EngineError::Validation("TRON signer weight must be greater than zero".into()));
        }
        let normalized = key.address.to_ascii_lowercase();
        if !local.insert(normalized.clone()) {
            return Err(EngineError::Validation("duplicate signer address in TRON permission".into()));
        }
        global_addresses.insert(normalized);
        Self::validate_tron_base58_address(&key.address)?;
        total = total.checked_add(key.weight as u64)
            .ok_or_else(|| EngineError::Validation("TRON signer weight overflow".into()))?;
    }

    if total < permission.threshold as u64 {
        return Err(EngineError::Validation("TRON permission threshold is unreachable".into()));
    }

    if expected_type == 0 && permission.id.unwrap_or(0) != 0 {
        return Err(EngineError::Validation("TRON owner permission must use id 0".into()));
    }
    if expected_type == 1 && permission.id.unwrap_or(1) != 1 {
        return Err(EngineError::Validation("TRON witness permission must use id 1".into()));
    }
    if expected_type == 2 && permission.id.is_some() && !(2..=9).contains(&permission.id.unwrap()) {
        return Err(EngineError::Validation("TRON active permission id must be 2 through 9".into()));
    }

    if expected_type == 0 && !permission.keys.iter().any(|k| k.address.eq_ignore_ascii_case(account_address)) {
        // Owner may intentionally be migrated to a completely new key set, so this
        // is informational rather than a hard failure.
    }

    if expected_type == 2 {
        let operations = permission.operations.as_deref()
            .ok_or_else(|| EngineError::Validation("TRON active permission requires a 32-byte operations bitmap".into()))?;
        let bytes = hex::decode(operations)
            .map_err(|_| EngineError::Validation("TRON operations must be hexadecimal".into()))?;
        if bytes.len() != 32 {
            return Err(EngineError::Validation("TRON operations bitmap must be exactly 32 bytes".into()));
        }
    }

    Ok(())
}

fn validate_tron_base58_address(address: &str) -> Result<(), EngineError> {
    let decoded = bs58::decode(address)
        .into_vec()
        .map_err(|_| EngineError::Validation("invalid TRON Base58 address".into()))?;
    if decoded.len() != 25 || decoded[0] != 0x41 {
        return Err(EngineError::Validation("invalid TRON Base58 address".into()));
    }
    let payload = &decoded[..21];
    let checksum = Keccak256::digest(payload);
    let checksum2 = Keccak256::digest(&checksum);
    if decoded[21..25] != checksum2[..4] {
        return Err(EngineError::Validation("invalid TRON Base58Check checksum".into()));
    }
    Ok(())
}

}
