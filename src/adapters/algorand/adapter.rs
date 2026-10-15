use crate::adapters::adapter_trait::WalletAdapter;
use crate::common::errors::EngineError;
use crate::crypto::bip32::{derive_child_key_ed25519, derive_master_key_ed25519};
use crate::crypto::derivation::DerivationPath;
use crate::crypto::signing::sign_ed25519_payload;
use ed25519_dalek::{PublicKey, SecretKey};
use sha2::{Digest, Sha512_256};
use std::collections::HashMap;

/// Algorand wallet adapter.
///
/// Algorand is a standalone adapter. It owns key derivation, Algorand address
/// construction, transaction signing, and chain-specific balance normalization.
/// RPC/API access and balance fetching remain in the provider layer.
pub struct AlgorandAdapter;

#[derive(Clone, Debug)]
pub struct AssetRow {
    pub address: String,
    pub symbol: String,
    pub decimals: Option<u32>,
    pub is_native: bool,
    pub balance: String,
}

impl AlgorandAdapter {
    /// Algorand SLIP-44 coin type is 283.
    /// Standard path:
    /// m/44'/283'/account'/change'/address_index'
    fn validate_path(path: &str) -> Result<(), EngineError> {
        let parsed = DerivationPath::parse(path.to_string())?;
        if parsed.segments.len() != 5 || parsed.segments.iter().any(|s| !s.is_hardened) {
            return Err(EngineError::Validation(
                "Algorand derivation_path must be a five-level hardened path such as m/44'/283'/0'/0'/0'".into(),
            ));
        }
        if parsed.segments[0].index != 44 || parsed.segments[1].index != 283 {
            return Err(EngineError::Validation(
                "Algorand derivation_path must use purpose 44' and coin type 283'".into(),
            ));
        }
        Ok(())
    }

    fn derive_public_key(seed: &[u8], path: &str) -> Result<[u8; 32], EngineError> {
        Self::validate_path(path)?;
        let master = derive_master_key_ed25519(seed)?;
        let parsed = DerivationPath::parse(path.to_string())?;
        let child = derive_child_key_ed25519(&master, &parsed)?;
        let secret = SecretKey::from_bytes(&child.key)
            .map_err(|_| EngineError::Crypto("invalid Algorand Ed25519 private key".into()))?;
        Ok(PublicKey::from(&secret).to_bytes())
    }

    /// Algorand address = base32(no padding, uppercase)(
    /// public_key || SHA512/256(public_key)[28..32]
    /// ).
    pub fn address_from_public_key(public_key: &[u8]) -> Result<String, EngineError> {
        if public_key.len() != 32 {
            return Err(EngineError::Validation(
                "Algorand public key must be 32 bytes".into(),
            ));
        }
        let checksum = Sha512_256::digest(public_key);
        let mut payload = Vec::with_capacity(36);
        payload.extend_from_slice(public_key);
        payload.extend_from_slice(&checksum[28..32]);
        Ok(base32_no_padding(&payload))
    }

    /// Convert raw provider amounts to decimal quantities without floating point.
    pub fn normalize_balances(assets: Vec<AssetRow>) -> Result<Vec<AssetRow>, EngineError> {
        assets.into_iter()
            .map(|mut asset| {
                let decimals = asset.decimals.ok_or_else(|| EngineError::Validation(
                    format!("missing decimals for Algorand asset {}", asset.address)
                ))?;
                asset.balance = normalize_raw_amount(&asset.balance, decimals)?;
                Ok(asset)
            })
            .collect()
    }

    /// Algorand transaction bytes must be the canonical msgpack-encoded
    /// transaction. The adapter applies the Algorand transaction domain tag.
    fn signing_message(tx_data: &[u8]) -> Vec<u8> {
        let mut message = Vec::with_capacity(2 + tx_data.len());
        message.extend_from_slice(b"TX");
        message.extend_from_slice(tx_data);
        message
    }
}

fn base32_no_padding(data: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::with_capacity((data.len() * 8 + 4) / 5);
    let mut acc = 0u32;
    let mut bits = 0u8;
    for &byte in data {
        acc = (acc << 8) | byte as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((acc >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((acc << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn normalize_raw_amount(raw: &str, decimals: u32) -> Result<String, EngineError> {
    if raw.trim().is_empty() || raw.starts_with('-') || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err(EngineError::Validation("invalid Algorand asset amount".into()));
    }
    let value = raw.trim().parse::<u128>()
        .map_err(|_| EngineError::Validation("Algorand asset amount exceeds u128".into()))?;
    if decimals == 0 {
        return Ok(value.to_string());
    }
    let scale = 10u128.checked_pow(decimals)
        .ok_or_else(|| EngineError::Validation("Algorand decimals exceed supported range".into()))?;
    let whole = value / scale;
    let fraction = value % scale;
    if fraction == 0 {
        return Ok(whole.to_string());
    }
    let mut fraction_text = format!("{:0width$}", fraction, width = decimals as usize);
    while fraction_text.ends_with('0') {
        fraction_text.pop();
    }
    Ok(format!("{}.{}", whole, fraction_text))
}

impl WalletAdapter for AlgorandAdapter {
    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
        Self::address_from_public_key(public_key)
    }

    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
        let public_key = Self::derive_public_key(seed, path)?;
        Self::address_from_public_key(&public_key)
    }

    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        if private_key.len() != 32 {
            return Err(EngineError::Validation(
                "Algorand Ed25519 private key must be 32 bytes".into(),
            ));
        }
        sign_ed25519_payload(private_key, &Self::signing_message(tx_data))
    }

    fn address_from_private_key(private_key: &[u8]) -> Result<String, EngineError> {
        let secret = SecretKey::from_bytes(private_key)
            .map_err(|_| EngineError::Crypto("invalid Algorand Ed25519 private key".into()))?;
        Self::address_from_public_key(PublicKey::from(&secret).as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algorand_requires_standard_hardened_path() {
        assert!(AlgorandAdapter::validate_path("m/44'/283'/0'/0'/0'").is_ok());
        assert!(AlgorandAdapter::validate_path("m/44'/60'/0'/0'/0'").is_err());
        assert!(AlgorandAdapter::validate_path("m/44'/283'/0'/0/0").is_err());
    }

    #[test]
    fn algorand_address_has_expected_shape() {
        let address = AlgorandAdapter::pub_key_to_address(&[0u8; 32]).unwrap();
        assert_eq!(address.len(), 58);
        assert_eq!(address, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAY5HFKQ");
        assert!(address.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()));
    }

    #[test]
    fn algorand_normalizes_base_units() {
        let assets = vec![AssetRow {
            address: "0".into(),
            symbol: "ALGO".into(),
            decimals: Some(6),
            is_native: true,
            balance: "1234567".into(),
        }];
        let normalized = AlgorandAdapter::normalize_balances(assets).unwrap();
        assert_eq!(normalized[0].balance, "1.234567");
    }

    #[test]
    fn algorand_signing_message_has_domain_tag() {
        assert_eq!(&AlgorandAdapter::signing_message(b"abc")[..2], b"TX");
    }
}
