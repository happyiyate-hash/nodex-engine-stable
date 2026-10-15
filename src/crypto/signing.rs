use crate::common::errors::EngineError;
use k256::ecdsa::{signature::Signer, Signature, SigningKey};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer as EdSigner};
use sha2::{Digest, Sha256};

/// Signs a payload using a secp256k1 private key.
/// The payload is hashed with SHA-256 before signing and the returned bytes
/// are the canonical fixed-width 64-byte ECDSA signature (r || s).
pub fn sign_secp256k1_payload(private_key: &[u8], payload: &[u8]) -> Result<Vec<u8>, EngineError> {
    if private_key.len() != 32 {
        return Err(EngineError::Validation("secp256k1 private key must be 32 bytes".into()));
    }

    let key_bytes: [u8; 32] = private_key.try_into()
        .map_err(|_| EngineError::Validation("invalid private key length".into()))?;
    let signing_key = SigningKey::from_bytes((&key_bytes).into())
        .map_err(|_| EngineError::Crypto("invalid secp256k1 private key".into()))?;

    let digest = Sha256::digest(payload);
    let signature: Signature = signing_key.sign(&digest);
    Ok(signature.to_bytes().to_vec())
}

pub fn sign_ed25519_payload(private_key: &[u8], payload: &[u8]) -> Result<Vec<u8>, EngineError> {
    if private_key.len() != 32 {
        return Err(EngineError::Validation("Ed25519 private key must be 32 bytes".into()));
    }
    let secret = SecretKey::from_bytes(private_key)
        .map_err(|_| EngineError::Crypto("invalid Ed25519 private key".into()))?;
    let public = PublicKey::from(&secret);
    let keypair = Keypair { secret, public };
    Ok(keypair.sign(payload).to_bytes().to_vec())
}

pub fn sign_sr25519_payload(private_key: &[u8], payload: &[u8]) -> Result<Vec<u8>, EngineError> {
    use schnorrkel::{ExpansionMode, MiniSecretKey, signing_context};
    if private_key.len() != 32 {
        return Err(EngineError::Validation("sr25519 mini-secret key must be 32 bytes".into()));
    }
    let mini = MiniSecretKey::from_bytes(private_key)
        .map_err(|_| EngineError::Crypto("invalid sr25519 private key".into()))?;
    let keypair = mini.expand_to_keypair(ExpansionMode::Ed25519);
    let ctx = signing_context(b"substrate");
    Ok(keypair.sign(ctx.bytes(payload)).to_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_payload_with_secp256k1() {
        let key = [7u8; 32];
        let sig = sign_secp256k1_payload(&key, b"nodex-test").expect("signature");
        assert_eq!(sig.len(), 64);
    }
}
