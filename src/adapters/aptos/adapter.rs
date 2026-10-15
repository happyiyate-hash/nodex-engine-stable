use crate::adapters::adapter_trait::WalletAdapter;
use crate::common::errors::EngineError;
use crate::crypto::bip32::{derive_child_key_ed25519, derive_master_key_ed25519};
use crate::crypto::chain_addresses::aptos_address_from_public_key;
use crate::crypto::derivation::DerivationPath;
use crate::crypto::signing::sign_ed25519_payload;
use crate::types::chain::ChainRequest;
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;

pub struct AptosAdapter;

impl AptosAdapter {
    fn validate_path(path: &str) -> Result<(), EngineError> {
        let parsed = DerivationPath::parse(path.to_string())?;
        if parsed.segments.len() != 5 || parsed.segments.iter().any(|s| !s.is_hardened) {
            return Err(EngineError::Validation(
                "Aptos derivation_path must be a five-level hardened path such as m/44'/637'/0'/0'/0'".into(),
            ));
        }
        if parsed.segments[0].index != 44 || parsed.segments[1].index != 637 {
            return Err(EngineError::Validation(
                "Aptos derivation_path must use purpose 44' and coin type 637'".into(),
            ));
        }
        Ok(())
    }

    fn derive_public_key(seed: &[u8], path: &str) -> Result<[u8; 32], EngineError> {
        Self::validate_path(path)?;
        let master = derive_master_key_ed25519(seed)?;
        let parsed = DerivationPath::parse(path.to_string())?;
        let child = derive_child_key_ed25519(&master, &parsed)?;
        let secret = ed25519_dalek::SecretKey::from_bytes(&child.key)
            .map_err(|_| EngineError::Crypto("invalid Aptos Ed25519 private key".into()))?;
        Ok(ed25519_dalek::PublicKey::from(&secret).to_bytes())
    }

    fn transaction_signing_digest(tx_data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(b"APTOS::RawTransaction");
        hasher.update(tx_data);
        hasher.finalize().into()
    }
}

impl WalletAdapter for AptosAdapter {
    fn derive_addresses(
        seed: &[u8],
        chains: &[ChainRequest],
    ) -> Result<HashMap<String, String>, EngineError> {
        let mut out = HashMap::with_capacity(chains.len());
        for chain in chains {
            let path = chain.derivation_path.as_deref().ok_or_else(|| {
                EngineError::Validation(format!(
                    "derivation_path is required for Aptos blockchain {}",
                    chain.name
                ))
            })?;
            let public_key = Self::derive_public_key(seed, path)?;
            out.insert(chain.name.clone(), aptos_address_from_public_key(&public_key)?);
        }
        Ok(out)
    }

    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
        aptos_address_from_public_key(public_key)
    }

    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
        let public_key = Self::derive_public_key(seed, path)?;
        aptos_address_from_public_key(&public_key)
    }

    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        if private_key.len() != 32 {
            return Err(EngineError::Validation(
                "Aptos Ed25519 private key must be 32 bytes".into(),
            ));
        }
        let digest = Self::transaction_signing_digest(tx_data);
        sign_ed25519_payload(private_key, &digest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aptos_requires_standard_hardened_path() {
        assert!(AptosAdapter::validate_path("m/44'/637'/0'/0'/0'").is_ok());
        assert!(AptosAdapter::validate_path("m/44'/60'/0'/0'/0'").is_err());
        assert!(AptosAdapter::validate_path("m/44'/637'/0'/0/0").is_err());
    }

    #[test]
    fn aptos_derives_address() {
        let chains = vec![ChainRequest {
            name: "Aptos".into(),
            chain_id: "1".into(),
            symbol: Some("APT".into()),
            network: Some("mainnet".into()),
            chain_type: "aptos".into(),
            derivation_path: Some("m/44'/637'/0'/0'/0'".into()),
            address_type: None,
            p2pkh_version: None,
            p2sh_version: None,
            bech32_hrp: None,
            rpc_provider: None,
            rpc_url: None,
        }];
        let result = AptosAdapter::derive_addresses(&[7u8; 64], &chains).unwrap();
        let address = result.get("Aptos").unwrap();
        assert_eq!(address.len(), 66);
        assert!(address.starts_with("0x"));
    }

    #[test]
    fn aptos_requires_explicit_derivation_path() {
        let chain = ChainRequest::new("Aptos", "aptos");
        assert!(AptosAdapter::derive_addresses(&[7u8; 64], &[chain]).is_err());
    }

    #[test]
    fn aptos_signing_digest_is_32_bytes() {
        assert_eq!(AptosAdapter::transaction_signing_digest(b"test").len(), 32);
    }
}
