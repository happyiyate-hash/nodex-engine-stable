use crate::common::errors::EngineError;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use pbkdf2::pbkdf2_hmac;
use rand::{rngs::OsRng, RngCore};
use sha2::Sha256;

const VERSION: u8 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const ITERATIONS: u32 = 600_000;

pub struct EncryptionService;

impl EncryptionService {
    pub fn encrypt(plaintext: &str, password: &str) -> Result<String, EngineError> {
        Self::encrypt_bytes(plaintext.as_bytes(), password)
    }

    pub fn decrypt(payload: &str, password: &str) -> Result<String, EngineError> {
        let bytes = Self::decrypt_bytes(payload, password)?;
        String::from_utf8(bytes)
            .map_err(|_| EngineError::Crypto("decrypted data is not valid UTF-8".into()))
    }

    pub fn encrypt_bytes(plaintext: &[u8], password: &str) -> Result<String, EngineError> {
        Self::validate_password(password)?;
        let mut salt = [0u8; SALT_LEN];
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut salt);
        OsRng.fill_bytes(&mut nonce_bytes);

        let mut key_bytes = [0u8; KEY_LEN];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, ITERATIONS, &mut key_bytes);

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
            .map_err(|_| EngineError::Crypto("wallet encryption failed".into()))?;

        let mut encoded = Vec::with_capacity(1 + SALT_LEN + NONCE_LEN + ciphertext.len());
        encoded.push(VERSION);
        encoded.extend_from_slice(&salt);
        encoded.extend_from_slice(&nonce_bytes);
        encoded.extend_from_slice(&ciphertext);
        Ok(hex::encode(encoded))
    }

    pub fn decrypt_bytes(payload: &str, password: &str) -> Result<Vec<u8>, EngineError> {
        Self::validate_password(password)?;
        let encoded = hex::decode(payload)
            .map_err(|_| EngineError::Crypto("invalid encrypted wallet payload".into()))?;
        if encoded.len() < 1 + SALT_LEN + NONCE_LEN + 16 || encoded[0] != VERSION {
            return Err(EngineError::Crypto("invalid or unsupported encrypted wallet payload".into()));
        }

        let salt = &encoded[1..1 + SALT_LEN];
        let nonce = &encoded[1 + SALT_LEN..1 + SALT_LEN + NONCE_LEN];
        let ciphertext = &encoded[1 + SALT_LEN + NONCE_LEN..];

        let mut key_bytes = [0u8; KEY_LEN];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, ITERATIONS, &mut key_bytes);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
        cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| EngineError::Crypto("invalid password or corrupted encrypted wallet".into()))
    }

    fn validate_password(password: &str) -> Result<(), EngineError> {
        if password.trim().is_empty() {
            return Err(EngineError::Validation("wallet PIN/password cannot be empty".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::EncryptionService;

    #[test]
    fn round_trip() {
        let encrypted = EncryptionService::encrypt("secret phrase", "123456").unwrap();
        assert_ne!(encrypted, "secret phrase");
        assert_eq!(EncryptionService::decrypt(&encrypted, "123456").unwrap(), "secret phrase");
        assert!(EncryptionService::decrypt(&encrypted, "wrong").is_err());
    }

    #[test]
    fn byte_round_trip() {
        let data = vec![0, 1, 2, 255, 128];
        let encrypted = EncryptionService::encrypt_bytes(&data, "password").unwrap();
        assert_eq!(EncryptionService::decrypt_bytes(&encrypted, "password").unwrap(), data);
    }
}
