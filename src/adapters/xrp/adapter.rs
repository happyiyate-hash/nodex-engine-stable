use sha2::Digest;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use crate::adapters::adapter_trait::WalletAdapter;
use crate::common::errors::EngineError;
use crate::crypto::address::hash160;
use crate::crypto::bip32::{derive_master_key, derive_child_key};
use crate::crypto::derivation::DerivationPath;
use crate::crypto::signing::sign_secp256k1_payload;
use crate::services::rpc::providers::RpcRequest;
use crate::services::rpc::service::RpcOrchestrator;

/// Minimal chain configuration for the XRP adapter.
#[derive(Clone, Debug)]
pub struct ChainConfig {
    pub rpc_url: String,
    pub name: Option<String>,
}

/// Asset representation used by the adapter.
#[derive(Clone, Debug)]
pub struct AssetRow {
    pub address: String,
    pub symbol: String,
    pub decimals: Option<u32>,
    pub is_native: bool,
    pub balance: String,
}


/// Complete on-ledger XRP authorization/security state returned by scan_security.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct XrpSecurityScan {
    pub address: String,
    pub account_exists: bool,
    pub account_flags: XrpAccountFlags,
    pub regular_key: Option<String>,
    pub signer_list: Option<XrpSignerList>,
    pub delegates: Vec<XrpDelegate>,
    pub authorization: XrpAuthorizationSummary,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct XrpAccountFlags {
    pub raw: u32,
    pub default_ripple: bool,
    pub deposit_auth: bool,
    pub disable_master_key: bool,
    pub disallow_incoming_check: bool,
    pub disallow_incoming_nft_offer: bool,
    pub disallow_incoming_pay_channel: bool,
    pub disallow_incoming_trustline: bool,
    pub disallow_xrp: bool,
    pub global_freeze: bool,
    pub no_freeze: bool,
    pub password_spent: bool,
    pub require_authorization: bool,
    pub require_destination_tag: bool,
    pub allow_trustline_clawback: bool,
    pub allow_trustline_locking: bool,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct XrpSigner {
    pub account: String,
    pub weight: u32,
    pub wallet_locator: Option<String>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct XrpSignerList {
    pub quorum: u32,
    pub signers: Vec<XrpSigner>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct XrpPermission {
    pub value: serde_json::Value,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct XrpDelegate {
    pub authorize: String,
    pub permissions: Vec<XrpPermission>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct XrpAuthorizationSummary {
    pub master_key_enabled: bool,
    pub regular_key_enabled: bool,
    pub multi_signing_enabled: bool,
    pub permission_delegation_enabled: bool,
}



impl XrpAccountFlags {
    fn from_raw(raw: u32) -> Self {
        Self {
            raw,
            default_ripple: raw & 0x00800000 != 0,
            deposit_auth: raw & 0x01000000 != 0,
            disable_master_key: raw & 0x00100000 != 0,
            disallow_incoming_check: raw & 0x08000000 != 0,
            disallow_incoming_nft_offer: raw & 0x04000000 != 0,
            disallow_incoming_pay_channel: raw & 0x02000000 != 0,
            disallow_incoming_trustline: raw & 0x10000000 != 0,
            disallow_xrp: raw & 0x00080000 != 0,
            global_freeze: raw & 0x00400000 != 0,
            no_freeze: raw & 0x00200000 != 0,
            password_spent: raw & 0x00010000 != 0,
            require_authorization: raw & 0x00040000 != 0,
            require_destination_tag: raw & 0x00020000 != 0,
            allow_trustline_clawback: raw & 0x80000000 != 0,
            allow_trustline_locking: raw & 0x40000000 != 0,
        }
    }
}

fn parse_signer_list(result: &serde_json::Value) -> Option<XrpSignerList> {
    let list = result.get("signer_lists")?.as_array()?.first()?;
    let quorum = list.get("SignerQuorum").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let signers = list.get("SignerEntries")?.as_array()?.iter().filter_map(|entry| {
        let signer = entry.get("SignerEntry")?;
        Some(XrpSigner {
            account: signer.get("Account")?.as_str()?.to_owned(),
            weight: signer.get("SignerWeight").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            wallet_locator: signer.get("WalletLocator").and_then(|v| v.as_str()).map(str::to_owned),
        })
    }).collect();
    Some(XrpSignerList { quorum, signers })
}


pub struct XrpAdapter;

impl XrpAdapter {
    pub const DEFAULT_DERIVATION_PATH: &'static str = "m/44'/144'/0'/0/0";

    pub fn normalize_balances(assets: Vec<AssetRow>) -> Result<Vec<AssetRow>, EngineError> {
        assets.into_iter().map(|mut asset| {
            if asset.is_native || asset.symbol.eq_ignore_ascii_case("XRP") {
                asset.decimals = Some(6);
                asset.balance = normalize_drops(&asset.balance)?;
            }
            Ok(asset)
        }).collect()
    }

    pub fn new(_chain: ChainConfig) -> Self { Self }

    fn base58check_account(payload: &[u8]) -> String {
        let mut data = Vec::with_capacity(25);
        data.push(0u8);
        data.extend_from_slice(payload);
        let first = sha2::Sha256::digest(&data);
        let second = sha2::Sha256::digest(first);
        data.extend_from_slice(&second[..4]);
        bs58::encode(data).with_alphabet(bs58::Alphabet::RIPPLE).into_string()
    }

    fn address_from_public(public_key: &[u8]) -> Result<String, EngineError> {
        if public_key.len() != 33 { return Err(EngineError::Validation("compressed XRP public key expected".into())); }
        Ok(Self::base58check_account(&hash160(public_key)))
    }

    /// Scan all authorization/security state that can be discovered from the XRP ledger.
    ///
    /// The RPC URL is supplied by the saved XRP ChainConfig. This deliberately does
    /// not invent an endpoint or use a global provider. Later transaction-management
    /// methods can reuse the same target/provider boundary.
    pub async fn scan_security(&self, account: &str, chain: &ChainConfig) -> Result<XrpSecurityScan, EngineError> {
        if chain.rpc_url.trim().is_empty() {
            return Err(EngineError::Validation("XRP RPC URL is required for security scanning".into()));
        }
        if !account.starts_with('r') {
            return Err(EngineError::Validation("invalid XRP classic account address".into()));
        }

        let client = reqwest::Client::new();
        let account_info = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "account_info",
            "params": [{
                "account": account,
                "ledger_index": "validated",
                "signer_lists": true
            }]
        });

        let response = client.post(&chain.rpc_url).json(&account_info).send().await
            .map_err(|e| EngineError::Network(format!("XRP account_info RPC failed: {e}")))?;
        let body: serde_json::Value = response.json().await
            .map_err(|e| EngineError::Network(format!("invalid XRP RPC response: {e}")))?;

        if body.get("result").and_then(|r| r.get("error")).is_some() {
            let message = body["result"]["error_message"].as_str()
                .or_else(|| body["result"]["error"].as_str())
                .unwrap_or("XRP account_info failed");
            let not_found = body["result"]["error"].as_str() == Some("actNotFound");
            if not_found {
                return Ok(XrpSecurityScan {
                    address: account.to_string(),
                    account_exists: false,
                    ..Default::default()
                });
            }
            return Err(EngineError::Network(message.to_string()));
        }

        let result = body.get("result").ok_or_else(|| EngineError::Network("XRP account_info returned no result".into()))?;
        let account_data = result.get("account_data").cloned().unwrap_or_default();
        let flags = account_data.get("Flags").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let account_flags = XrpAccountFlags::from_raw(flags);

        let regular_key = account_data.get("RegularKey").and_then(|v| v.as_str()).map(str::to_owned);
        let signer_list = parse_signer_list(result);

        let delegates = self.scan_delegates(&client, &chain.rpc_url, account).await?;

        let authorization = XrpAuthorizationSummary {
            master_key_enabled: !account_flags.disable_master_key,
            regular_key_enabled: regular_key.is_some(),
            multi_signing_enabled: signer_list.is_some(),
            permission_delegation_enabled: !delegates.is_empty(),
        };

        Ok(XrpSecurityScan {
            address: account.to_string(),
            account_exists: true,
            account_flags,
            regular_key,
            signer_list,
            delegates,
            authorization,
        })
    }

    async fn scan_delegates(
        &self,
        client: &reqwest::Client,
        rpc_url: &str,
        account: &str,
    ) -> Result<Vec<XrpDelegate>, EngineError> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "account_objects",
            "params": [{
                "account": account,
                "ledger_index": "validated",
                "type": "delegate"
            }]
        });
        let response = client.post(rpc_url).json(&request).send().await
            .map_err(|e| EngineError::Network(format!("XRP delegate scan RPC failed: {e}")))?;
        let body: serde_json::Value = response.json().await
            .map_err(|e| EngineError::Network(format!("invalid XRP delegate response: {e}")))?;
        if let Some(err) = body.get("result").and_then(|r| r.get("error")) {
            return Err(EngineError::Network(
                body["result"]["error_message"].as_str()
                    .or_else(|| err.as_str()).unwrap_or("XRP delegate scan failed").to_string()
            ));
        }

        let objects = body["result"]["account_objects"].as_array().cloned().unwrap_or_default();
        Ok(objects.into_iter().filter_map(|obj| {
            if obj.get("LedgerEntryType").and_then(|v| v.as_str()) != Some("Delegate") {
                return None;
            }
            let authorize = obj.get("Authorize").and_then(|v| v.as_str())?.to_string();
            let permissions = obj.get("Permissions").and_then(|v| v.as_array()).cloned().unwrap_or_default()
                .into_iter().filter_map(|p| {
                    p.get("Permission").cloned().map(|inner| XrpPermission {
                        value: inner.get("PermissionValue").cloned().unwrap_or(inner)
                    })
                }).collect();
            Some(XrpDelegate { authorize, permissions })
        }).collect())
    }

    pub fn prepare_security_operation(
        &self,
        request: XrpSecurityOperationRequest,
        current: Option<&XrpSecurityScan>,
    ) -> Result<serde_json::Value, EngineError> {
        if request.account.trim().is_empty() {
            return Err(EngineError::Validation("XRP account is required".into()));
        }
        let p = &request.params;
        match request.action.as_str() {
            "set_regular_key" => {
                let key = p.get("regular_key").and_then(|v| v.as_str())
                    .ok_or_else(|| EngineError::Validation("regular_key is required".into()))?;
                Ok(serde_json::json!({"TransactionType":"SetRegularKey","Account":request.account,"RegularKey":key}))
            }
            "remove_regular_key" => Ok(serde_json::json!({"TransactionType":"SetRegularKey","Account":request.account})),
            "disable_master_key" => {
                require_master_key(current)?;
                Ok(serde_json::json!({"TransactionType":"AccountSet","Account":request.account,"SetFlag":4}))
            }
            "enable_master_key" => Ok(serde_json::json!({"TransactionType":"AccountSet","Account":request.account,"ClearFlag":4})),
            "set_account_flag" | "clear_account_flag" => {
                let flag = p.get("flag").and_then(|v| v.as_str())
                    .ok_or_else(|| EngineError::Validation("flag is required".into()))?;
                let n = account_set_flag(flag)?;
                let mut tx=serde_json::json!({"TransactionType":"AccountSet","Account":request.account});
                tx[if request.action=="set_account_flag" {"SetFlag"} else {"ClearFlag"}]=serde_json::json!(n);
                Ok(tx)
            }
            "set_signer_list" => {
                let quorum=p.get("quorum").and_then(|v|v.as_u64()).ok_or_else(||EngineError::Validation("quorum is required".into()))?;
                let signers=p.get("signers").and_then(|v|v.as_array()).ok_or_else(||EngineError::Validation("signers is required".into()))?;
                if signers.is_empty() || signers.len()>32 || quorum==0 { return Err(EngineError::Validation("signer list requires 1..=32 signers and quorum > 0".into())); }
                let entries: Result<Vec<_>,_> = signers.iter().map(|s| {
                    let a=s.get("account").and_then(|v|v.as_str()).ok_or_else(||EngineError::Validation("signer account is required".into()))?;
                    let w=s.get("weight").and_then(|v|v.as_u64()).ok_or_else(||EngineError::Validation("signer weight is required".into()))?;
                    if w==0 || w>u32::MAX as u64 { return Err(EngineError::Validation("invalid signer weight".into())); }
                    Ok(serde_json::json!({"SignerEntry":{"Account":a,"SignerWeight":w}}))
                }).collect();
                Ok(serde_json::json!({"TransactionType":"SignerListSet","Account":request.account,"SignerQuorum":quorum,"SignerEntries":entries?}))
            }
            "remove_signer_list" => Ok(serde_json::json!({"TransactionType":"SignerListSet","Account":request.account,"SignerQuorum":0})),
            "set_delegate" | "update_delegate" => {
                let delegate=p.get("delegate").and_then(|v|v.as_str()).ok_or_else(||EngineError::Validation("delegate is required".into()))?;
                let permissions=p.get("permissions").and_then(|v|v.as_array()).ok_or_else(||EngineError::Validation("permissions is required".into()))?;
                if permissions.len()>10 { return Err(EngineError::Validation("XRPL allows at most 10 delegate permissions".into())); }
                let mut seen=std::collections::HashSet::new();
                let mut out=Vec::new();
                for x in permissions {
                    let v=x.as_str().ok_or_else(||EngineError::Validation("permission must be a string".into()))?;
                    if !seen.insert(v.to_string()) || !valid_permission(v) { return Err(EngineError::Validation(format!("unsupported or duplicate XRPL permission: {v}"))); }
                    out.push(serde_json::json!({"Permission":{"PermissionValue":v}}));
                }
                Ok(serde_json::json!({"TransactionType":"DelegateSet","Account":request.account,"Authorize":delegate,"Permissions":out}))
            }
            "revoke_delegate" => {
                let delegate=p.get("delegate").and_then(|v|v.as_str()).ok_or_else(||EngineError::Validation("delegate is required".into()))?;
                Ok(serde_json::json!({"TransactionType":"DelegateSet","Account":request.account,"Authorize":delegate,"Permissions":[]}))
            }
            "authorize_depositor" | "revoke_depositor" => {
                let other=p.get("authorize").and_then(|v|v.as_str()).ok_or_else(||EngineError::Validation("authorize is required".into()))?;
                let mut tx=serde_json::json!({"TransactionType":"DepositPreauth","Account":request.account});
                tx[if request.action=="authorize_depositor" {"Authorize"} else {"Unauthorize"}]=serde_json::json!(other);
                Ok(tx)
            }
            _ => Err(EngineError::Validation(format!("unsupported XRP security action: {}", request.action)))
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum XrpSecurityStep {
    Prepared,
    AwaitingUserConfirmation,
    Signed,
    AwaitingBroadcastConfirmation,
    Submitted,
    Validated,
    Rejected,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct XrpSecurityConfirmation {
    pub operation_id: String,
    pub account: String,
    pub action: String,
    pub unsigned_transaction: serde_json::Value,
    pub step: XrpSecurityStep,
    pub requires_user_confirmation: bool,
    pub confirmation_message: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct XrpSignedSecurityOperation {
    pub operation_id: String,
    pub account: String,
    pub action: String,
    pub signed_tx_blob: String,
    pub tx_hash: Option<String>,
    pub step: XrpSecurityStep,
    pub requires_broadcast_confirmation: bool,
}

impl XrpAdapter {
    /// Stage 1: prepare an operation and stop before signing.
    pub fn prepare_security_confirmation(
        &self,
        operation_id: String,
        request: XrpSecurityOperationRequest,
        current: Option<&XrpSecurityScan>,
    ) -> Result<XrpSecurityConfirmation, EngineError> {
        let tx = self.prepare_security_operation(request.clone(), current)?;
        Ok(XrpSecurityConfirmation {
            operation_id,
            account: request.account,
            action: request.action,
            unsigned_transaction: tx,
            step: XrpSecurityStep::AwaitingUserConfirmation,
            requires_user_confirmation: true,
            confirmation_message: "Review this XRP security transaction and explicitly confirm before local signing.".into(),
        })
    }

    /// Build the complete transaction shown to the user before the first confirmation.
    /// Network fields are obtained through the shared RPC provider layer.
    pub async fn prepare_security_transaction(
        &self,
        request: XrpSecurityOperationRequest,
        current: Option<&XrpSecurityScan>,
        chain: &ChainConfig,
        rpc: &RpcOrchestrator,
    ) -> Result<XrpSecurityConfirmation, EngineError> {
        if chain.rpc_url.trim().is_empty() {
            return Err(EngineError::Validation("XRP RPC URL is required".into()));
        }
        let account = request.account.clone();
        let account_req = RpcRequest {
            provider: "xrp".into(),
            chain_type: "xrp".into(),
            chain_id: "xrp".into(),
            endpoint: chain.rpc_url.clone(),
            method: "account_info".into(),
            params: serde_json::json!([{"account": account, "ledger_index": "validated"}]),
        };
        let fee_req = RpcRequest {
            provider: "xrp".into(),
            chain_type: "xrp".into(),
            chain_id: "xrp".into(),
            endpoint: chain.rpc_url.clone(),
            method: "fee".into(),
            params: serde_json::json!([]),
        };
        let results = rpc.execute(vec![account_req, fee_req]).await;
        let account_result = results.get(0).and_then(|r| r.as_ref().ok())
            .ok_or_else(|| EngineError::Validation("XRP account_info RPC failed".into()))?;
        let fee_result = results.get(1).and_then(|r| r.as_ref().ok())
            .ok_or_else(|| EngineError::Validation("XRP fee RPC failed".into()))?;
        let account_data = account_result.result.get("account_data")
            .ok_or_else(|| EngineError::Validation("XRP account_info returned no account_data".into()))?;
        let sequence = account_data.get("Sequence").and_then(|v| v.as_u64())
            .ok_or_else(|| EngineError::Validation("XRP account Sequence is missing".into()))?;
        let ledger_index = account_result.result.get("ledger_index").and_then(|v| v.as_u64())
            .ok_or_else(|| EngineError::Validation("XRP validated ledger index is missing".into()))?;
        let fee = fee_result.result.get("drops").and_then(|v| v.get("base_fee"))
            .and_then(|v| v.as_str()).unwrap_or("12");
        let mut tx = self.prepare_security_operation(request.clone(), current)?;
        tx["Sequence"] = serde_json::json!(sequence);
        tx["Fee"] = serde_json::json!(fee);
        tx["LastLedgerSequence"] = serde_json::json!(ledger_index.saturating_add(20));

        Ok(XrpSecurityConfirmation {
            operation_id: format!("xrp-security-{}-{}", request.account, sequence),
            account: request.account,
            action: request.action,
            unsigned_transaction: tx,
            step: XrpSecurityStep::AwaitingUserConfirmation,
            requires_user_confirmation: true,
            confirmation_message: "Review the complete XRP transaction and explicitly confirm before local signing.".into(),
        })
    }

    /// Submit only after the second explicit user confirmation.
    pub async fn broadcast_security_operation(
        &self,
        signed: &XrpSignedSecurityOperation,
        user_confirmed: bool,
        chain: &ChainConfig,
        rpc: &RpcOrchestrator,
    ) -> Result<serde_json::Value, EngineError> {
        let blob = self.authorize_broadcast(signed, user_confirmed)?;
        let request = RpcRequest {
            provider: "xrp".into(),
            chain_type: "xrp".into(),
            chain_id: "xrp".into(),
            endpoint: chain.rpc_url.clone(),
            method: "submit".into(),
            params: serde_json::json!([{"tx_blob": blob}]),
        };
        let result = rpc.execute_one(request).await
            .map_err(EngineError::Unexpected)?;
        Ok(result.result)
    }

    /// Poll the XRP ledger until the submitted transaction is validated.
    pub async fn wait_for_validated(
        &self,
        tx_hash: &str,
        chain: &ChainConfig,
        rpc: &RpcOrchestrator,
    ) -> Result<serde_json::Value, EngineError> {
        for _ in 0..30 {
            let request = RpcRequest {
                provider: "xrp".into(),
                chain_type: "xrp".into(),
                chain_id: "xrp".into(),
                endpoint: chain.rpc_url.clone(),
                method: "tx".into(),
                params: serde_json::json!([tx_hash]),
            };
            if let Ok(response) = rpc.execute_one(request).await {
                let result = response.result;
                if result.get("validated").and_then(|v| v.as_bool()) == Some(true) {
                    return Ok(result);
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        }
        Err(EngineError::Unexpected("timed out waiting for validated XRP transaction".into()))
    }

    /// Stage 2: after the user confirms, the local signer supplies the signed blob.
    /// The adapter never receives or stores the private key.
    pub fn accept_signed_security_operation(
        &self,
        operation_id: String,
        account: String,
        action: String,
        signed_tx_blob: String,
        tx_hash: Option<String>,
    ) -> Result<XrpSignedSecurityOperation, EngineError> {
        if signed_tx_blob.trim().is_empty() {
            return Err(EngineError::Validation("signed XRP transaction blob is required".into()));
        }
        Ok(XrpSignedSecurityOperation {
            operation_id,
            account,
            action,
            signed_tx_blob,
            tx_hash,
            step: XrpSecurityStep::AwaitingBroadcastConfirmation,
            requires_broadcast_confirmation: true,
        })
    }

    /// Stage 3: broadcast is a separate explicit user-confirmed action.
    pub fn authorize_broadcast<'a>(
        &self,
        signed: &'a XrpSignedSecurityOperation,
        user_confirmed: bool,
    ) -> Result<&'a str, EngineError> {
        if !user_confirmed {
            return Err(EngineError::Validation("explicit user confirmation is required before XRP broadcast".into()));
        }
        Ok(signed.signed_tx_blob.as_str())
    }
}

fn normalize_drops(raw: &str) -> Result<String, EngineError> {
    let value = raw.trim().parse::<u128>().map_err(|_| EngineError::Validation("invalid XRP drops amount".into()))?;
    let whole = value / 1_000_000;
    let fraction = value % 1_000_000;
    if fraction == 0 { return Ok(whole.to_string()); }
    let mut f = format!("{:06}", fraction);
    while f.ends_with('0') { f.pop(); }
    Ok(format!("{}.{}", whole, f))
}

impl WalletAdapter for XrpAdapter {
    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> { Self::address_from_public(public_key) }
    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
        let parsed = DerivationPath::parse(path.to_string())?;
        let master = derive_master_key(seed)?;
        let child = derive_child_key(&master, &parsed)?;
        let secret = k256::SecretKey::from_bytes((&child.key).into()).map_err(|_| EngineError::Crypto("invalid XRP derived key".into()))?;
        Self::address_from_public(secret.public_key().to_encoded_point(true).as_bytes())
    }
    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> { sign_secp256k1_payload(private_key, tx_data) }
    fn address_from_private_key(private_key: &[u8]) -> Result<String, EngineError> {
        let secret = k256::SecretKey::from_bytes(private_key.into()).map_err(|_| EngineError::Crypto("invalid XRP private key".into()))?;
        Self::address_from_public(secret.public_key().to_encoded_point(true).as_bytes())
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct XrpSecurityOperationRequest {
    pub account: String,
    pub action: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

fn require_master_key(scan: Option<&XrpSecurityScan>) -> Result<(), EngineError> {
    if scan.map(|s| s.account_flags.disable_master_key).unwrap_or(false) {
        return Err(EngineError::Validation("master key is already disabled".into()));
    }
    Ok(())
}

fn account_set_flag(name: &str) -> Result<u32, EngineError> {
    match name {
        "require_destination_tag" => Ok(1),
        "require_authorization" => Ok(2),
        "disallow_xrp" => Ok(3),
        "disable_master" => Ok(4),
        "no_freeze" => Ok(6),
        "global_freeze" => Ok(7),
        "default_ripple" => Ok(8),
        "deposit_authorization" => Ok(9),
        "allow_trustline_clawback" => Ok(16),
        "allow_trustline_locking" => Ok(17),
        _ => Err(EngineError::Validation(format!("unsupported XRP AccountSet flag: {name}")))
    }
}

fn valid_permission(v: &str) -> bool {
    matches!(v,
        "AccountDomainSet"|"AccountSet"|"AccountDelete"|"CheckCancel"|"CheckCash"|
        "CheckCreate"|"DepositPreauth"|"EscrowCancel"|"EscrowCreate"|"EscrowFinish"|
        "NFTokenAcceptOffer"|"NFTokenBurn"|"NFTokenCancelOffer"|"NFTokenCreateOffer"|
        "NFTokenMint"|"OfferCreate"|"OfferCancel"|"Payment"|"PaymentChannelClaim"|
        "PaymentChannelCreate"|"PaymentChannelFund"|"TrustSet"|"URITokenBurn"|
        "URITokenCreateSellOffer"|"URITokenMint"
    )
}



