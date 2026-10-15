use crate::adapters::tron::TronAdapter;
use crate::common::errors::EngineError;
use crate::services::rpc::RpcOrchestrator;
use ethers::abi::{encode, Token};
use ethers::types::{Address, U256};
use k256::ecdsa::SigningKey;
use serde_json::Value;
use sha2::{Digest as Sha2Digest, Sha256};
use sha3::Keccak256;

pub async fn execute_sign_transaction(
    adapter: &TronAdapter,
    rpc: &RpcOrchestrator,
    message: Value,
) -> Result<Value, EngineError> {
    // Key material is resolved internally by the wallet/key-management
    // layer. It must never arrive from the application transaction JSON.
    let (private_key, from) = resolve_signing_context(&message)?;

    // Permission selection is an adapter concern. If the caller did not
    // provide an explicit permission, the adapter uses the transaction's
    // permission context (defaulting to owner permission 0).
    // Permission selection is adapter-owned. The application never supplies it.

    let mut transaction_message = message.clone();
    if let Some(object) = transaction_message.as_object_mut() {
        object.insert("from".into(), Value::String(from));
    }

    let transaction = if let Some(transaction) = transaction_message.get("transaction").cloned() {
        transaction
    } else {
        construct_transaction(adapter, rpc, &transaction_message, None).await?
    };

    let (transaction_permission_id, _) = tron_transaction_permission_context(&transaction)?;
    let signed = sign_tron_transaction(
        adapter,
        rpc,
        transaction,
        &private_key,
        transaction_permission_id,
    ).await?;

    // Broadcasting belongs to the selected adapter. The transaction service
    // never receives the signed transaction and never performs this step.
    let broadcast_result = adapter
        .broadcast_signed_transaction(rpc, signed.clone())
        .await?;

    Ok(serde_json::json!({
        "ok": true,
        "action": "sign_transaction",
        "status": "broadcasted",
        "transaction": signed,
        "broadcast_result": broadcast_result
    }))
}

async fn construct_transaction(
    adapter: &TronAdapter,
    rpc: &RpcOrchestrator,
    message: &Value,
    permission_id: Option<u32>,
) -> Result<Value, EngineError> {
    let operation = message
        .get("operation")
        .or_else(|| message.get("type"))
        .or_else(|| message.get("sub_action"))
        .or_else(|| message.get("action"))
        .and_then(Value::as_str)
        .ok_or_else(|| EngineError::Validation(
            "sign_transaction message requires operation (native_transfer or token_transfer)".into()
        ))?;

    match operation {
        "native_transfer" | "send_trx" | "send" | "sign_network_transaction" => {
            let from = required_string(message, "from")?;
            let to = required_string(message, "to")?;
            let amount_sun = message
                .get("amount_sun")
                .and_then(Value::as_u64)
                .ok_or_else(|| EngineError::Validation("native_transfer requires amount_sun".into()))?;

            if amount_sun == 0 {
                return Err(EngineError::Validation("TRX amount_sun must be greater than zero".into()));
            }

            let permission_id = permission_id.unwrap_or(0);
            let response = adapter
                .build_trx_transaction(rpc, &from, &to, amount_sun, permission_id)
                .await?;

            extract_transaction(response)
        }

        "token_transfer" | "send_token" | "sign_token_transaction" => {
            build_trc20_transfer(adapter, rpc, message, permission_id.unwrap_or(0)).await
        }

        "batch_transfer" | "multi_send" => Err(EngineError::Unsupported(
            "TRON batch_transfer must be implemented by the adapter for the selected contract; the generic transaction service does not split batches".into()
        )),

        other => Err(EngineError::Unsupported(format!(
            "unsupported TRON sign_transaction operation: {other}"
        ))),
    }
}

async fn build_trc20_transfer(
    adapter: &TronAdapter,
    rpc: &RpcOrchestrator,
    message: &Value,
    permission_id: u32,
) -> Result<Value, EngineError> {
    let from = required_string(message, "from")?;
    let contract = required_string(message, "token_contract")?;
    let to = required_string(message, "to")?;
    let amount = required_string(message, "amount")?;
    // Fee/energy limits are resolved by the adapter/network configuration.
    // They are not part of the normal application transaction contract.

    let owner_hex = tron_address_to_hex_prefixed(&from)?;
    let contract_hex = tron_address_to_hex_prefixed(&contract)?;
    let recipient = tron_address_to_h160(&to)?;
    let amount = U256::from_dec_str(&amount)
        .map_err(|_| EngineError::Validation("token_transfer amount must be a decimal integer".into()))?;

    let parameter = encode(&[
        Token::Address(recipient),
        Token::Uint(amount),
    ]);

    adapter.require_rpc_config()?;
    let response = adapter.execute_tron_rpc(
        rpc,
        "/wallet/triggersmartcontract",
        serde_json::json!({
            "owner_address": owner_hex,
            "contract_address": contract_hex,
            "function_selector": "transfer(address,uint256)",
            "parameter": hex::encode(parameter),
            "fee_limit": message.get("fee_limit").and_then(Value::as_u64).unwrap_or(100_000_000),
            "call_value": 0,
            "Permission_id": permission_id,
            "visible": false
        }),
    ).await?;

    extract_transaction(response)
}

fn extract_transaction(response: Value) -> Result<Value, EngineError> {
    if let Some(transaction) = response.get("transaction").cloned() {
        return Ok(transaction);
    }
    if response.get("raw_data_hex").is_some() {
        return Ok(response);
    }
    Err(EngineError::Validation(
        "TRON transaction construction response did not contain a transaction".into()
    ))
}

fn required_string<'a>(message: &'a Value, field: &str) -> Result<&'a str, EngineError> {
    message
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| EngineError::Validation(format!("sign_transaction message.{field} is required")))
}

fn tron_address_to_h160(address: &str) -> Result<Address, EngineError> {
    let bytes = tron_address_bytes(address)?;
    Ok(Address::from_slice(&bytes[1..21]))
}

fn tron_address_to_hex_prefixed(address: &str) -> Result<String, EngineError> {
    Ok(hex::encode(tron_address_bytes(address)?))
}

fn tron_address_bytes(address: &str) -> Result<Vec<u8>, EngineError> {
    let decoded = bs58::decode(address)
        .into_vec()
        .map_err(|_| EngineError::Validation("invalid TRON Base58 address".into()))?;

    if decoded.len() != 25 || decoded[0] != 0x41 {
        return Err(EngineError::Validation("invalid TRON Base58 address".into()));
    }

    let checksum = Keccak256::digest(&decoded[..21]);
    let checksum2 = Keccak256::digest(&checksum);
    if decoded[21..25] != checksum2[..4] {
        return Err(EngineError::Validation("invalid TRON Base58Check checksum".into()));
    }

    Ok(decoded)
}

pub async fn sign_tron_transaction(
    adapter: &TronAdapter,
    rpc: &RpcOrchestrator,
    mut transaction: Value,
    private_key_hex: &str,
    requested_permission_id: u32,
) -> Result<Value, EngineError> {
    let raw_data_hex = transaction.get("raw_data_hex")
        .and_then(Value::as_str)
        .ok_or_else(|| EngineError::Validation("transaction.raw_data_hex is required for TRON signing".into()))?;
    let raw_data = hex::decode(raw_data_hex)
        .map_err(|_| EngineError::Validation("transaction.raw_data_hex is invalid hexadecimal".into()))?;
    if raw_data.is_empty() {
        return Err(EngineError::Validation("transaction.raw_data_hex cannot be empty".into()));
    }

    let key_hex = private_key_hex.strip_prefix("0x")
        .or_else(|| private_key_hex.strip_prefix("0X"))
        .unwrap_or(private_key_hex);
    let key_bytes = hex::decode(key_hex)
        .map_err(|_| EngineError::Validation("TRON private_key must be hexadecimal".into()))?;
    let key_array: [u8; 32] = key_bytes.try_into()
        .map_err(|_| EngineError::Validation("TRON private_key must be exactly 32 bytes".into()))?;

    let signing_key = SigningKey::from_bytes((&key_array).into())
        .map_err(|_| EngineError::Crypto("invalid TRON secp256k1 private key".into()))?;
    let public = signing_key.verifying_key().to_encoded_point(false);
    let signer_address = crate::adapters::tron::derivation::pub_key_to_address(public.as_bytes())?;

    let (permission_id, contract_type) = tron_transaction_permission_context(&transaction)?;
    if permission_id != requested_permission_id {
        return Err(EngineError::Validation(
            "requested permission_id does not match transaction Permission_id".into()
        ));
    }
    if permission_id == 1 {
        return Err(EngineError::Validation(
            "Witness permission cannot be used for normal transaction signing".into()
        ));
    }

    let permissions = adapter.inspect_account_permissions_via_rpc(rpc, &signer_address).await?;
    TronAdapter::validate_permission_for_signing(
        &permissions,
        permission_id,
        contract_type,
        &signer_address,
    )?;

    let sign_weight = get_sign_weight(adapter, rpc, transaction.clone()).await?;
    if let Some(code) = sign_weight.get("result").and_then(|v| v.get("code")).and_then(Value::as_str) {
        if code == "SIGNATURE_FORMAT_ERROR" {
            return Err(EngineError::Validation("existing TRON transaction signatures are invalid".into()));
        }
    }

    let signer_hex = tron_address_to_hex20_prefixed(&signer_address)?;
    crate::adapters::tron::signing::validate_existing_signatures(
        &transaction,
        sign_weight.get("approved_list").and_then(Value::as_array),
        &signer_hex,
    )?;

    let digest = Sha256::digest(&raw_data);
    let (signature, recovery_id) = signing_key
        .sign_digest_recoverable(Sha256::new_with_prefix(&raw_data))
        .map_err(|_| EngineError::Crypto("TRON transaction signing failed".into()))?;

    let recovery_byte = recovery_id.to_byte();
    if recovery_byte > 1 {
        return Err(EngineError::Crypto("TRON produced an unsupported recovery id".into()));
    }

    let mut signature_bytes = signature.to_bytes().to_vec();
    signature_bytes.push(recovery_byte);
    crate::adapters::tron::signing::append_signature(
        &mut transaction,
        hex::encode(signature_bytes),
    )?;

    let computed_txid = hex::encode(digest);
    if let Some(existing) = transaction.get("txID").and_then(Value::as_str) {
        if !existing.eq_ignore_ascii_case(&computed_txid) {
            return Err(EngineError::Validation(
                "transaction.txID does not match SHA-256(raw_data_hex)".into()
            ));
        }
    } else if let Some(object) = transaction.as_object_mut() {
        object.insert("txID".into(), Value::String(computed_txid));
    }

    Ok(transaction)
}

pub async fn get_sign_weight(
    adapter: &TronAdapter,
    rpc: &RpcOrchestrator,
    transaction: Value,
) -> Result<Value, EngineError> {
    adapter.require_rpc_config()?;
    adapter.execute_tron_rpc(rpc, "/wallet/getsignweight", transaction).await
}

pub async fn get_approved_list(
    adapter: &TronAdapter,
    rpc: &RpcOrchestrator,
    transaction: Value,
) -> Result<Value, EngineError> {
    adapter.require_rpc_config()?;
    adapter.execute_tron_rpc(rpc, "/wallet/getapprovedlist", transaction).await
}

fn tron_transaction_permission_context(transaction: &Value) -> Result<(u32, u16), EngineError> {
    let contract = transaction.get("raw_data")
        .and_then(|v| v.get("contract"))
        .and_then(Value::as_array)
        .and_then(|v| v.first())
        .ok_or_else(|| EngineError::Validation("TRON transaction.raw_data.contract[0] is required".into()))?;

    let contract_name = contract.get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| EngineError::Validation("TRON transaction contract type is missing".into()))?;

    let contract_type = tron_contract_type_id(contract_name)
        .ok_or_else(|| EngineError::Validation(format!("unsupported TRON contract type: {contract_name}")))?;

    let permission_id = contract.get("Permission_id")
        .or_else(|| contract.get("permission_id"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;

    if permission_id > 9 {
        return Err(EngineError::Validation("TRON permission_id must be between 0 and 9".into()));
    }

    Ok((permission_id, contract_type))
}

fn tron_address_to_hex20_prefixed(address: &str) -> Result<String, EngineError> {
    Ok(hex::encode(tron_address_bytes(address)?[..21].to_vec()))
}

fn tron_contract_type_id(name: &str) -> Option<u16> {
    Some(match name {
        "AccountCreateContract" => 0,
        "TransferContract" => 1,
        "TransferAssetContract" => 2,
        "VoteAssetContract" => 3,
        "VoteWitnessContract" => 4,
        "WitnessCreateContract" => 5,
        "AssetIssueContract" => 6,
        "WitnessUpdateContract" => 8,
        "ParticipateAssetIssueContract" => 9,
        "AccountUpdateContract" => 10,
        "FreezeBalanceContract" => 11,
        "UnfreezeBalanceContract" => 12,
        "WithdrawBalanceContract" => 13,
        "UnfreezeAssetContract" => 14,
        "UpdateAssetContract" => 15,
        "ProposalCreateContract" => 16,
        "ProposalApproveContract" => 17,
        "ProposalDeleteContract" => 18,
        "SetAccountIdContract" => 19,
        "CustomContract" => 20,
        "CreateSmartContract" => 30,
        "TriggerSmartContract" => 31,
        "GetContract" => 32,
        "UpdateSettingContract" => 33,
        "ExchangeCreateContract" => 41,
        "ExchangeInjectContract" => 42,
        "ExchangeWithdrawContract" => 43,
        "ExchangeTransactionContract" => 44,
        "UpdateEnergyLimitContract" => 45,
        "AccountPermissionUpdateContract" => 46,
        "ClearABIContract" => 48,
        "UpdateBrokerageContract" => 49,
        "MarketSellAssetContract" => 52,
        "MarketCancelOrderContract" => 53,
        "FreezeBalanceV2Contract" => 54,
        "UnfreezeBalanceV2Contract" => 55,
        "WithdrawExpireUnfreezeContract" => 56,
        "DelegateResourceContract" => 57,
        "UnDelegateResourceContract" => 58,
        "CancelAllUnfreezeV2Contract" => 59,
        _ => return None,
    })
}



fn resolve_signing_context(message: &Value) -> Result<(String, String), EngineError> {
    let private_key = message
        .get("_signing_key_hex")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| EngineError::Validation("wallet signing context is missing".into()))?;
    let key = hex::decode(private_key.strip_prefix("0x").unwrap_or(private_key))
        .map_err(|_| EngineError::Validation("wallet signing context contains an invalid private key".into()))?;
    let from = crate::adapters::tron::derivation::address_from_private_key(&key)?;
    Ok((hex::encode(key), from))
}
