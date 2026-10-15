use crate::common::errors::EngineError;
use crate::crypto::derivation::DerivationPath;
use hmac::{Hmac, Mac};
use k256::{FieldBytes, Scalar, SecretKey};
use k256::elliptic_curve::{PrimeField, sec1::ToEncodedPoint};
use sha2::Sha512;

type HmacSha512 = Hmac<Sha512>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedKey {
    pub depth: u8,
    pub chain_code: [u8; 32],
    pub key: [u8; 32],
}

pub fn derive_master_key(seed: &[u8]) -> Result<ExtendedKey, EngineError> {
    let mut mac = HmacSha512::new_from_slice(b"Bitcoin seed")
        .map_err(|_| EngineError::Crypto("hmac initialization failed".into()))?;
    mac.update(seed);
    let result = mac.finalize().into_bytes();
    let mut key = [0u8; 32];
    let mut chain = [0u8; 32];
    key.copy_from_slice(&result[..32]);
    chain.copy_from_slice(&result[32..]);
    SecretKey::from_bytes((&key).into())
        .map_err(|_| EngineError::Crypto("invalid master secp256k1 key".into()))?;
    Ok(ExtendedKey { depth: 0, chain_code: chain, key })
}

pub fn derive_child_key(parent: &ExtendedKey, path: &DerivationPath) -> Result<ExtendedKey, EngineError> {
    let mut current = parent.clone();
    for segment in &path.segments {
        let mut mac = HmacSha512::new_from_slice(&current.chain_code)
            .map_err(|_| EngineError::Crypto("hmac initialization failed".into()))?;

        let mut input = Vec::with_capacity(37);
        if segment.is_hardened {
            input.push(0x00);
            input.extend_from_slice(&current.key);
        } else {
            let secret = SecretKey::from_bytes((&current.key).into())
                .map_err(|_| EngineError::Crypto("invalid parent secp256k1 key".into()))?;
            let public = secret.public_key();
            let encoded = public.to_encoded_point(true);
            input.extend_from_slice(encoded.as_bytes());
        }
        // BIP-32 encodes hardened child indices by setting the high bit.
        // DerivationPath stores the numeric index and hardened flag separately,
        // so the bit must be applied here before serializing the child number.
        let child_index = if segment.is_hardened {
            segment.index | 0x8000_0000
        } else {
            segment.index
        };
        input.extend_from_slice(&child_index.to_be_bytes());
        mac.update(&input);
        let result = mac.finalize().into_bytes();

        let il = Scalar::from_repr(*FieldBytes::from_slice(&result[..32]))
            .into_option()
            .ok_or_else(|| EngineError::Crypto("invalid BIP32 child scalar".into()))?;
        if bool::from(il.is_zero()) {
            return Err(EngineError::Crypto("invalid BIP32 child scalar".into()));
        }

        let parent_scalar = Scalar::from_repr(*FieldBytes::from_slice(&current.key))
            .into_option()
            .ok_or_else(|| EngineError::Crypto("invalid parent secp256k1 scalar".into()))?;
        let child_scalar = il + parent_scalar;
        if bool::from(child_scalar.is_zero()) {
            return Err(EngineError::Crypto("invalid BIP32 child key".into()));
        }

        let child_bytes = child_scalar.to_bytes();
        let mut child_key = [0u8; 32];
        child_key.copy_from_slice(&child_bytes);

        let mut child_chain = [0u8; 32];
        child_chain.copy_from_slice(&result[32..]);
        current = ExtendedKey {
            depth: current.depth.checked_add(1).ok_or_else(|| EngineError::Crypto("BIP32 depth overflow".into()))?,
            chain_code: child_chain,
            key: child_key,
        };
    }
    Ok(current)
}

pub fn derive_master_key_ed25519(seed: &[u8]) -> Result<ExtendedKey, EngineError> {
    let mut mac = HmacSha512::new_from_slice(b"ed25519 seed")
        .map_err(|_| EngineError::Crypto("hmac initialization failed".into()))?;
    mac.update(seed);
    let result = mac.finalize().into_bytes();
    let mut key = [0u8; 32];
    let mut chain = [0u8; 32];
    key.copy_from_slice(&result[..32]);
    chain.copy_from_slice(&result[32..]);
    Ok(ExtendedKey { depth: 0, chain_code: chain, key })
}

pub fn derive_child_key_ed25519(parent: &ExtendedKey, path: &DerivationPath) -> Result<ExtendedKey, EngineError> {
    for segment in &path.segments {
        if !segment.is_hardened {
            return Err(EngineError::Unsupported("non-hardened ed25519 derivation is unsupported".into()));
        }
    }
    let mut current = parent.clone();
    for segment in &path.segments {
        let mut mac = HmacSha512::new_from_slice(&current.chain_code)
            .map_err(|_| EngineError::Crypto("hmac initialization failed".into()))?;
        let mut input = Vec::with_capacity(37);
        input.push(0x00);
        input.extend_from_slice(&current.key);
        // SLIP-0010 also serializes hardened child indices with the high bit set.
        let child_index = segment.index | 0x8000_0000;
        input.extend_from_slice(&child_index.to_be_bytes());
        mac.update(&input);
        let result = mac.finalize().into_bytes();
        let mut child_key = [0u8; 32];
        let mut child_chain = [0u8; 32];
        child_key.copy_from_slice(&result[..32]);
        child_chain.copy_from_slice(&result[32..]);
        current = ExtendedKey { depth: current.depth + 1, chain_code: child_chain, key: child_key };
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::{derive_child_key, derive_master_key};
    use crate::crypto::derivation::DerivationPath;

    #[test]
    fn derives_master_key_from_seed() {
        let seed = [0x42u8; 64];
        let key = derive_master_key(&seed).expect("master key should derive");
        assert_eq!(key.depth, 0);
    }

    #[test]
    fn derives_child_key_from_path() {
        let seed = [0x7au8; 64];
        let master = derive_master_key(&seed).expect("master key should derive");
        let path = DerivationPath::parse("m/0'/1/2'".to_string()).expect("path should parse");
        let child = derive_child_key(&master, &path).expect("child key should derive");
        assert_eq!(child.depth, path.segments.len() as u8);
    }

    #[test]
    fn hardened_and_non_hardened_indices_are_not_interchangeable() {
        let seed = [0x7au8; 64];
        let master = derive_master_key(&seed).expect("master key should derive");
        let hardened = DerivationPath::parse("m/44'".to_string()).expect("path should parse");
        let non_hardened = DerivationPath::parse("m/44".to_string()).expect("path should parse");
        let hardened_child = derive_child_key(&master, &hardened).expect("hardened child should derive");
        let non_hardened_child = derive_child_key(&master, &non_hardened).expect("non-hardened child should derive");
        assert_ne!(hardened_child.key, non_hardened_child.key);
        assert_ne!(hardened_child.chain_code, non_hardened_child.chain_code);
    }
}
