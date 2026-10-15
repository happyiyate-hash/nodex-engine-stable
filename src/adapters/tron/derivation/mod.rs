use crate::common::errors::EngineError;
use crate::crypto::address::base58check_address;
use crate::crypto::bip32::{derive_child_key, derive_master_key, ExtendedKey};
use crate::crypto::derivation::DerivationPath;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use sha3::{Digest, Keccak256};

/// Canonical BIP-44 account path for the first TRON address.
pub const TRON_DEFAULT_DERIVATION_PATH: &str = "m/44'/195'/0'/0/0";

/// Canonical BIP-44 external-chain prefix used for indexed TRON addresses.
pub const TRON_EXTERNAL_CHAIN_PATH: &str = "m/44'/195'/0'/0";

#[derive(Debug, Clone, serde::Serialize)]
pub struct TronDerivedChild {
    pub child_index: u32,
    pub derivation_path: String,
    pub address: String,
}

/// Derive the secp256k1 master key from a wallet seed.
pub fn derive_master(seed: &[u8]) -> Result<ExtendedKey, EngineError> {
    derive_master_key(seed)
}

/// Derive child key material from an already-derived parent and relative path.
pub fn derive_child(parent: &ExtendedKey, path: &DerivationPath) -> Result<ExtendedKey, EngineError> {
    derive_child_key(parent, path)
}

/// Derive a private child key directly from a wallet seed and absolute derivation path.
pub fn derive_private_key(seed: &[u8], path: &str) -> Result<[u8; 32], EngineError> {
    let master = derive_master(seed)?;
    let parsed = DerivationPath::parse(path.to_string())?;
    Ok(derive_child(&master, &parsed)?.key)
}

/// Derive a public key directly from a wallet seed and absolute derivation path.
///
/// The returned key is the uncompressed 65-byte SEC1 representation because
/// TRON address generation hashes the 64-byte X/Y public-key payload.
pub fn derive_public_key(seed: &[u8], path: &str) -> Result<Vec<u8>, EngineError> {
    let private_key = derive_private_key(seed, path)?;
    public_key_from_private_key(&private_key)
}

/// Derive the address at m/44'/195'/0'/0/<index>.
///
/// This is the normal HD-wallet mechanism for generating additional TRON
/// accounts/addresses from the same wallet seed.
pub fn derive_address_at_index(seed: &[u8], index: u32) -> Result<String, EngineError> {
    let path = format!("{TRON_EXTERNAL_CHAIN_PATH}/{index}");
    derive_address_from_path(seed, &path)
}

/// Derive a contiguous range of TRON external addresses from one wallet seed.
pub fn derive_addresses(
    seed: &[u8],
    start_index: u32,
    count: u32,
) -> Result<Vec<String>, EngineError> {
    let end = start_index
        .checked_add(count)
        .ok_or_else(|| EngineError::Validation("TRON address index range overflow".into()))?;

    (start_index..end)
        .map(|index| derive_address_at_index(seed, index))
        .collect()
}

/// Derive logical TRON children from one wallet seed.
///
/// A child is identified by its deterministic index. The same index can be
/// used by the wallet layer to group this TRON account with the corresponding
/// Ethereum/Solana/etc. account derived from the same root secret.
pub fn derive_child_accounts(
    seed: &[u8],
    start_index: u32,
    count: u32,
) -> Result<Vec<TronDerivedChild>, EngineError> {
    let end = start_index
        .checked_add(count)
        .ok_or_else(|| EngineError::Validation("TRON child index range overflow".into()))?;

    (start_index..end)
        .map(|child_index| {
            let derivation_path = format!("{TRON_EXTERNAL_CHAIN_PATH}/{child_index}");
            let address = derive_address_from_path(seed, &derivation_path)?;
            Ok(TronDerivedChild {
                child_index,
                derivation_path,
                address,
            })
        })
        .collect()
}

/// Convert a private secp256k1 key into its uncompressed public key.
pub fn public_key_from_private_key(private_key: &[u8; 32]) -> Result<Vec<u8>, EngineError> {
    let secret = k256::SecretKey::from_bytes(private_key.into())
        .map_err(|_| EngineError::Crypto("invalid secp256k1 private key".into()))?;

    Ok(secret
        .public_key()
        .to_encoded_point(false)
        .as_bytes()
        .to_vec())
}

/// Convert a secp256k1 public key into a TRON Base58Check address.
///
/// Both compressed and uncompressed SEC1 input are accepted and normalized to
/// the uncompressed point before Keccak-256 hashing.
pub fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
    let encoded = match public_key.len() {
        33 | 65 => k256::PublicKey::from_sec1_bytes(public_key)
            .map_err(|_| EngineError::Validation("invalid secp256k1 public key".into()))?
            .to_encoded_point(false),
        _ => {
            return Err(EngineError::Validation(
                "TRON public key must be 33 or 65 bytes".into(),
            ))
        }
    };

    let digest = Keccak256::digest(&encoded.as_bytes()[1..]);
    Ok(base58check_address(0x41, &digest[12..]))
}

/// Convert a raw 32-byte secp256k1 private key into a TRON address.
pub fn address_from_private_key(private_key: &[u8]) -> Result<String, EngineError> {
    if private_key.len() != 32 {
        return Err(EngineError::Validation(
            "TRON private key must be exactly 32 bytes".into(),
        ));
    }

    let private_key: [u8; 32] = private_key
        .try_into()
        .map_err(|_| EngineError::Validation("invalid TRON private key length".into()))?;

    let public_key = public_key_from_private_key(&private_key)?;
    pub_key_to_address(&public_key)
}

/// Derive a TRON address from any valid absolute HD derivation path.
///
/// Unlike the old implementation, path is honored. Callers can therefore
/// derive /0, /1, /2, or an explicitly supplied account/change path instead
/// of silently receiving the first canonical address.
pub fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
    let private_key = derive_private_key(seed, path)?;
    address_from_private_key(&private_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_is_the_first_tron_external_address() {
        assert_eq!(TRON_DEFAULT_DERIVATION_PATH, "m/44'/195'/0'/0/0");
    }

    #[test]
    fn indexed_path_is_distinct_for_different_children() {
        let seed = [7u8; 32];
        let first = derive_address_at_index(&seed, 0).unwrap();
        let second = derive_address_at_index(&seed, 1).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn address_from_private_key_rejects_wrong_length() {
        assert!(address_from_private_key(&[1u8; 31]).is_err());
        assert!(address_from_private_key(&[1u8; 33]).is_err());
    }

    #[test]
    fn matches_tron_tip_bip39_bip32_vector() {
        // TRON TIP-01 Test 2 supplies this mnemonic/seed and the expected
        // account-level key for m/44'/195'/1'.
        let mnemonic = crate::crypto::seed::Mnemonic::new(
            "ceiling present easily example nurse silver ecology plug accident decrease special aware",
        ).unwrap();
        let seed = crate::crypto::seed::derive_master_seed(&mnemonic, None).unwrap();

        let expected_seed = "e0977fc24a90cf457a12c3e074e337abefa2efab4646324a20c80c9308bc19f1be4cab879f6b7644841f3a2076b65421fca6eae42200b8b6e72efdb471ff4ac5";
        assert_eq!(hex::encode(&seed), expected_seed);

        let private_key = derive_private_key(&seed, "m/44'/195'/1'").unwrap();
        assert_eq!(
            hex::encode(private_key),
            "76cbcbc004e3d8c2bd6bb91168ac8ff14615d846d15e07c1837da3410ff013de"
        );

        let public_key = derive_public_key(&seed, "m/44'/195'/1'").unwrap();
        assert_eq!(
            hex::encode(public_key),
            "0472d3e4254b126ee7ef47e58b9407e073e9a2935d4442667859278333dab9e2bdd29366e5be98cdb47b46f56756d03b32157aaf646fbf42b868f65e9b0d98e50b"
        );
    }

    #[test]
    fn current_tron_address_format_is_41_prefixed_base58() {
        let private_key = hex::decode(
            "76cbcbc004e3d8c2bd6bb91168ac8ff14615d846d15e07c1837da3410ff013de",
        ).unwrap();
        let address = address_from_private_key(&private_key).unwrap();
        assert_eq!(address.len(), 34);
        assert!(address.starts_with('T'));
    }

    #[test]
    fn structured_children_preserve_index_path_and_address() {
        let seed = [7u8; 32];
        let children = derive_child_accounts(&seed, 0, 3).unwrap();

        assert_eq!(children.len(), 3);
        assert_eq!(children[0].child_index, 0);
        assert_eq!(children[1].child_index, 1);
        assert_eq!(children[2].child_index, 2);
        assert_eq!(children[0].derivation_path, "m/44'/195'/0'/0/0");
        assert_eq!(children[1].derivation_path, "m/44'/195'/0'/0/1");
        assert_eq!(children[2].derivation_path, "m/44'/195'/0'/0/2");
        assert_ne!(children[0].address, children[1].address);
        assert_ne!(children[1].address, children[2].address);
    }

    #[test]
    fn explicit_paths_really_change_the_derived_address() {
        let seed = [7u8; 32];
        let first = derive_address_from_path(&seed, "m/44'/195'/0'/0/0").unwrap();
        let second = derive_address_from_path(&seed, "m/44'/195'/0'/0/1").unwrap();
        assert_ne!(first, second);
    }
}
