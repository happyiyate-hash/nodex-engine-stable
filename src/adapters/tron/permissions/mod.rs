use serde::{Deserialize, Serialize};

pub mod get_permission;
pub mod implement_permission;

#[derive(Deserialize)]
pub(crate) struct TronAccountResponse {
    owner_permission: Option<TronPermission>,
    active_permission: Option<Vec<TronPermission>>,
    witness_permission: Option<TronPermission>,
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TronPermission {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub permission_name: Option<String>,
    #[serde(rename = "type", alias = "permission_type")]
    pub permission_type: Option<u32>,
    pub threshold: u32,
    pub operations: Option<String>,
    pub keys: Vec<TronPermissionKey>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TronAccountPermissions {
    pub owner: Option<TronPermission>,
    pub witness: Option<TronPermission>,
    #[serde(default, rename = "actives", alias = "active")]
    pub active: Vec<TronPermission>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TronPermissionKey {
    pub address: String,
    #[serde(default)]
    pub from: Option<String>,
    pub weight: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TronPermissionRequest {
    pub action: String,
    /// Opaque transaction message. The transaction executor never interprets this.
    #[serde(default)]
    pub message: Option<serde_json::Value>,
    #[serde(default)]
    pub address: String,
    pub key: Option<TronPermissionKey>,
    pub permission: Option<TronPermission>,
    pub update: Option<TronAccountPermissions>,
    pub transaction: Option<serde_json::Value>,
    pub to: Option<String>,
    pub amount_sun: Option<u64>,
    /// Legacy permission-management field; not part of sign_transaction message.
    pub permission_id: Option<u32>,
    #[serde(default)]
    pub outputs: Option<Vec<TronTransferOutput>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TronTransferOutput {
    pub to: String,
    pub amount_sun: u64,
}

