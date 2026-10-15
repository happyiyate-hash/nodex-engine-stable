use crate::common::errors::EngineError;
use crate::crypto::signing::sign_secp256k1_payload;

/// Signs the adapter's canonical transaction payload with a derived
/// secp256k1 private key. Transaction serialization remains chain-specific.
pub fn sign_payload(private_key: &[u8], payload: &[u8]) -> Result<Vec<u8>, EngineError> {
    sign_secp256k1_payload(private_key, payload)
}
