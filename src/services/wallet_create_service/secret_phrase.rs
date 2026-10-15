use crate::common::errors::EngineError;
use crate::crypto::mnemonic::{generate_mnemonic, validate_mnemonic};
use crate::crypto::seed::{derive_master_seed, Mnemonic};
use crate::services::address_derivation_service::AddressDerivationService;
use crate::types::chain::ChainRequest;
use crate::services::encryption_service::EncryptionService;
use crate::services::event_service::EventService;
use crate::services::wallet_registry::WalletRegistry;
use serde_json::{json, Value};

pub struct SecretPhraseService;

#[derive(Debug, Clone)]
pub struct SecretPhrasePayload {
    pub mnemonic: String,
    pub seed: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct WalletInitialization {
    pub mnemonic: String,
    pub seed: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct WalletCreateResult {
    pub wallet: WalletResult,
    pub state: WalletInitialization,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WalletResult {
    pub success: bool,
    pub wallet_id: String,
    /// The mnemonic is returned to the caller only so the wallet UI can show it
    /// to the user during creation/import. It must never be written to the
    /// persistent keystore in plaintext.
    pub mnemonic: String,
    pub encrypted: bool,
    pub addresses: Value,
    pub encrypted_mnemonic: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WalletCreateService;

impl WalletCreateService {
    pub fn create(word_count: usize, pin: Option<&str>) -> Result<WalletCreateResult, EngineError> {
        Self::create_with_chains(word_count, pin, &[])
    }

    pub fn create_with_chains(
        word_count: usize,
        pin: Option<&str>,
        chains: &[ChainRequest],
    ) -> Result<WalletCreateResult, EngineError> {
        let payload = SecretPhraseService::create(word_count)?;
        let wallet = Self::build_wallet(payload, pin, chains, None)?;
        EventService::emit(crate::events::event::WalletEvent::WalletCreated { wallet_id: wallet.wallet.wallet_id.clone() })?;
        Ok(wallet)
    }

    pub fn create_named(word_count: usize, pin: Option<&str>, chains: &[ChainRequest], name: Option<&str>) -> Result<WalletCreateResult, EngineError> {
        let payload = SecretPhraseService::create(word_count)?;
        let wallet = Self::build_wallet(payload, pin, chains, name)?;
        EventService::emit(crate::events::event::WalletEvent::WalletCreated { wallet_id: wallet.wallet.wallet_id.clone() })?;
        Ok(wallet)
    }

    pub fn import_named(mnemonic: &str, passphrase: Option<&str>, pin: Option<&str>, chains: &[ChainRequest], name: Option<&str>) -> Result<WalletCreateResult, EngineError> {
        let payload = SecretPhraseService::import(mnemonic, passphrase)?;
        let wallet = Self::build_wallet(payload, pin, chains, name)?;
        EventService::emit(crate::events::event::WalletEvent::WalletImported { wallet_id: wallet.wallet.wallet_id.clone() })?;
        Ok(wallet)
    }

    pub fn import(mnemonic: &str, passphrase: Option<&str>, pin: Option<&str>) -> Result<WalletCreateResult, EngineError> {
        Self::import_with_chains(mnemonic, passphrase, pin, &[])
    }

    pub fn import_with_chains(
        mnemonic: &str,
        passphrase: Option<&str>,
        pin: Option<&str>,
        chains: &[ChainRequest],
    ) -> Result<WalletCreateResult, EngineError> {
        let payload = SecretPhraseService::import(mnemonic, passphrase)?;
        let wallet = Self::build_wallet(payload, pin, chains, None)?;
        EventService::emit(crate::events::event::WalletEvent::WalletImported { wallet_id: wallet.wallet.wallet_id.clone() })?;
        Ok(wallet)
    }

    pub fn initialize(mnemonic: &str, passphrase: Option<&str>) -> Result<WalletInitialization, EngineError> {
        let payload = SecretPhraseService::import(mnemonic, passphrase)?;
        Ok(WalletInitialization {
            mnemonic: payload.mnemonic,
            seed: payload.seed,
        })
    }

    fn build_wallet(payload: SecretPhrasePayload, pin: Option<&str>, chains: &[ChainRequest], name: Option<&str>) -> Result<WalletCreateResult, EngineError> {
        let pin = pin.ok_or_else(|| EngineError::Validation("wallet PIN/password is required".into()))?;
        let addresses = AddressDerivationService::derive_all_addresses(&payload.seed, chains)?;
        let encrypted_mnemonic = Some(EncryptionService::encrypt(&payload.mnemonic, pin)?);
        let encrypted = true;

        let addresses_value = json!(addresses);
        let record = WalletRegistry::register_named(
            payload.mnemonic.clone(),
            payload.seed.clone(),
            chains,
            &addresses_value,
            name,
            None,
            0,
            pin,
        )?;

        let wallet = WalletResult {
            success: true,
            wallet_id: record.wallet_id,
            mnemonic: payload.mnemonic.clone(),
            encrypted,
            addresses: addresses_value,
            encrypted_mnemonic: encrypted_mnemonic.clone(),
        };

        Ok(WalletCreateResult {
            wallet,
            state: WalletInitialization {
                mnemonic: payload.mnemonic,
                seed: payload.seed,
            },
        })
    }
}

impl SecretPhraseService {
    pub fn create(word_count: usize) -> Result<SecretPhrasePayload, EngineError> {
        if !matches!(word_count, 12 | 15 | 18 | 21 | 24) {
            return Err(EngineError::Validation(
                "word_count must be one of 12, 15, 18, 21, or 24".into(),
            ));
        }

        let mnemonic = generate_mnemonic(word_count)?;
        validate_mnemonic(&mnemonic)?;
        let mnemonic = Mnemonic::new(&mnemonic)?;
        let seed = derive_master_seed(&mnemonic, None)?;

        Ok(SecretPhrasePayload {
            mnemonic: mnemonic.as_str().to_string(),
            seed,
        })
    }

    pub fn import(mnemonic: &str, passphrase: Option<&str>) -> Result<SecretPhrasePayload, EngineError> {
        validate_mnemonic(mnemonic)?;
        let mnemonic = Mnemonic::new(mnemonic)?;
        let seed = derive_master_seed(&mnemonic, passphrase)?;

        Ok(SecretPhrasePayload {
            mnemonic: mnemonic.as_str().to_string(),
            seed,
        })
    }

    pub fn validate(mnemonic: &str) -> Result<(), EngineError> {
        validate_mnemonic(mnemonic)
    }

    pub fn derive_seed(mnemonic: &str, passphrase: Option<&str>) -> Result<Vec<u8>, EngineError> {
        validate_mnemonic(mnemonic)?;
        let mnemonic = Mnemonic::new(mnemonic)?;
        derive_master_seed(&mnemonic, passphrase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_new_secret_phrase_and_seed() {
        let payload = SecretPhraseService::create(12).expect("create mnemonic should succeed");
        assert_eq!(payload.mnemonic.split_whitespace().count(), 12);
        assert_eq!(payload.seed.len(), 64);
    }

    #[test]
    fn rejects_invalid_word_count() {
        assert!(SecretPhraseService::create(13).is_err());
    }

    #[test]
    fn imports_existing_secret_phrase_and_derives_seed() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let payload = SecretPhraseService::import(phrase, None).expect("import should succeed");
        assert_eq!(payload.mnemonic, phrase);
        assert_eq!(payload.seed.len(), 64);
    }
}
