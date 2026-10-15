use crate::common::errors::EngineError;
use crate::services::encryption_service::EncryptionService;
use crate::types::chain::{ChainInfo, ChainRequest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalletType { Hd, PrivateKey }
impl Default for WalletType { fn default() -> Self { Self::Hd } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBlockchain {
    pub name: String,
    pub chain_id: String,
    pub symbol: Option<String>,
    pub decimals: u32,
    pub chain_type: String,
    pub derivation_path: String,
    pub address: String,
    #[serde(default)]
    pub rpc_provider: Option<String>,
    #[serde(default)]
    pub rpc_url: Option<String>,
    #[serde(default)]
    pub balance: Option<String>,
    #[serde(default)]
    pub tokens: Vec<WalletToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletToken {
    pub name: String,
    pub symbol: String,
    pub decimals: u32,
    pub contract_address: String,
    #[serde(default)]
    pub balance: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletRecord {
    pub wallet_id: String,
    #[serde(default)]
    pub wallet_group_id: String,
    pub name: String,
    /// Parent wallet ID for derived child wallets. Root wallets have no parent.
    #[serde(default)]
    pub parent_wallet_id: Option<String>,
    /// Child wallet IDs owned by this wallet. The IDs are metadata only; child
    /// secrets are never stored separately from the wallet's secret group.
    #[serde(default)]
    pub child_wallet_ids: Vec<String>,
    pub secret_group_id: String,
    pub child_index: u32,
    pub blockchains: Vec<WalletBlockchain>,
    pub active: bool,
    #[serde(default)]
    pub wallet_type: WalletType,
}

#[derive(Debug, Clone, Serialize)]
pub struct WalletChoice {
    pub wallet_id: String,
    pub name: String,
    pub secret_group_id: String,
    pub child_index: u32,
    pub matching_blockchains: Vec<WalletBlockchain>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WalletGroup {
    pub wallet_id: String,
    pub name: String,
    pub secret_group_id: String,
    pub wallet_count: usize,
    pub active: bool,
    pub active_child_id: Option<String>,
    pub child_wallet_ids: Vec<String>,
    pub wallets: Vec<WalletRecord>,
}

#[derive(Debug, Clone)]
struct StoredWallet {
    record: WalletRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSecret {
    encrypted_mnemonic: String,
    encrypted_seed: String,
    encrypted_private_key: Option<String>,
}

#[derive(Default)]
struct RegistryState {
    wallets: HashMap<String, StoredWallet>,
    secrets: HashMap<String, StoredSecret>,
    unlocked_secrets: HashMap<String, (String, Vec<u8>)>,
    active_wallet_id: Option<String>,
    active_child_id: Option<String>,
    next_wallet_id: u64,
}

/// Portable, versioned persistence payload. Secret material remains encrypted
/// with the user's PIN/password; unlocked mnemonic/seed bytes are never exported.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletRegistrySnapshot {
    pub version: u32,
    wallets: Vec<WalletRecord>,
    secrets: HashMap<String, StoredSecret>,
    active_wallet_id: Option<String>,
    active_child_id: Option<String>,
    next_wallet_id: u64,
}

static REGISTRY: OnceLock<Mutex<RegistryState>> = OnceLock::new();

fn registry() -> &'static Mutex<RegistryState> {
    REGISTRY.get_or_init(|| Mutex::new(RegistryState::default()))
}

pub struct WalletRegistry;

fn default_decimals(chain_type: &str, symbol: Option<&str>) -> u32 {
    match symbol.unwrap_or_default().to_ascii_uppercase().as_str() {
        "BTC" | "BCH" | "LTC" | "DOGE" => 8,
        "SOL" => 9,
        "XRP" => 6,
        "ADA" => 6,
        "ALGO" => 6,
        "TRX" => 6,
        "HBAR" => 8,
        "XTZ" => 6,
        "SUI" => 9,
        "APT" => 8,
        _ => match chain_type.to_ascii_lowercase().as_str() { "utxo" | "btc" => 8, "xrp" | "tron" => 6, "solana" | "sui" => 9, _ => 18 }
    }
}

impl WalletRegistry {
    /// Export wallet records and encrypted secrets for storage by the host app
    /// (for example, encrypted application storage or browser localStorage).
    /// No unlocked secret material is included in this snapshot.
    pub fn export_snapshot() -> Result<WalletRegistrySnapshot, EngineError> {
        let state = registry().lock()
            .map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        Ok(WalletRegistrySnapshot {
            version: 1,
            wallets: state.wallets.values().map(|w| w.record.clone()).collect(),
            secrets: state.secrets.clone(),
            active_wallet_id: state.active_wallet_id.clone(),
            active_child_id: state.active_child_id.clone(),
            next_wallet_id: state.next_wallet_id,
        })
    }

    /// Restore a previously exported snapshot. This only restores encrypted
    /// records; users must unlock with their PIN/password before signing.
    pub fn restore_snapshot(snapshot: WalletRegistrySnapshot) -> Result<(), EngineError> {
        if snapshot.version != 1 {
            return Err(EngineError::Validation(format!(
                "unsupported wallet registry snapshot version: {}", snapshot.version
            )));
        }

        let mut wallets = HashMap::new();
        for mut record in snapshot.wallets {
            if record.wallet_id.trim().is_empty() || record.secret_group_id.trim().is_empty() {
                return Err(EngineError::Validation("snapshot contains a wallet with an empty ID".into()));
            }
            let secret = snapshot.secrets.get(&record.secret_group_id)
                .ok_or_else(|| EngineError::Validation(format!(
                    "snapshot is missing encrypted secrets for wallet group {}", record.secret_group_id
                )))?;
            match record.wallet_type {
                WalletType::Hd if secret.encrypted_mnemonic.is_empty() || secret.encrypted_seed.is_empty() => {
                    return Err(EngineError::Validation(format!(
                        "snapshot has incomplete HD secret material for wallet group {}", record.secret_group_id
                    )));
                }
                WalletType::PrivateKey if secret.encrypted_private_key.as_deref().unwrap_or("").is_empty() => {
                    return Err(EngineError::Validation(format!(
                        "snapshot has no encrypted private key for wallet group {}", record.secret_group_id
                    )));
                }
                _ => {}
            }
            if wallets.contains_key(&record.wallet_id) {
                return Err(EngineError::Validation(format!(
                    "snapshot contains duplicate wallet ID {}", record.wallet_id
                )));
            }
            record.active = false;
            wallets.insert(record.wallet_id.clone(), StoredWallet { record });
        }

        for (group_id, secret) in &snapshot.secrets {
            let used = wallets.values().any(|w| w.record.secret_group_id == *group_id);
            if !used {
                return Err(EngineError::Validation(format!(
                    "snapshot contains orphaned secret group {}", group_id
                )));
            }
            if secret.encrypted_mnemonic.is_empty() && secret.encrypted_seed.is_empty()
                && secret.encrypted_private_key.is_none() {
                return Err(EngineError::Validation(format!(
                    "snapshot contains empty encrypted secret material for {}", group_id
                )));
            }
        }

        let requested_active_group = snapshot.active_wallet_id.clone();
        let mut active_child_id = snapshot.active_child_id.filter(|id| wallets.contains_key(id));
        if active_child_id.is_none() {
            if let Some(group_id) = requested_active_group.as_deref() {
                active_child_id = wallets.values()
                    .find(|w| w.record.wallet_group_id == group_id && w.record.child_index == 0)
                    .map(|w| w.record.wallet_id.clone());
            }
        }
        let active_wallet_id = active_child_id.as_ref()
            .and_then(|id| wallets.get(id).map(|w| w.record.wallet_group_id.clone()))
            .or(requested_active_group.filter(|group| wallets.values().any(|w| w.record.wallet_group_id == *group)));
        if let Some(id) = active_child_id.as_ref() {
            if let Some(wallet) = wallets.get_mut(id) {
                wallet.record.active = true;
            }
        }

        let mut state = registry().lock()
            .map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        state.wallets = wallets;
        state.secrets = snapshot.secrets;
        state.unlocked_secrets.clear();
        state.active_child_id = active_child_id;
        state.active_wallet_id = active_wallet_id;
        state.next_wallet_id = snapshot.next_wallet_id.max(
            state.wallets.values().filter_map(|w| {
                w.record.wallet_group_id.strip_prefix("wallet_")?.parse::<u64>().ok()
            }).max().unwrap_or(0)
        );
        Ok(())
    }
    pub fn register(
        mnemonic: String,
        seed: Vec<u8>,
        chains: &[ChainRequest],
        addresses: &serde_json::Value,
    ) -> Result<WalletRecord, EngineError> {
        Self::register_named(mnemonic, seed, chains, addresses, None, None, 0, "")
    }

    pub fn register_named(
        mnemonic: String,
        seed: Vec<u8>,
        chains: &[ChainRequest],
        addresses: &serde_json::Value,
        name: Option<&str>,
        secret_group_id: Option<&str>,
        child_index: u32,
        password: &str,
    ) -> Result<WalletRecord, EngineError> {
        if password.trim().is_empty() { return Err(EngineError::Validation("wallet PIN/password cannot be empty".into())); }
        let encrypted_mnemonic = EncryptionService::encrypt(&mnemonic, password)?;
        let encrypted_seed = EncryptionService::encrypt_bytes(&seed, password)?;
        let mut state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        state.next_wallet_id += 1;
        let wallet_group_id = secret_group_id.map(str::to_string).unwrap_or_else(|| format!("wallet_{}", state.next_wallet_id));
        let wallet_id = format!("{}_child_0", wallet_group_id);
        let group_id = wallet_group_id.clone();
        let wallet_name = format!("Wallet {}", state.next_wallet_id);
        let blockchains = Self::build_blockchains(chains, addresses)?;
        let record = WalletRecord {
            wallet_id: wallet_id.clone(),
            wallet_group_id: wallet_group_id.clone(),
            name: name.map(str::to_owned).unwrap_or(wallet_name),
            parent_wallet_id: None,
            child_wallet_ids: Vec::new(),
            secret_group_id: group_id.clone(),
            child_index,
            blockchains,
            active: true,
            wallet_type: WalletType::Hd,
        };
        if let Some(old_id) = state.active_child_id.clone() {
            if let Some(old) = state.wallets.get_mut(&old_id) {
                old.record.active = false;
            }
        }
        state.secrets.entry(group_id).or_insert(StoredSecret {
            encrypted_mnemonic,
            encrypted_seed,
            encrypted_private_key: None,
        });
        state.wallets.insert(wallet_id.clone(), StoredWallet { record: record.clone() });
        state.unlocked_secrets.clear();
        state.unlocked_secrets.insert(wallet_id.clone(), (mnemonic, seed));
        state.active_wallet_id = Some(wallet_group_id);
        state.active_child_id = Some(wallet_id);
        Ok(record)
    }

    pub fn create_child(
        parent_wallet_id: &str,
        name: Option<&str>,
        chains: &[ChainRequest],
        password: &str,
    ) -> Result<WalletRecord, EngineError> {
        Self::create_child_at_index(parent_wallet_id, None, name, chains, password)
    }

    /// Create a child at an exact index when requested, without materializing
    /// intermediate indices. With no index, create the next index after the
    /// highest child already registered in this secret group.
    pub fn create_child_at_index(
        parent_wallet_id: &str,
        requested_index: Option<u32>,
        name: Option<&str>,
        chains: &[ChainRequest],
        password: &str,
    ) -> Result<WalletRecord, EngineError> {
        Self::create_child_at_index_with_session(parent_wallet_id, requested_index, name, chains, Some(password))
    }

    /// Create a child using an already-authorized runtime secret when available.
    /// A PIN/password is only required when the group is currently locked.
    pub fn create_child_at_index_with_session(
        parent_wallet_id: &str,
        requested_index: Option<u32>,
        name: Option<&str>,
        chains: &[ChainRequest],
        password: Option<&str>,
    ) -> Result<WalletRecord, EngineError> {
        let mut state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let parent = state.wallets.get(parent_wallet_id)
            .or_else(|| state.wallets.values().find(|w| w.record.secret_group_id == parent_wallet_id && w.record.child_index == 0))
            .ok_or_else(|| EngineError::Validation(format!("wallet/group not found: {}", parent_wallet_id)))?
            .clone();
        if parent.record.wallet_type != WalletType::Hd {
            return Err(EngineError::Validation("standalone private-key wallets cannot create child wallets".into()));
        }
        let (parent_mnemonic, parent_seed) = if let Some(password) = password {
            let parent_secret = state.secrets.get(&parent.record.secret_group_id)
                .ok_or_else(|| EngineError::Unexpected(format!("wallet secret group missing: {}", parent.record.secret_group_id)))?;
            (
                EncryptionService::decrypt(&parent_secret.encrypted_mnemonic, password)?,
                EncryptionService::decrypt_bytes(&parent_secret.encrypted_seed, password)?,
            )
        } else {
            state.unlocked_secrets.iter()
                .find(|(child_id, _)| state.wallets.get(*child_id)
                    .map(|w| w.record.secret_group_id == parent.record.secret_group_id)
                    .unwrap_or(false))
                .map(|(_, (mnemonic, seed))| (mnemonic.clone(), seed.clone()))
                .ok_or_else(|| EngineError::Validation(
                    "wallet is locked; provide pin/password to authorize child derivation".into()
                ))?
        };
        let next_index = requested_index.unwrap_or_else(|| {
            state.wallets.values()
                .filter(|w| w.record.secret_group_id == parent.record.secret_group_id)
                .map(|w| w.record.child_index)
                .max()
                .unwrap_or(parent.record.child_index)
                .saturating_add(1)
        });
        let wallet_id = format!("{}_child_{}", parent.record.secret_group_id, next_index);
        if state.wallets.contains_key(&wallet_id) {
            return Err(EngineError::Validation(format!(
                "child index {} already exists in wallet group {}",
                next_index, parent.record.secret_group_id
            )));
        }
        let wallet_name = name.map(str::to_owned).unwrap_or_else(|| format!("Child {}", next_index));

        // When chains are omitted, reuse the parent's configured chains but
        // derive each one at the requested child index, not at its stored path.
        let source_chains: Vec<ChainRequest> = if chains.is_empty() {
            parent.record.blockchains.iter().map(|b| ChainRequest {
                name: b.name.clone(),
                chain_id: b.chain_id.clone(),
                symbol: b.symbol.clone(),
                chain_type: b.chain_type.clone(),
                derivation_path: None,
                rpc_provider: b.rpc_provider.clone(),
                rpc_url: b.rpc_url.clone(),
                network: None,
                p2pkh_version: None,
                p2sh_version: None,
            bech32_hrp: None,
                address_type: None,
            }).collect()
        } else {
            chains.to_vec()
        };
        let mut child_chains = Vec::with_capacity(source_chains.len());
        let mut addresses = serde_json::Map::new();
        for chain in &source_chains {
            let mut child = chain.clone();
            if let Some(explicit_path) = child.derivation_path.as_deref() {
                // A supplied child_index wins over catalog/default path endings.
                // If no index was supplied, the path index has already selected
                // next_index and is preserved as-is.
                if requested_index.is_some() {
                    child.derivation_path = Some(Self::path_for_child_index(explicit_path, next_index)?);
                } else {
                    let path_index = Self::child_index_from_path(explicit_path)?;
                    if path_index != next_index {
                        return Err(EngineError::Validation(format!(
                            "derivation_path {} targets child index {}, but selected child index is {}",
                            explicit_path, path_index, next_index
                        )));
                    }
                }
            } else {
                let base_path = child.path()?;
                child.derivation_path = Some(Self::path_for_child_index(&base_path, next_index)?);
            }
            let address = crate::services::address_service::AddressService::derive_address(
                &parent_seed, &child
            )?;
            let info = ChainInfo::from_request(&child)?;
            child_chains.push(child);
            addresses.insert(info.name, serde_json::Value::String(address));
        }

        let blockchains = Self::build_blockchains(&child_chains, &serde_json::Value::Object(addresses))?;
        let root_wallet_id = state.wallets.values()
            .find(|w| w.record.secret_group_id == parent.record.secret_group_id && w.record.child_index == 0)
            .map(|w| w.record.wallet_id.clone())
            .unwrap_or_else(|| parent.record.wallet_id.clone());
        let record = WalletRecord {
            wallet_id: wallet_id.clone(),
            wallet_group_id: parent.record.secret_group_id.clone(),
            name: wallet_name,
            parent_wallet_id: Some(root_wallet_id.clone()),
            child_wallet_ids: Vec::new(),
            secret_group_id: parent.record.secret_group_id.clone(),
            child_index: next_index,
            blockchains,
            active: true,
            wallet_type: WalletType::Hd,
        };
        if let Some(old_id) = state.active_child_id.clone() {
            if let Some(old) = state.wallets.get_mut(&old_id) {
                old.record.active = false;
            }
        }
        state.wallets.insert(wallet_id.clone(), StoredWallet { record: record.clone() });
        state.unlocked_secrets.clear();
        state.unlocked_secrets.insert(wallet_id.clone(), (parent_mnemonic, parent_seed));
        if let Some(root_stored) = state.wallets.get_mut(&root_wallet_id) {
            if !root_stored.record.child_wallet_ids.contains(&wallet_id) {
                root_stored.record.child_wallet_ids.push(wallet_id.clone());
            }
        }
        state.active_wallet_id = Some(parent.record.secret_group_id.clone());
        state.active_child_id = Some(wallet_id);
        Ok(record)
    }

    pub fn child_index_from_path(path: &str) -> Result<u32, EngineError> {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() < 2 || parts[0] != "m" {
            return Err(EngineError::Validation(format!("unsupported child derivation path: {}", path)));
        }
        parts.last()
            .and_then(|part| part.trim_end_matches('\'').parse::<u32>().ok())
            .ok_or_else(|| EngineError::Validation(format!("invalid child index in derivation path: {}", path)))
    }

    pub fn wallet_count(secret_group_id: &str) -> Result<usize, EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        Ok(state.wallets.values().filter(|w| w.record.secret_group_id == secret_group_id).count())
    }

    pub fn list_group(secret_group_id: &str) -> Result<WalletGroup, EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let mut wallets: Vec<WalletRecord> = state.wallets.values()
            .filter(|w| w.record.secret_group_id == secret_group_id)
            .map(|w| w.record.clone())
            .collect();
        wallets.sort_by_key(|w| w.child_index);
        if wallets.is_empty() {
            return Err(EngineError::Validation(format!("wallet group not found: {}", secret_group_id)));
        }
        let wallet_id = wallets.first().map(|w| w.wallet_group_id.clone()).unwrap_or_else(|| secret_group_id.to_string());
        let active = state.active_wallet_id.as_deref() == Some(wallet_id.as_str());
        Ok(WalletGroup {
            wallet_id,
            name: wallets.first().map(|w| w.name.clone()).unwrap_or_default(),
            secret_group_id: secret_group_id.to_string(),
            wallet_count: wallets.len(),
            active,
            active_child_id: if active { state.active_child_id.clone() } else { None },
            child_wallet_ids: wallets.iter().map(|w| w.wallet_id.clone()).collect(),
            wallets,
        })
    }

    pub fn groups() -> Result<Vec<WalletGroup>, EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let mut grouped: HashMap<String, Vec<WalletRecord>> = HashMap::new();
        for wallet in state.wallets.values() {
            grouped.entry(wallet.record.secret_group_id.clone()).or_default().push(wallet.record.clone());
        }
        let mut groups: Vec<WalletGroup> = grouped.into_iter().map(|(secret_group_id, mut wallets)| {
            wallets.sort_by_key(|w| w.child_index);
            let wallet_id = wallets.first().map(|w| w.wallet_group_id.clone()).unwrap_or_else(|| secret_group_id.clone());
            let active = state.active_wallet_id.as_deref() == Some(wallet_id.as_str());
            let child_wallet_ids = wallets.iter().map(|w| w.wallet_id.clone()).collect();
            WalletGroup { wallet_id, name: wallets.first().map(|w| w.name.clone()).unwrap_or_default(), secret_group_id, wallet_count: wallets.len(), active, active_child_id: if active { state.active_child_id.clone() } else { None }, child_wallet_ids, wallets }
        }).collect();
        groups.sort_by(|a, b| a.secret_group_id.cmp(&b.secret_group_id));
        Ok(groups)
    }

    pub fn wallets_for_group(secret_group_id: &str) -> Result<Vec<WalletRecord>, EngineError> {
        Ok(Self::list_group(secret_group_id)?.wallets)
    }

    pub fn wallet_secret(wallet_id: &str, password: &str) -> Result<(String, Vec<u8>), EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let wallet = state.wallets.get(wallet_id).ok_or_else(|| EngineError::Validation(format!("wallet not found: {}", wallet_id)))?;
        if wallet.record.wallet_type != WalletType::Hd { return Err(EngineError::Validation("standalone private-key wallets do not expose a mnemonic seed".into())); }
        let secret = state.secrets.get(&wallet.record.secret_group_id)
            .ok_or_else(|| EngineError::Unexpected(format!("wallet secret group missing: {}", wallet.record.secret_group_id)))?;
        Ok((EncryptionService::decrypt(&secret.encrypted_mnemonic, password)?, EncryptionService::decrypt_bytes(&secret.encrypted_seed, password)?))
    }

    pub fn unlocked_wallet_secret(wallet_id: &str) -> Result<(String, Vec<u8>), EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        state.unlocked_secrets.get(wallet_id).cloned()
            .ok_or_else(|| EngineError::Validation(format!("wallet is locked or not unlocked in this session: {}", wallet_id)))
    }

    pub fn wallet_private_key(wallet_id: &str, password: &str) -> Result<Vec<u8>, EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let wallet = state.wallets.get(wallet_id).ok_or_else(|| EngineError::Validation(format!("wallet not found: {}", wallet_id)))?;
        let secret = state.secrets.get(&wallet.record.secret_group_id)
            .ok_or_else(|| EngineError::Unexpected(format!("wallet secret group missing: {}", wallet.record.secret_group_id)))?;
        let encrypted = secret.encrypted_private_key.clone().ok_or_else(|| EngineError::Validation("wallet does not contain an imported private key".into()))?;
        EncryptionService::decrypt_bytes(&encrypted, password)
    }

    pub fn register_private_key(name: Option<&str>, chain: ChainRequest, private_key: Vec<u8>, address: String, password: &str) -> Result<WalletRecord, EngineError> {
        let encrypted_private_key = EncryptionService::encrypt_bytes(&private_key, password)?;
        let mut state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        state.next_wallet_id += 1;
        let wallet_group_id = format!("wallet_{}", state.next_wallet_id);
        let wallet_id = format!("{}_child_0", wallet_group_id);
        let info = ChainInfo::from_request(&chain)?;
        let secret_group_id = wallet_group_id.clone();
        let record = WalletRecord { wallet_id: wallet_id.clone(), wallet_group_id: wallet_group_id.clone(), name: name.map(str::to_owned).unwrap_or_else(|| format!("Wallet {}", state.next_wallet_id)), parent_wallet_id: None, child_wallet_ids: Vec::new(), secret_group_id: secret_group_id.clone(), child_index: 0, blockchains: vec![WalletBlockchain { name: info.name, chain_id: chain.chain_id.clone(), symbol: chain.symbol.clone(), chain_type: info.chain_type, derivation_path: info.derivation_path, address, rpc_provider: chain.rpc_provider.clone(), rpc_url: chain.rpc_url.clone(), decimals: default_decimals(&chain.chain_type, chain.symbol.as_deref()), balance: None, tokens: Vec::new() }], active: true, wallet_type: WalletType::PrivateKey };
        if let Some(old_id) = state.active_child_id.clone() { if let Some(old) = state.wallets.get_mut(&old_id) { old.record.active = false; } }
        state.secrets.insert(secret_group_id, StoredSecret {
            encrypted_mnemonic: String::new(),
            encrypted_seed: String::new(),
            encrypted_private_key: Some(encrypted_private_key),
        });
        state.wallets.insert(wallet_id.clone(), StoredWallet { record: record.clone() });
        state.active_wallet_id = Some(record.wallet_group_id.clone());
        state.active_child_id = Some(wallet_id);
        Ok(record)
    }

    pub fn add_blockchain(
        wallet_id: &str,
        chain: ChainRequest,
        address: String,
    ) -> Result<WalletRecord, EngineError> {
        let mut state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let wallet = state.wallets.get_mut(wallet_id)
            .ok_or_else(|| EngineError::Validation(format!("wallet not found: {}", wallet_id)))?;
        let info = ChainInfo::from_request(&chain)?;
        if wallet.record.wallet_type == WalletType::PrivateKey && !chain.chain_type.eq_ignore_ascii_case("evm") {
            return Err(EngineError::Validation("private-key wallets can only add EVM blockchains".into()));
        }
        if wallet.record.blockchains.iter().any(|b| b.name.eq_ignore_ascii_case(&info.name)) {
            return Err(EngineError::Validation(format!("blockchain already registered: {}", info.name)));
        }
        wallet.record.blockchains.push(WalletBlockchain {
            name: info.name,
            chain_id: chain.chain_id.clone(),
            symbol: chain.symbol.clone(),
            decimals: default_decimals(&chain.chain_type, chain.symbol.as_deref()),
            chain_type: info.chain_type,
            derivation_path: info.derivation_path,
            address,
            rpc_provider: chain.rpc_provider.clone(),
            rpc_url: chain.rpc_url.clone(),
            balance: None,
            tokens: Vec::new(),
        });
        Ok(wallet.record.clone())
    }

    /// Attach token contract metadata to one specific child wallet and network.
    /// Re-adding the same contract on the same chain updates its metadata in place.
    pub fn add_token(
        wallet_id: &str,
        chain_id: &str,
        token: WalletToken,
    ) -> Result<WalletRecord, EngineError> {
        if token.contract_address.trim().is_empty() {
            return Err(EngineError::Validation("token contract_address is required".into()));
        }
        if token.symbol.trim().is_empty() {
            return Err(EngineError::Validation("token symbol is required".into()));
        }
        let mut state = registry().lock()
            .map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let wallet = state.wallets.get_mut(wallet_id)
            .ok_or_else(|| EngineError::Validation(format!("wallet not found: {}", wallet_id)))?;
        let chain = wallet.record.blockchains.iter_mut()
            .find(|b| b.chain_id.eq_ignore_ascii_case(chain_id))
            .ok_or_else(|| EngineError::Validation(format!(
                "chain '{}' is not registered on child wallet '{}'", chain_id, wallet_id
            )))?;
        if let Some(existing) = chain.tokens.iter_mut()
            .find(|t| t.contract_address.eq_ignore_ascii_case(&token.contract_address)) {
            *existing = token;
        } else {
            chain.tokens.push(token);
        }
        Ok(wallet.record.clone())
    }

    pub fn update_balances(
        balances: &[crate::services::balance_service::ChainBalance],
    ) -> Result<Vec<WalletRecord>, EngineError> {
        let mut state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let mut updated = Vec::new();

        for balance in balances {
            let Some(wallet) = state.wallets.get_mut(&balance.wallet_id) else {
                continue;
            };

            let Some(blockchain) = wallet.record.blockchains.iter_mut().find(|chain| {
                chain.chain_id == balance.chain_id
                    && chain.chain_type.eq_ignore_ascii_case(&balance.chain_type)
                    && chain.address == balance.address
            }) else {
                continue;
            };

            blockchain.balance = balance.native.as_ref().map(|asset| asset.balance.clone());
            blockchain.tokens = balance.tokens.iter().map(|token| WalletToken {
                name: token.name.clone().unwrap_or_else(|| token.symbol.clone().unwrap_or_default()),
                symbol: token.symbol.clone().unwrap_or_default(),
                decimals: token.decimals.unwrap_or(0),
                contract_address: token.address.clone(),
                balance: Some(token.balance.clone()),
            }).collect();

            updated.push(wallet.record.clone());
        }

        Ok(updated)
    }

    pub fn switch(wallet_id: &str, password: &str) -> Result<(String, Vec<u8>, Option<Vec<u8>>), EngineError> {
        let mut state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let child_id = state.wallets.get(wallet_id).map(|w| w.record.wallet_id.clone())
            .or_else(|| state.wallets.values().find(|w| w.record.secret_group_id == wallet_id && w.record.child_index == 0).map(|w| w.record.wallet_id.clone()))
            .ok_or_else(|| EngineError::Validation(format!("wallet/group not found: {}", wallet_id)))?;
        let stored = state.wallets.get(&child_id).ok_or_else(|| EngineError::Validation(format!("child wallet not found: {}", child_id)))?.clone();

        let secret = state.secrets.get(&stored.record.secret_group_id)
            .ok_or_else(|| EngineError::Unexpected(format!("wallet secret group missing: {}", stored.record.secret_group_id)))?;
        let (mnemonic, seed, private_key) = if stored.record.wallet_type == WalletType::Hd {
            let mnemonic = EncryptionService::decrypt(&secret.encrypted_mnemonic, password)?;
            let seed = EncryptionService::decrypt_bytes(&secret.encrypted_seed, password)?;
            (mnemonic, seed, None)
        } else {
            let private_key = secret.encrypted_private_key.clone()
                .ok_or_else(|| EngineError::Validation("wallet does not contain an imported private key".into()))
                .and_then(|value| EncryptionService::decrypt_bytes(&value, password))?;
            (String::new(), Vec::new(), Some(private_key))
        };

        for wallet in state.wallets.values_mut() {
            wallet.record.active = false;
        }
        if let Some(wallet) = state.wallets.get_mut(&child_id) {
            wallet.record.active = true;
        }
        state.active_wallet_id = Some(stored.record.wallet_group_id.clone());
        state.active_child_id = Some(child_id.clone());
        state.unlocked_secrets.clear();
        if stored.record.wallet_type == WalletType::Hd {
            state.unlocked_secrets.insert(child_id.clone(), (mnemonic.clone(), seed.clone()));
        }
        Ok((mnemonic, seed, private_key))
    }

    pub fn restore_persisted(
        record: WalletRecord,
        mnemonic: Option<String>,
        seed: Option<Vec<u8>>,
        private_key: Option<Vec<u8>>,
        password: &str,
        activate: bool,
    ) -> Result<WalletRecord, EngineError> {
        if password.trim().is_empty() {
            return Err(EngineError::Validation("wallet PIN/password cannot be empty".into()));
        }
        let mut state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let (encrypted_mnemonic, encrypted_seed, encrypted_private_key) = match record.wallet_type {
            WalletType::Hd => {
                let mnemonic = mnemonic.as_ref().ok_or_else(|| EngineError::Validation("missing mnemonic for HD wallet restore".into()))?;
                let seed = seed.as_ref().ok_or_else(|| EngineError::Validation("missing seed for HD wallet restore".into()))?;
                (EncryptionService::encrypt(mnemonic, password)?, EncryptionService::encrypt_bytes(seed, password)?, None)
            }
            WalletType::PrivateKey => {
                let key = private_key.ok_or_else(|| EngineError::Validation("missing private key for private-key wallet restore".into()))?;
                (String::new(), String::new(), Some(EncryptionService::encrypt_bytes(&key, password)?))
            }
        };

        if activate {
            for wallet in state.wallets.values_mut() {
                wallet.record.active = false;
            }
        }

        let wallet_id = record.wallet_id.clone();
        let wallet_type = record.wallet_type;
        let numeric_id = wallet_id.strip_prefix("wallet_").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        state.next_wallet_id = state.next_wallet_id.max(numeric_id);
        let mut stored_record = record;
        if stored_record.wallet_group_id.trim().is_empty() {
            stored_record.wallet_group_id = stored_record.secret_group_id.clone();
        }
        stored_record.active = activate;
        state.secrets.insert(stored_record.secret_group_id.clone(), StoredSecret {
            encrypted_mnemonic,
            encrypted_seed,
            encrypted_private_key,
        });
        state.wallets.insert(wallet_id.clone(), StoredWallet {
            record: stored_record.clone(),
        });
        if activate {
            state.active_wallet_id = Some(stored_record.wallet_group_id.clone());
            state.active_child_id = Some(wallet_id.clone());
        }
        if wallet_type == WalletType::Hd {
            if let (Some(mnemonic), Some(seed)) = (mnemonic, seed) {
                state.unlocked_secrets.insert(wallet_id.clone(), (mnemonic, seed));
            }
        } else {
            state.unlocked_secrets.remove(&wallet_id);
        }
        Ok(stored_record)
    }

    pub fn delete(wallet_id: &str) -> Result<(), EngineError> {
        let mut state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        state.unlocked_secrets.remove(wallet_id);
        let removed = state.wallets.remove(wallet_id)
            .ok_or_else(|| EngineError::Validation(format!("wallet not found: {}", wallet_id)))?;
        if let Some(parent_id) = removed.record.parent_wallet_id.as_deref() {
            if let Some(parent) = state.wallets.get_mut(parent_id) {
                parent.record.child_wallet_ids.retain(|id| id != wallet_id);
            }
        }
        let group_still_used = state.wallets.values().any(|w| w.record.secret_group_id == removed.record.secret_group_id);
        if !group_still_used {
            state.secrets.remove(&removed.record.secret_group_id);
        }
        if state.active_child_id.as_deref() == Some(wallet_id) {
            state.active_wallet_id = None;
            state.active_child_id = None;
            if let Some(id) = state.wallets.keys().next().cloned() {
                if let Some(wallet) = state.wallets.get_mut(&id) {
                    wallet.record.active = true;
                    let group_id = wallet.record.wallet_group_id.clone();
                    state.active_wallet_id = Some(group_id);
                    state.active_child_id = Some(id);
                }
            }
        }
        Ok(())
    }

    pub fn list() -> Result<Vec<WalletRecord>, EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        Ok(state.wallets.values().map(|w| w.record.clone()).collect())
    }

    pub fn active() -> Result<Option<WalletRecord>, EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        Ok(state.active_child_id.as_ref().and_then(|id| state.wallets.get(id).map(|w| w.record.clone())))
    }

    pub fn active_group_id() -> Result<Option<String>, EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        Ok(state.active_wallet_id.clone())
    }

    pub fn active_child_id() -> Result<Option<String>, EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        Ok(state.active_child_id.clone())
    }

    pub fn switch_child(child_id: &str) -> Result<WalletRecord, EngineError> {
        let mut state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let child = state.wallets.get(child_id)
            .ok_or_else(|| EngineError::Validation(format!("child wallet not found: {}", child_id)))?
            .record.clone();
        for wallet in state.wallets.values_mut() { wallet.record.active = false; }
        if let Some(wallet) = state.wallets.get_mut(child_id) { wallet.record.active = true; }
        state.active_wallet_id = Some(child.wallet_group_id.clone());
        state.active_child_id = Some(child_id.to_string());
        state.unlocked_secrets.clear();
        Ok(child)
    }

    pub fn children(wallet_id: &str) -> Result<Vec<WalletRecord>, EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let mut children: Vec<WalletRecord> = state.wallets.values()
            .filter(|w| w.record.secret_group_id == wallet_id)
            .map(|w| w.record.clone())
            .collect();
        if children.is_empty() { return Err(EngineError::Validation(format!("wallet group not found: {}", wallet_id))); }
        children.sort_by_key(|w| w.child_index);
        Ok(children)
    }

    pub fn get(wallet_id: &str) -> Result<WalletRecord, EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        state.wallets.get(wallet_id).map(|w| w.record.clone())
            .or_else(|| state.wallets.values().find(|w| w.record.secret_group_id == wallet_id && w.record.child_index == 0).map(|w| w.record.clone()))
            .ok_or_else(|| EngineError::Validation(format!("wallet/group not found: {}", wallet_id)))
    }

    /// Resolve chain-specific signing material internally. The caller supplies
    /// the PIN only to this trusted wallet layer; adapters never access
    /// encrypted storage or wallet secrets directly.
    pub fn derive_address_for_child(
        wallet_id: &str,
        chain: &ChainRequest,
        password: &str,
    ) -> Result<String, EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let child_id = state.wallets.get(wallet_id).map(|w| w.record.wallet_id.clone())
            .or_else(|| state.active_child_id.clone().filter(|id| state.wallets.get(id).map(|w| w.record.wallet_group_id == wallet_id).unwrap_or(false)))
            .or_else(|| state.wallets.values().find(|w| w.record.wallet_group_id == wallet_id && w.record.child_index == 0).map(|w| w.record.wallet_id.clone()))
            .ok_or_else(|| EngineError::Validation(format!("wallet/group not found: {}", wallet_id)))?;
        let wallet = state.wallets.get(&child_id)
            .ok_or_else(|| EngineError::Validation(format!("child wallet not found: {}", child_id)))?;
        if wallet.record.wallet_type != WalletType::Hd {
            return Err(EngineError::Validation("address derivation from seed is only available for HD wallets".into()));
        }
        let secret = state.secrets.get(&wallet.record.secret_group_id)
            .ok_or_else(|| EngineError::Unexpected(format!("wallet secret group missing: {}", wallet.record.secret_group_id)))?;
        let seed = EncryptionService::decrypt_bytes(&secret.encrypted_seed, password)?;
        let mut child_chain = chain.clone();
        child_chain.derivation_path = Some(Self::path_for_child_index(
            &chain.path()?,
            wallet.record.child_index,
        )?);
        crate::services::address_service::AddressService::derive_address(&seed, &child_chain)
    }

    pub fn resolve_signing_key(
        wallet_id: &str,
        blockchain: &str,
        password: &str,
    ) -> Result<Vec<u8>, EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let child_id = state.wallets.get(wallet_id).map(|w| w.record.wallet_id.clone())
            .or_else(|| state.active_child_id.clone().filter(|id| state.wallets.get(id).map(|w| w.record.wallet_group_id == wallet_id).unwrap_or(false)))
            .or_else(|| state.wallets.values().find(|w| w.record.wallet_group_id == wallet_id && w.record.child_index == 0).map(|w| w.record.wallet_id.clone()))
            .ok_or_else(|| EngineError::Validation(format!("wallet/group not found: {}", wallet_id)))?;
        let wallet = state.wallets.get(&child_id)
            .ok_or_else(|| EngineError::Validation(format!("child wallet not found: {}", child_id)))?;
        let matching_chains: Vec<_> = wallet.record.blockchains.iter()
            .filter(|b| b.chain_id.eq_ignore_ascii_case(blockchain)
                || b.name.eq_ignore_ascii_case(blockchain)
                || b.chain_type.eq_ignore_ascii_case(blockchain))
            .collect();
        if matching_chains.is_empty() {
            return Err(EngineError::Validation(format!(
                "CHAIN_NOT_REGISTERED: wallet {} has no blockchain configured for {}",
                wallet_id, blockchain
            )));
        }
        if matching_chains.len() > 1 {
            return Err(EngineError::Validation(format!(
                "CHAIN_SELECTION_AMBIGUOUS: provide chain_id; '{}' matches multiple configured networks",
                blockchain
            )));
        }
        let chain = matching_chains[0];
        let secret = state.secrets.get(&wallet.record.secret_group_id)
            .ok_or_else(|| EngineError::Unexpected(format!(
                "wallet secret group missing: {}",
                wallet.record.secret_group_id
            )))?;

        match wallet.record.wallet_type {
            WalletType::PrivateKey => {
                let encrypted = secret.encrypted_private_key.as_ref()
                    .ok_or_else(|| EngineError::Validation("wallet does not contain an imported private key".into()))?;
                EncryptionService::decrypt_bytes(encrypted, password)
            }
            WalletType::Hd => {
                let seed = EncryptionService::decrypt_bytes(&secret.encrypted_seed, password)?;
                crate::crypto::key_derivation::KeyDerivationService::derive_private_key_at_path(&seed, &chain.chain_type, &chain.derivation_path)
            }
        }
    }

    pub fn choices_for_chains(chains: &[ChainRequest]) -> Result<Vec<WalletChoice>, EngineError> {
        let state = registry().lock().map_err(|_| EngineError::Unexpected("wallet registry lock poisoned".into()))?;
        let wanted: Vec<(String, String)> = chains.iter()
            .map(|c| (c.name.to_ascii_lowercase(), c.chain_type.to_ascii_lowercase()))
            .collect();
        Ok(state.wallets.values().map(|wallet| {
            let matching = wallet.record.blockchains.iter()
                .filter(|b| wanted.iter().any(|(name, chain_type)|
                    name == &b.name.to_ascii_lowercase() && chain_type == &b.chain_type.to_ascii_lowercase()
                ))
                .cloned().collect();
            WalletChoice {
                wallet_id: wallet.record.wallet_id.clone(),
                name: wallet.record.name.clone(),
                secret_group_id: wallet.record.secret_group_id.clone(),
                child_index: wallet.record.child_index,
                matching_blockchains: matching,
                active: wallet.record.active,
            }
        }).filter(|choice| !choice.matching_blockchains.is_empty()).collect())
    }

    fn group_id(seed: &[u8]) -> String {
        let prefix_len = seed.len().min(12);
        format!("seed_{}", hex::encode(&seed[..prefix_len]))
    }

    /// Return the derivation path for an exact logical child index.
    ///
    /// The final path component is the account/child index for the wallet
    /// schemes supported by this registry. Setting it directly makes the
    /// operation deterministic and idempotent for both creation and import.
    fn path_for_child_index(path: &str, index: u32) -> Result<String, EngineError> {
        let mut parts: Vec<String> = path.split('/').map(str::to_string).collect();
        if parts.len() < 2 || parts[0] != "m" {
            return Err(EngineError::Validation(format!("unsupported child derivation path: {}", path)));
        }
        let last = parts.last_mut()
            .ok_or_else(|| EngineError::Validation(format!("invalid derivation path: {}", path)))?;
        let hardened = last.ends_with('\'');
        last.trim_end_matches('\'')
            .parse::<u32>()
            .map_err(|_| EngineError::Validation(format!("invalid child index in derivation path: {}", path)))?;
        *last = format!("{}{}", index, if hardened { "'" } else { "" });
        Ok(parts.join("/"))
    }

    fn build_blockchains(chains: &[ChainRequest], addresses: &serde_json::Value) -> Result<Vec<WalletBlockchain>, EngineError> {
        let object = addresses.as_object().ok_or_else(|| EngineError::Unexpected("derived addresses must be an object".into()))?;
        chains.iter().map(|chain| {
            let info = ChainInfo::from_request(chain)?;
            let normalized_name = info.name.trim().to_ascii_lowercase().replace([' ', '-', '/'], "_");
            let address = object.get(&info.name)
                .or_else(|| object.get(&normalized_name))
                .and_then(|v| v.as_str())
                .ok_or_else(|| EngineError::Unexpected(format!("missing derived address for {}", info.name)))?;
            Ok(WalletBlockchain {
                name: info.name,
                chain_id: chain.chain_id.clone(),
                symbol: chain.symbol.clone(),
                decimals: default_decimals(&chain.chain_type, chain.symbol.as_deref()),
                chain_type: info.chain_type,
                derivation_path: info.derivation_path,
                address: address.to_string(),
                rpc_provider: chain.rpc_provider.clone(),
                rpc_url: chain.rpc_url.clone(),
                balance: None,
                tokens: Vec::new(),
            })
        }).collect()
    }
}


#[cfg(test)]
mod tests {
    use super::{WalletRegistry, WalletRegistrySnapshot};
    use std::collections::HashMap;

    #[test]
    fn registry_snapshot_round_trips_through_json() {
        let snapshot = WalletRegistrySnapshot {
            version: 1,
            wallets: Vec::new(),
            secrets: HashMap::new(),
            active_wallet_id: None,
            active_child_id: None,
            next_wallet_id: 0,
        };
        let encoded = serde_json::to_string(&snapshot).unwrap();
        let decoded: WalletRegistrySnapshot = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.version, 1);
        assert!(decoded.wallets.is_empty());
        assert!(decoded.secrets.is_empty());
    }

    #[test]
    fn registry_snapshot_rejects_unsupported_version() {
        let snapshot = WalletRegistrySnapshot {
            version: 999,
            wallets: Vec::new(),
            secrets: HashMap::new(),
            active_wallet_id: None,
            active_child_id: None,
            next_wallet_id: 0,
        };
        assert!(WalletRegistry::restore_snapshot(snapshot).is_err());
    }

    #[test]
    fn child_path_uses_exact_index_for_evm() {
        assert_eq!(
            WalletRegistry::path_for_child_index("m/44'/60'/0'/0/0", 2).unwrap(),
            "m/44'/60'/0'/0/2"
        );
        assert_eq!(
            WalletRegistry::path_for_child_index("m/44'/60'/0'/0/1", 2).unwrap(),
            "m/44'/60'/0'/0/2"
        );
    }

    #[test]
    fn child_path_uses_exact_index_for_tron() {
        assert_eq!(
            WalletRegistry::path_for_child_index("m/44'/195'/0'/0/0", 3).unwrap(),
            "m/44'/195'/0'/0/3"
        );
    }

    #[test]
    fn child_path_preserves_hardened_final_index() {
        assert_eq!(
            WalletRegistry::path_for_child_index("m/44'/501'/0'/0'", 4).unwrap(),
            "m/44'/501'/0'/4'"
        );
    }

    #[test]
    fn explicit_child_index_is_read_from_derivation_path() {
        assert_eq!(
            WalletRegistry::child_index_from_path("m/44'/60'/0'/0/5").unwrap(),
            5
        );
        assert_eq!(
            WalletRegistry::child_index_from_path("m/44'/501'/0'/4'").unwrap(),
            4
        );
        assert!(WalletRegistry::child_index_from_path("m/44'/60'/0'/0/not-an-index").is_err());
    }
}
