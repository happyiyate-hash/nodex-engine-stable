use crate::common::errors::EngineError;
use serde_json::{json, Value};


pub trait WalletAdapter {
    /// Handles the complete batch derivation contract for this adapter family.
    /// The adapter owns grouping, derivation-path policy, and address generation.
    fn derive_addresses(seed: &[u8], chains: &[crate::types::chain::ChainRequest]) -> Result<std::collections::HashMap<String, String>, EngineError> {
        let mut out = std::collections::HashMap::with_capacity(chains.len());
        for chain in chains {
            let path = chain.derivation_path.as_deref().ok_or_else(|| EngineError::Validation(
                format!("derivation_path is required for adapter {}", chain.chain_type)
            ))?;
            let address = Self::derive_address_from_path(seed, path)?;
            out.insert(chain.name.clone(), address);
        }
        Ok(out)
    }

    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError>;
    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError>;

    /// Optional raw derivation primitives. Adapters that support them expose
    /// them through the same engine action contract; unsupported operations
    /// return EngineError::Unsupported instead of being silently emulated.
    fn default_derivation_path() -> Option<&'static str> {
        None
    }

    fn derive_address_at_index(seed: &[u8], index: u32) -> Result<String, EngineError> {
        let _ = (seed, index);
        Err(EngineError::Unsupported("indexed address derivation is not supported by this adapter".into()))
    }

    fn derive_addresses_at_index(
        seed: &[u8],
        start_index: u32,
        count: u32,
    ) -> Result<Vec<String>, EngineError> {
        let end = start_index.checked_add(count)
            .ok_or_else(|| EngineError::Validation("address index range overflow".into()))?;
        (start_index..end)
            .map(|index| Self::derive_address_at_index(seed, index))
            .collect()
    }

    /// Optional structured child enumeration. Implementations return the
    /// deterministic child index/path/address tuple without exposing keys.
    fn derive_child_accounts(
        seed: &[u8],
        start_index: u32,
        count: u32,
    ) -> Result<Value, EngineError> {
        let _ = (seed, start_index, count);
        Err(EngineError::Unsupported("structured child enumeration is not supported by this adapter".into()))
    }

    fn derive_private_key(seed: &[u8], path: &str) -> Result<Vec<u8>, EngineError> {
        let _ = (seed, path);
        Err(EngineError::Unsupported("private-key derivation is not supported by this adapter".into()))
    }

    fn derive_public_key(seed: &[u8], path: &str) -> Result<Vec<u8>, EngineError> {
        let _ = (seed, path);
        Err(EngineError::Unsupported("public-key derivation is not supported by this adapter".into()))
    }

    fn derive_child_key(seed: &[u8], path: &str) -> Result<Value, EngineError> {
        let _ = (seed, path);
        Err(EngineError::Unsupported("child-key derivation is not supported by this adapter".into()))
    }

    /// Uniform adapter operation dispatcher. The wallet service supplies the
    /// seed; callers only send action + chain_type + derivation parameters.
    fn handle_derivation_request(seed: &[u8], action: &str, params: &Value) -> Result<Value, EngineError> {
        match action {
            "derive_address" => {
                let address = if let Some(index) = params.get("index").and_then(Value::as_u64) {
                    Self::derive_address_at_index(seed, u32::try_from(index)
                        .map_err(|_| EngineError::Validation("index is out of range".into()))?)?
                } else {
                    let path = params.get("path")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .or_else(|| Self::default_derivation_path().map(str::to_owned))
                        .ok_or_else(|| EngineError::Validation(
                            "derive_address requires path or an adapter default derivation path".into()
                        ))?;
                    Self::derive_address_from_path(seed, &path)?
                };
                Ok(json!({ "address": address }))
            }
            "derive_addresses" => {
                let addresses = if let (Some(start), Some(count)) = (
                    params.get("start_index").and_then(Value::as_u64),
                    params.get("count").and_then(Value::as_u64),
                ) {
                    Self::derive_addresses_at_index(
                        seed,
                        u32::try_from(start).map_err(|_| EngineError::Validation("start_index is out of range".into()))?,
                        u32::try_from(count).map_err(|_| EngineError::Validation("count is out of range".into()))?,
                    )?
                } else {
                    let paths = params.get("paths")
                        .and_then(Value::as_array)
                        .ok_or_else(|| EngineError::Validation(
                            "derive_addresses requires start_index/count or paths".into()
                        ))?;
                    paths.iter().map(|path| {
                        let path = path.as_str()
                            .ok_or_else(|| EngineError::Validation("derive_addresses paths must be strings".into()))?;
                        Self::derive_address_from_path(seed, path)
                    }).collect::<Result<Vec<_>, _>>()?
                };
                Ok(json!({ "addresses": addresses }))
            }
            "derive_private_key" => {
                let path = params.get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| EngineError::Validation("derive_private_key requires path".into()))?;
                Ok(json!({ "private_key_hex": hex::encode(Self::derive_private_key(seed, path)?) }))
            }
            "derive_public_key" => {
                let path = params.get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| EngineError::Validation("derive_public_key requires path".into()))?;
                Ok(json!({ "public_key_hex": hex::encode(Self::derive_public_key(seed, path)?) }))
            }
            "derive_child_key" => {
                let path = params.get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| EngineError::Validation("derive_child_key requires path".into()))?;
                Self::derive_child_key(seed, path)
            },
            "derive_child_accounts" => {
                let start = params.get("start_index").and_then(Value::as_u64)
                    .ok_or_else(|| EngineError::Validation("derive_child_accounts requires start_index".into()))?;
                let count = params.get("count").and_then(Value::as_u64)
                    .ok_or_else(|| EngineError::Validation("derive_child_accounts requires count".into()))?;
                Self::derive_child_accounts(
                    seed,
                    u32::try_from(start).map_err(|_| EngineError::Validation("start_index is out of range".into()))?,
                    u32::try_from(count).map_err(|_| EngineError::Validation("count is out of range".into()))?,
                )
            }
            _ => Err(EngineError::Unsupported(format!("unsupported derivation action: {action}"))),
        }
    }

    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError>;
    fn address_from_private_key(private_key: &[u8]) -> Result<String, EngineError> {
        let _ = private_key;
        Err(EngineError::Unsupported("raw private-key import is not supported by this adapter".into()))
    }
}
