use crate::common::errors::EngineError;
use crate::adapters::orchestrator::{AdapterOrchestrator, AdapterTargetRequest, AdapterTargetResponse};
use crate::crypto::encryption::EncryptionService;
use crate::services::event_service::EventService;
use crate::services::wallet_create_service::secret_phrase::{SecretPhrasePayload, SecretPhraseService};
use crate::services::wallet::WalletRegistry;
use crate::types::chain::ChainRequest;
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct WalletInitialization { pub mnemonic: String, pub seed: Vec<u8> }

#[derive(Debug, Clone)]
pub struct WalletCreateResult { pub wallet: WalletResult, pub state: WalletInitialization }

#[derive(Debug, Clone, serde::Serialize)]
pub struct WalletResult {
    pub success: bool,
    /// Wallet-group ID (the container that owns the encrypted secret).
    pub wallet_id: String,
    /// First operational child created automatically with the group.
    pub child_id: String,
    pub mnemonic: String,
    pub encrypted: bool,
    pub addresses: Value,
    pub encrypted_mnemonic: Option<String>,
    /// Per-chain derivation failures. Successful chains remain in addresses.
    pub derivation_errors: Vec<Value>,
    pub partial: bool,
}

#[derive(Debug, Clone, Default)]
pub struct WalletCreateService;

impl WalletCreateService {
    pub fn create(word_count: usize, pin: Option<&str>) -> Result<WalletCreateResult, EngineError> { Self::create_with_chains(word_count, pin, &[]) }
    pub fn create_with_chains(word_count: usize, pin: Option<&str>, chains: &[ChainRequest]) -> Result<WalletCreateResult, EngineError> {
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
    pub fn import(mnemonic: &str, passphrase: Option<&str>, pin: Option<&str>) -> Result<WalletCreateResult, EngineError> { Self::import_with_chains(mnemonic, passphrase, pin, &[]) }
    pub fn import_with_chains(mnemonic: &str, passphrase: Option<&str>, pin: Option<&str>, chains: &[ChainRequest]) -> Result<WalletCreateResult, EngineError> {
        let payload = SecretPhraseService::import(mnemonic, passphrase)?;
        let wallet = Self::build_wallet(payload, pin, chains, None)?;
        EventService::emit(crate::events::event::WalletEvent::WalletImported { wallet_id: wallet.wallet.wallet_id.clone() })?;
        Ok(wallet)
    }
    pub fn initialize(mnemonic: &str, passphrase: Option<&str>) -> Result<WalletInitialization, EngineError> {
        let payload = SecretPhraseService::import(mnemonic, passphrase)?;
        Ok(WalletInitialization { mnemonic: payload.mnemonic, seed: payload.seed })
    }
    fn build_wallet(payload: SecretPhrasePayload, pin: Option<&str>, chains: &[ChainRequest], name: Option<&str>) -> Result<WalletCreateResult, EngineError> {
        let pin = pin.ok_or_else(|| EngineError::Validation("wallet PIN/password is required".into()))?;
        let adapter = AdapterOrchestrator::with_default_rpc()?;
        let mut addresses = std::collections::HashMap::new();
        let mut successful_chains = Vec::with_capacity(chains.len());
        let mut derivation_errors = Vec::new();

        // Stream progress as each adapter finishes so the UI does not need
        // to wait for every configured chain before showing completed addresses.
        EventService::publish("wallet.creation.started", json!({
            "total_chains": chains.len()
        }));
        for chain in chains {
            EventService::publish("wallet.creation.chain.started", json!({
                "chain_id": chain.chain_id,
                "chain_type": chain.chain_type,
                "chain_name": chain.name
            }));
            match adapter.adapter_target(AdapterTargetRequest::Derive {
                seed: &payload.seed,
                chains: std::slice::from_ref(chain),
            }) {
                Ok(AdapterTargetResponse::Addresses(values)) => {
                    if let Some((key, address)) = values.into_iter().next() {
                        addresses.insert(key, address.clone());
                        successful_chains.push(chain.clone());
                        EventService::publish("address.generated", json!({
                            "chain_id": chain.chain_id,
                            "chain_type": chain.chain_type,
                            "chain_name": chain.name,
                            "address": address
                        }));
                    } else {
                        let message = "adapter returned no derived address";
                        derivation_errors.push(json!({
                            "chain": chain.name,
                            "chain_id": chain.chain_id,
                            "chain_type": chain.chain_type,
                            "success": false,
                            "error": message
                        }));
                        EventService::publish("wallet.creation.chain.failed", json!({
                            "chain_id": chain.chain_id,
                            "chain_type": chain.chain_type,
                            "message": message
                        }));
                    }
                }
                Ok(_) => {
                    let message = "adapter returned an unexpected derivation response";
                    derivation_errors.push(json!({
                        "chain": chain.name,
                        "chain_id": chain.chain_id,
                        "chain_type": chain.chain_type,
                        "success": false,
                        "error": message
                    }));
                    EventService::publish("wallet.creation.chain.failed", json!({
                        "chain_id": chain.chain_id,
                        "chain_type": chain.chain_type,
                        "message": message
                    }));
                }
                Err(error) => {
                    let message = error.to_string();
                    derivation_errors.push(json!({
                        "chain": chain.name,
                        "chain_id": chain.chain_id,
                        "chain_type": chain.chain_type,
                        "success": false,
                        "error": message
                    }));
                    EventService::publish("wallet.creation.chain.failed", json!({
                        "chain_id": chain.chain_id,
                        "chain_type": chain.chain_type,
                        "message": message
                    }));
                }
            }
        }

        let encrypted_mnemonic = Some(EncryptionService::encrypt(&payload.mnemonic, pin)?);
        let addresses_value = json!(addresses);

        // Persist only chains whose addresses were actually derived. Failed
        // chains remain visible in the response through derivation_errors and
        // can be retried independently later.
        let record = WalletRegistry::register_named(
            payload.mnemonic.clone(),
            payload.seed.clone(),
            &successful_chains,
            &addresses_value,
            name,
            None,
            0,
            pin,
        )?;

        EventService::publish("wallet.creation.completed", json!({
            "address_count": addresses.len(),
            "failure_count": derivation_errors.len(),
            "partial": !derivation_errors.is_empty()
        }));

        Ok(WalletCreateResult {
            wallet: WalletResult {
                success: true,
                wallet_id: record.wallet_group_id.clone(),
                child_id: record.wallet_id.clone(),
                mnemonic: payload.mnemonic.clone(),
                encrypted: true,
                addresses: addresses_value,
                encrypted_mnemonic,
                partial: !derivation_errors.is_empty(),
                derivation_errors,
            },
            state: WalletInitialization { mnemonic: payload.mnemonic, seed: payload.seed },
        })
    }
}
