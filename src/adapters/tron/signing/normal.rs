use crate::common::errors::EngineError;
use crate::crypto::signing::sign_secp256k1_payload;

/// Generic secp256k1 payload signing used by the WalletAdapter contract.
pub fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
    sign_secp256k1_payload(private_key, tx_data)
}
