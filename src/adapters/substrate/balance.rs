//! Substrate native-balance fetching and normalization.
//!
//! The adapter builds the System.Account storage key and decodes its SCALE
//! value. Network I/O is exclusively delegated to the shared RPC orchestrator.
use std::hash::Hasher;
use std::sync::Arc;

use blake2::{Blake2b, Digest};
use blake2::digest::consts::U16;
use serde_json::json;
use twox_hash::XxHash64;

use crate::common::errors::EngineError;
use crate::services::balance_service::providers::{
    BalanceProvider, BalanceProviderError, BalanceProviderRequest, BalanceProviderResponse,
};
use crate::services::balance_service::service::{BalanceAsset, ChainBalance};
use crate::services::rpc::providers::RpcRequest;

fn twox128(data: &[u8]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for seed in 0..2 {
        let mut hasher = XxHash64::with_seed(seed);
        hasher.write(data);
        out[(seed as usize) * 8..(seed as usize + 1) * 8]
            .copy_from_slice(&hasher.finish().to_le_bytes());
    }
    out
}

fn account_id32(address: &str) -> Result<[u8; 32], BalanceProviderError> {
    let decoded = bs58::decode(address.trim()).into_vec().map_err(|e| {
        BalanceProviderError::Message(format!("invalid Substrate SS58 address: {e}"))
    })?;
    if decoded.len() < 35 {
        return Err(BalanceProviderError::Message(
            "Substrate SS58 address is too short for AccountId32".into(),
        ));
    }
    let prefix_len = if decoded[0] & 0b0100_0000 == 0 { 1 } else { 2 };
    if decoded.len() != prefix_len + 32 + 2 {
        return Err(BalanceProviderError::Message(
            "Substrate address is not an SS58 AccountId32 address".into(),
        ));
    }
    let payload_end = decoded.len() - 2;
    let payload = &decoded[prefix_len..payload_end];
    let mut checksum_input = b"SS58PRE".to_vec();
    checksum_input.extend_from_slice(&decoded[..payload_end]);
    let checksum = blake2::Blake2b512::digest(&checksum_input);
    if decoded[payload_end..] != checksum[..2] {
        return Err(BalanceProviderError::Message(
            "Substrate SS58 address checksum is invalid".into(),
        ));
    }
    let mut account = [0u8; 32];
    account.copy_from_slice(payload);
    Ok(account)
}

fn system_account_storage_key(address: &str) -> Result<String, BalanceProviderError> {
    let account = account_id32(address)?;
    let mut key = Vec::with_capacity(80);
    key.extend_from_slice(&twox128(b"System"));
    key.extend_from_slice(&twox128(b"Account"));
    let account_hash = Blake2b::<U16>::digest(account);
    key.extend_from_slice(&account_hash);
    key.extend_from_slice(&account);
    Ok(format!("0x{}", hex::encode(key)))
}

/// System.Account begins with four 32-bit account counters, then AccountData.
/// The first AccountData field, free, is a little-endian u128.
fn decode_free_balance(storage: &serde_json::Value) -> Result<u128, BalanceProviderError> {
    if storage.is_null() {
        return Ok(0);
    }
    let encoded = storage.as_str().ok_or_else(|| {
        BalanceProviderError::Message("Substrate state_getStorage returned a non-hex value".into())
    })?;
    let bytes = hex::decode(encoded.trim_start_matches("0x")).map_err(|e| {
        BalanceProviderError::Message(format!("invalid Substrate SCALE storage hex: {e}"))
    })?;
    if bytes.len() < 32 {
        return Err(BalanceProviderError::Message(format!(
            "Substrate System.Account value is too short ({} bytes)", bytes.len()
        )));
    }
    let mut amount = [0u8; 16];
    amount.copy_from_slice(&bytes[16..32]);
    Ok(u128::from_le_bytes(amount))
}

pub fn normalize(mut balance: ChainBalance) -> Result<ChainBalance, EngineError> {
    let native = balance.native.take().ok_or_else(|| {
        EngineError::Validation("Substrate balance response has no native asset".into())
    })?;
    let rows: Vec<crate::adapters::substrate::AssetRow> =
        std::iter::once(native.clone())
            .chain(balance.tokens.iter().cloned())
            .map(|asset| crate::adapters::substrate::AssetRow {
                address: asset.address,
                symbol: asset.symbol.unwrap_or_default(),
                decimals: asset.decimals,
                is_native: asset.is_native,
                balance: asset.balance,
            })
            .collect();
    let rows = crate::adapters::substrate::SubstrateAdapter::normalize_balances(rows)?;
    let mut rows = rows.into_iter();
    let first = rows.next().ok_or_else(|| {
        EngineError::Validation("Substrate adapter returned no normalized native balance".into())
    })?;
    balance.native = Some(BalanceAsset {
        address: first.address,
        symbol: if first.symbol.is_empty() { native.symbol.clone() } else { Some(first.symbol) },
        name: native.name,
        decimals: first.decimals,
        balance: first.balance,
        is_native: true,
    });
    balance.tokens = rows.map(|asset| BalanceAsset {
        address: asset.address,
        symbol: if asset.symbol.is_empty() { None } else { Some(asset.symbol) },
        name: None,
        decimals: asset.decimals,
        balance: asset.balance,
        is_native: false,
    }).collect();
    Ok(balance)
}

/// Fetch Substrate native balances via the shared RPC orchestrator. No HTTP
/// client or provider-specific transport is created in this adapter.
pub async fn fetch_batch(
    _provider: Arc<dyn BalanceProvider>,
    requests: Vec<BalanceProviderRequest>,
) -> Result<Vec<BalanceProviderResponse>, BalanceProviderError> {
    let mut output = Vec::with_capacity(requests.len());
    for wrapped in requests {
        let request = wrapped.request;
        if request.address.trim().is_empty() {
            output.push(BalanceProviderResponse::error(
                &request,
                BalanceProviderError::Message("missing Substrate account address".into()),
            ));
            continue;
        }
        if request.rpc_endpoint.trim().is_empty() {
            output.push(BalanceProviderResponse::error(
                &request,
                BalanceProviderError::Message("missing Substrate RPC endpoint".into()),
            ));
            continue;
        }

        let key = match system_account_storage_key(&request.address) {
            Ok(key) => key,
            Err(error) => {
                output.push(BalanceProviderResponse::error(&request, error));
                continue;
            }
        };
        let rpc_provider = if request.provider.trim().is_empty() { "public" } else { request.provider.as_str() };
        let rpc_result = wrapped.rpc.execute_one(RpcRequest {
            provider: rpc_provider.to_owned(),
            chain_type: "substrate".into(),
            chain_id: request.chain.chain_id.clone(),
            endpoint: request.rpc_endpoint.clone(),
            method: "state_getStorage".into(),
            params: json!([key]),
        }).await;

        let raw = match rpc_result {
            Ok(response) => response.result,
            Err(error) => {
                output.push(BalanceProviderResponse::error(
                    &request,
                    BalanceProviderError::Message(error),
                ));
                continue;
            }
        };
        let amount = match decode_free_balance(&raw) {
            Ok(amount) => amount,
            Err(error) => {
                output.push(BalanceProviderResponse::error(&request, error));
                continue;
            }
        };

        let network = format!("{} {}", request.chain.name, request.chain.network.as_deref().unwrap_or(""));
        let lower = network.to_ascii_lowercase();
        let (symbol, decimals) = if lower.contains("kusama") || lower.contains("westend") {
            (request.chain.symbol.clone().unwrap_or_else(|| if lower.contains("kusama") { "KSM".into() } else { "WND".into() }), 12)
        } else {
            (request.chain.symbol.clone().unwrap_or_else(|| "DOT".into()), 10)
        };
        let raw_balance = ChainBalance {
            wallet_id: request.wallet_id.clone(),
            chain_id: request.chain.chain_id.clone(),
            chain_type: request.chain.chain_type.clone(),
            address: request.address.clone(),
            native: Some(BalanceAsset {
                address: "native".into(),
                symbol: Some(symbol),
                name: Some(request.chain.name.clone()),
                decimals: Some(decimals),
                balance: amount.to_string(),
                is_native: true,
            }),
            // Generic Substrate token balances require runtime-specific asset
            // storage metadata; never fabricate token balances from native storage.
            tokens: Vec::new(),
        };
        match normalize(raw_balance) {
            Ok(balance) => output.push(BalanceProviderResponse::success(balance)),
            Err(error) => output.push(BalanceProviderResponse::error(
                &request,
                BalanceProviderError::Message(error.to_string()),
            )),
        }
    }
    Ok(output)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_system_account_storage_key_from_ss58_address() {
        let address = crate::adapters::substrate::SubstrateAdapter::address_for_chain(&[7u8; 32], 0).unwrap();
        let key = system_account_storage_key(&address).unwrap();
        assert!(key.starts_with("0x"));
        assert_eq!(key.len(), 162); // 80-byte storage key, hex encoded.
    }

    #[test]
    fn decodes_free_balance_from_scale_account_info() {
        let mut encoded = vec![0u8; 32];
        encoded[16..32].copy_from_slice(&123456789u128.to_le_bytes());
        assert_eq!(decode_free_balance(&serde_json::Value::String(format!("0x{}", hex::encode(encoded)))).unwrap(), 123456789);
    }

    #[test]
    fn missing_substrate_account_has_zero_balance() {
        assert_eq!(decode_free_balance(&serde_json::Value::Null).unwrap(), 0);
    }

    #[test]
    fn validates_ss58_checksum() {
        let address = crate::adapters::substrate::SubstrateAdapter::address_for_chain(&[9u8; 32], 0).unwrap();
        assert!(account_id32(&address).is_ok());
        let mut decoded = bs58::decode(address).into_vec().unwrap();
        decoded[4] ^= 1;
        let corrupted = bs58::encode(decoded).into_string();
        assert!(account_id32(&corrupted).is_err());
    }
}
