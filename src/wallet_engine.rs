use crate::common::errors::EngineError;
use crate::crypto::address_derivation::AddressDerivationService;
use crate::crypto::mnemonic::generate_mnemonic;
use crate::services::address_service::AddressService;
use crate::crypto::encryption::EncryptionService;
use crate::services::transaction::TransactionService;
use crate::types::chain::ChainRequest;
use crate::services::wallet::{WalletOrchestrator, WalletService, WalletState};
use crate::services::wallet::WalletRegistry;
use crate::crypto::key_derivation::KeyDerivationService;
use crate::services::wallet_create_service::CreateWalletOrchestrator;
use crate::services::wallet_create_service::{SecretPhraseService, WalletCreateService};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::adapters::orchestrator::{AdapterOrchestrator, AdapterRuntimeConfig};
use crate::adapters::registry::AdapterRegistry;
use crate::services::balance_service::{BalanceFetchService, BalanceProviderRegistry, BalanceRequest};
use crate::services::event_service::EventService;
use crate::services::rpc::RpcOrchestrator;
use std::sync::Arc;
use std::time::Duration;

// Render/runtime RPC overrides take precedence over URLs embedded in the chain
// catalog. Example: NODEX_RPC_ETHEREUM=https://... or NODEX_RPC_1=https://...
fn rpc_env_key_part(value: &str) -> String {
    value.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_uppercase() } else { '_' })
        .collect()
}

fn resolve_rpc_endpoint(chain_name: &str, chain_id: &str, configured_url: Option<&str>) -> String {
    for key in [
        format!("NODEX_RPC_{}", rpc_env_key_part(chain_name)),
        format!("NODEX_RPC_{}", rpc_env_key_part(chain_id)),
    ] {
        if let Ok(value) = std::env::var(&key) {
            let value = value.trim();
            if !value.is_empty() {
                return value.to_owned();
            }
        }
    }
    configured_url.unwrap_or_default().trim().to_owned()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WalletRequest {
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WalletResponse {
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
}

impl WalletResponse {
    pub fn ok(data: Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

pub struct WalletProvider {
    wallet_id: String,
    mnemonic: String,
    seed: Vec<u8>,
    private_key: Option<Vec<u8>>,
    wallet_service: WalletService,
}

impl WalletProvider {
    pub fn create_wallet(word_count: usize, chains: &[ChainRequest], pin: Option<&str>) -> Result<Self, EngineError> {
        let result = WalletCreateService::create_with_chains(word_count, pin, chains)?;
        let wallet_service = WalletService::new(&result.state.mnemonic)?;

        Ok(Self {
            wallet_id: result.wallet.child_id.clone(),
            mnemonic: result.state.mnemonic,
            seed: result.state.seed,
            private_key: None,
            wallet_service,
        })
    }

    pub fn import_wallet(mnemonic: &str, passphrase: Option<&str>, chains: &[ChainRequest], pin: Option<&str>) -> Result<Self, EngineError> {
        let result = WalletCreateService::import_with_chains(mnemonic, passphrase, pin, chains)?;
        let wallet_service = WalletService::new(&result.state.mnemonic)?;

        Ok(Self {
            wallet_id: result.wallet.child_id.clone(),
            mnemonic: result.state.mnemonic,
            seed: result.state.seed,
            private_key: None,
            wallet_service,
        })
    }

    pub fn new(mnemonic: &str) -> Result<Self, EngineError> {
        let state = WalletCreateService::initialize(mnemonic, None)?;
        let wallet_service = WalletService::new(&state.mnemonic)?;
        // A provider initialized from a supplied mnemonic is standalone
        // until it creates/imports or explicitly selects a registered wallet.
        // Never bind it implicitly to whichever unrelated wallet is active.
        Ok(Self {
            wallet_id: String::new(),
            mnemonic: state.mnemonic,
            seed: state.seed,
            private_key: None,
            wallet_service,
        })
    }

    pub fn get_address_by_chain(&self, chain: &ChainRequest) -> Result<String, EngineError> {
        AddressService::derive_address(&self.seed, chain)
    }

    pub fn sign_transaction_by_chain(&self, chain_type: &str, tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        let private_key = match self.private_key.as_deref() { Some(key) => key.to_vec(), None => KeyDerivationService::derive_private_key(&self.seed, chain_type)? };
        TransactionService::sign_transaction(chain_type, &private_key, tx_data)
    }

    pub fn derive_address_from_path(&self, chain_type: &str, path: &str) -> Result<String, EngineError> {
        let params = json!({
            "chain_type": chain_type,
            "path": path
        });
        let result = AdapterOrchestrator::with_default_rpc()?.derive(&self.seed, "derive_address", &params)?;
        result
            .get("address")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| EngineError::Unexpected(format!(
                "adapter returned no address for chain_type {}",
                chain_type
            )))
    }

    pub fn backup_wallet(&self) -> Result<String, EngineError> {
        self.wallet_service.backup_wallet()
    }

    pub fn restore_wallet(&mut self, mnemonic: &str) -> Result<(), EngineError> {
        let payload = SecretPhraseService::import(mnemonic, None)?;
        self.mnemonic = payload.mnemonic;
        self.seed = payload.seed;
        self.wallet_service.restore_wallet(&self.mnemonic)?;
        Ok(())
    }

    pub fn wallet_state(&self) -> WalletState {
        self.wallet_service.state()
    }

    pub fn wallet_id(&self) -> &str {
        &self.wallet_id
    }

    pub fn mnemonic(&self) -> &str {
        &self.mnemonic
    }

    pub fn encrypt_vault(&self, password: &str) -> Result<String, EngineError> {
        EncryptionService::encrypt(&self.mnemonic, password)
    }

    pub fn decrypt_vault(encrypted_vault: &str, password: &str) -> Result<String, EngineError> {
        EncryptionService::decrypt(encrypted_vault, password)
    }

    /// Engine-wide request lifecycle. UI subscribers receive progress and a
    /// sanitized JSON result independently of the request/response HTTP call.
    pub async fn handle_request_async(&mut self, request: WalletRequest) -> Result<WalletResponse, EngineError> {
        let action = request.action.clone();
        let request_id = request.params.get("request_id").and_then(Value::as_str).map(str::to_owned);
        let mut context = json!({ "action": action, "request_id": request_id });
        if let Some(wallet_id) = request.params.get("wallet_id").and_then(Value::as_str) {
            context["wallet_id"] = json!(wallet_id);
        }
        if let Some(child_id) = request.params.get("child_id").and_then(Value::as_str) {
            context["child_id"] = json!(child_id);
        }
        if let Some(chain_id) = request.params.get("chain_id").and_then(Value::as_str) {
            context["chain_id"] = json!(chain_id);
        }
        EventService::publish("request.started", context.clone());

        let result = self.handle_request_async_inner(request).await;
        match &result {
            Ok(response) => {
                let mut safe_data = response.data.clone().unwrap_or(Value::Null);
                Self::redact_event_secrets(&mut safe_data);
                let mut event = context;
                event["success"] = json!(response.success);
                event["status"] = json!(if response.success { "success" } else { "error" });
                event["data"] = safe_data;
                if let Some(error) = response.error.as_deref() {
                    event["message"] = json!(error);
                }
                EventService::publish(
                    if response.success { "request.completed" } else { "request.failed" },
                    event,
                );
            }
            Err(error) => {
                let mut event = context;
                event["success"] = json!(false);
                event["status"] = json!("error");
                event["message"] = json!(error.to_string());
                EventService::publish("request.failed", event);
            }
        }
        result
    }

    fn redact_event_secrets(value: &mut Value) {
        match value {
            Value::Object(object) => {
                for key in ["mnemonic", "seed", "private_key", "secret_phrase", "backup", "password", "pin", "encrypted_vault", "encrypted_mnemonic"] {
                    object.remove(key);
                }
                for nested in object.values_mut() {
                    Self::redact_event_secrets(nested);
                }
            }
            Value::Array(items) => {
                for item in items {
                    Self::redact_event_secrets(item);
                }
            }
            _ => {}
        }
    }

    /// Async adapter boundary. WalletEngine only routes chain_type and forwards
    /// the opaque body; adapter-specific actions are interpreted below the registry.
    async fn handle_request_async_inner(&mut self, request: WalletRequest) -> Result<WalletResponse, EngineError> {
        // Wallet creation is request/response only. It must not trigger balance/RPC work.
        if matches!(
            request.action.as_str(),
            "create_wallet" | "create_wallet_group" | "create_master_wallet" | "generate_wallet" | "add_wallet"
        ) {
            // Every master-group creation alias uses the same deterministic
            // group + Child 0 path and never triggers balance/RPC work.
            return self.handle_create_wallet(request.params);
        }
        if request.action == "discover_child_accounts" {
            return self.handle_discover_child_accounts(request.params).await;
        }
        if request.action == "get_balance" { return self.handle_get_balance(request.params); }
        if request.action == "refresh_balance" { return self.handle_refresh_balance(request.params).await; }
        if request.action != "adapter_call" && request.action != "adapter_target" {
            return self.handle_request_inner(request);
        }

        let wallet = self.resolve_wallet_record(&request.params)?;
        let wallet_id = wallet.wallet_id.clone();

        let targets = if request.action == "adapter_target" {
            request.params.get("targets").and_then(Value::as_array)
                .ok_or_else(|| EngineError::Validation("adapter_target requires targets".into()))?
                .clone()
        } else {
            vec![request.params.clone()]
        };

        let mut results = Vec::with_capacity(targets.len());

        for target in targets {
            let chain_type = target.get("chain_type")
                .and_then(Value::as_str)
                .ok_or_else(|| EngineError::Validation("adapter_target requires chain_type".into()))?;
            let body = target.get("body")
                .ok_or_else(|| EngineError::Validation("adapter_target requires body".into()))?;

            let chain_id = target.get("chain_id").or_else(|| request.params.get("chain_id"))
                .and_then(Value::as_str);
            let matching: Vec<_> = wallet.blockchains.iter()
                .filter(|b| b.chain_type.eq_ignore_ascii_case(chain_type)
                    && chain_id.map(|id| b.chain_id.eq_ignore_ascii_case(id)).unwrap_or(true))
                .collect();
            if matching.is_empty() {
                return Err(EngineError::Validation(format!(
                    "CHAIN_NOT_REGISTERED: child {} has no configured {} network{}",
                    wallet_id, chain_type,
                    chain_id.map(|id| format!(" with chain_id {}", id)).unwrap_or_default()
                )));
            }
            if matching.len() > 1 {
                return Err(EngineError::Validation(format!(
                    "CHAIN_SELECTION_AMBIGUOUS: provide chain_id for child {} / chain_type {}",
                    wallet_id, chain_type
                )));
            }
            let blockchain = matching[0];

            let rpc_provider = blockchain.rpc_provider.clone().unwrap_or_default();
            let rpc_url = blockchain.rpc_url.clone().unwrap_or_default();
            if rpc_provider.trim().is_empty() || rpc_url.trim().is_empty() {
                return Err(EngineError::Validation(format!(
                    "RPC provider and RPC URL are not configured for child {} / chain_id {}",
                    wallet_id, blockchain.chain_id
                )));
            }

            // Pass only public child/network context to the adapter. PIN,
            // mnemonic, seed and private keys never cross this boundary.
            let mut adapter_body = body.clone();
            if let Some(object) = adapter_body.as_object_mut() {
                // Always overwrite caller-supplied context; adapter context must
                // come from the authoritative registry, never from request spoofing.
                object.insert("wallet_context".to_string(), json!({
                    "wallet_id": wallet.wallet_group_id,
                    "child_id": wallet.wallet_id,
                    "child_index": wallet.child_index,
                    "chain_id": blockchain.chain_id,
                    "chain_type": blockchain.chain_type,
                    "address": blockchain.address,
                    "derivation_path": blockchain.derivation_path
                }));
            }
            let body_bytes = serde_json::to_vec(&adapter_body)
                .map_err(|e| EngineError::Validation(format!("invalid adapter body: {e}")))?;
            let response = AdapterOrchestrator::with_default_rpc()?.call(
                chain_type,
                &body_bytes,
                AdapterRuntimeConfig {
                    rpc_provider,
                    rpc_url,
                    chain_name: Some(blockchain.name.clone()),
                },
            ).await?;

            let data: Value = serde_json::from_slice(&response)
                .map_err(|e| EngineError::Unexpected(format!("adapter returned invalid JSON: {e}")))?;
            results.push(json!({
                "child_id": wallet.wallet_id,
                "chain_id": blockchain.chain_id,
                "chain_type": chain_type,
                "address": blockchain.address,
                "data": data
            }));
        }

        Ok(WalletResponse::ok(json!({
            "targets": results
        })))
    }

    async fn handle_discover_child_accounts(&self, params: Value) -> Result<WalletResponse, EngineError> {
        let chain_type = params.get("chain_type")
            .and_then(Value::as_str)
            .unwrap_or("tron");
        let start_index = params.get("start_index")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let max_count = params.get("max_count")
            .and_then(Value::as_u64)
            .unwrap_or(20);
        let gap_limit = params.get("gap_limit")
            .and_then(Value::as_u64)
            .unwrap_or(3);
        let pin = params.get("pin").or_else(|| params.get("password"))
            .and_then(Value::as_str)
            .ok_or_else(|| EngineError::Validation("wallet PIN/password is required".into()))?;

        let wallet = self.resolve_wallet_record(&params)?;
        let wallet_id = wallet.wallet_id.clone();
        if !wallet.wallet_type.eq(&crate::services::wallet_registry::WalletType::Hd) {
            return Err(EngineError::Validation("child discovery requires an HD wallet".into()));
        }
        let blockchain = wallet.blockchains.iter()
            .find(|chain| chain.chain_type.eq_ignore_ascii_case(chain_type) || chain.name.eq_ignore_ascii_case(chain_type))
            .ok_or_else(|| EngineError::Validation(format!("wallet {} has no blockchain configured for {}", wallet_id, chain_type)))?;
        let rpc_provider = blockchain.rpc_provider.clone().unwrap_or_default();
        let rpc_url = blockchain.rpc_url.clone().unwrap_or_default();
        if rpc_provider.trim().is_empty() || rpc_url.trim().is_empty() {
            return Err(EngineError::Validation(format!("RPC provider and RPC URL are required for {} child discovery", chain_type)));
        }
        let (_, seed) = WalletRegistry::wallet_secret(&wallet.wallet_id, pin)?;
        let rpc = RpcOrchestrator::with_defaults()?;
        let discovered = AdapterRegistry::discover_child_accounts(
            chain_type,
            &rpc,
            AdapterRuntimeConfig {
                rpc_provider,
                rpc_url,
                chain_name: Some(blockchain.name.clone()),
            },
            &seed,
            u32::try_from(start_index).map_err(|_| EngineError::Validation("start_index is out of range".into()))?,
            u32::try_from(max_count).map_err(|_| EngineError::Validation("max_count is out of range".into()))?,
            u32::try_from(gap_limit).map_err(|_| EngineError::Validation("gap_limit is out of range".into()))?,
        ).await?;

        Ok(WalletResponse::ok(json!({
            "wallet_id": wallet.wallet_group_id,
            "chain_type": chain_type,
            "start_index": start_index,
            "max_count": max_count,
            "gap_limit": gap_limit,
            "children": discovered,
        })))
    }

    fn handle_get_balance(&self, params: Value) -> Result<WalletResponse, EngineError> {
        let wallet = self.resolve_wallet_record(&params)?;
        let chain_id = params.get("chain_id").and_then(Value::as_str);
        let balances = BalanceFetchService::get_saved_balance(&wallet.wallet_id, chain_id)?;
        Ok(WalletResponse::ok(json!({
            "wallet_id": wallet.wallet_group_id,
            "child_id": wallet.wallet_id,
            "child_index": wallet.child_index,
            "balances": balances
        })))
    }

    async fn handle_refresh_balance(&self, params: Value) -> Result<WalletResponse, EngineError> {
        // Preferred wallet-aware flow: resolve a specific registered child and
        // use only its already-saved addresses/network/token configuration.
        // No address derivation and no caller-supplied address map are needed.
        if params.get("child_id").and_then(Value::as_str).is_some()
            || params.get("wallet_id").and_then(Value::as_str).is_some()
            || params.get("wallet_group_id").and_then(Value::as_str).is_some()
            || WalletOrchestrator::active_child_id()?.is_some()
            || (params.get("chains").is_none() && params.get("addresses").is_none())
        {
            let wallet = self.resolve_wallet_record(&params)?;
            let requested_chain_id = params.get("chain_id").and_then(Value::as_str);
            let requests: Vec<BalanceRequest> = wallet.blockchains.iter()
                .filter(|chain| requested_chain_id.map(|id| chain.chain_id.eq_ignore_ascii_case(id)).unwrap_or(true))
                .map(|chain| BalanceRequest {
                    wallet_id: wallet.wallet_id.clone(),
                    provider: chain.rpc_provider.as_deref()
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or("public")
                        .to_owned(),
                    rpc_endpoint: resolve_rpc_endpoint(&chain.name, &chain.chain_id, chain.rpc_url.as_deref()),
                    chain: ChainRequest {
                        name: chain.name.clone(),
                        chain_id: chain.chain_id.clone(),
                        symbol: chain.symbol.clone(),
                        chain_type: chain.chain_type.clone(),
                        derivation_path: Some(chain.derivation_path.clone()),
                        address_type: None,
                        network: None,
                        p2pkh_version: None,
                        p2sh_version: None,
                        bech32_hrp: None,
                        rpc_provider: chain.rpc_provider.clone().filter(|value| !value.trim().is_empty()).or_else(|| Some("public".to_owned())),
                        rpc_url: Some(resolve_rpc_endpoint(&chain.name, &chain.chain_id, chain.rpc_url.as_deref())),
                    },
                    address: chain.address.clone(),
                    assets: chain.tokens.iter().map(|token| token.contract_address.clone()).collect(),
                }).collect();

            if requests.is_empty() {
                return Err(EngineError::Validation(format!(
                    "CHAIN_NOT_REGISTERED: no configured chain matches the requested network for child '{}'",
                    wallet.wallet_id
                )));
            }
            let total_requests = requests.len();
            let service = Arc::new(BalanceFetchService::new(
                BalanceProviderRegistry::with_defaults(),
                Arc::new(RpcOrchestrator::with_defaults()?),
            ));
            // Keep each chain's outcome independent: missing configuration and
            // transport/provider errors are returned alongside successful balances.
            let configured_requests = requests.iter()
                .filter(|request| !request.provider.trim().is_empty() && !request.rpc_endpoint.trim().is_empty())
                .cloned()
                .collect();
            let mut rx = service.get_balance(configured_requests).await;
            let mut balances = Vec::new();
            let mut errors = Vec::new();
            for request in requests.iter().filter(|request| request.provider.trim().is_empty() || request.rpc_endpoint.trim().is_empty()) {
                let failure = json!({
                    "wallet_id": request.wallet_id,
                    "chain_id": request.chain.chain_id,
                    "chain_name": request.chain.name,
                    "provider": request.provider,
                    "rpc_url": request.rpc_endpoint,
                    "message": "each requested chain needs a configured RPC provider and RPC URL"
                });
                EventService::publish("balance.fetch.failed", failure.clone());
                errors.push(failure);
            }
            while let Some(result) = rx.recv().await {
                match result {
                    Ok(balance) => {
                        // Persist each successful chain before announcing it as updated.
                        WalletRegistry::update_balances(&[balance.clone()])?;
                        balances.push(balance);
                    }
                    Err(error) => errors.push(json!({
                        "wallet_id": error.wallet_id,
                        "chain_id": error.chain_id,
                        "provider": error.provider,
                        "message": error.message
                    })),
                }
            }
            let success = errors.is_empty();
            let status = if success { "success" } else if balances.is_empty() { "failed" } else { "partial_success" };
            let error = if success { None } else {
                Some(format!("balance refresh {}: {} of {} chain(s) failed",
                    if balances.is_empty() { "failed" } else { "partially failed" },
                    errors.len(), total_requests))
            };
            EventService::publish("balance.refresh.completed", json!({
                "wallet_id": wallet.wallet_group_id,
                "child_id": wallet.wallet_id,
                "request_id": params.get("request_id").and_then(Value::as_str),
                "success_count": balances.len(),
                "failure_count": errors.len(),
                "status": status
            }));
            let data = json!({
                "wallet_id": wallet.wallet_group_id,
                "child_id": wallet.wallet_id,
                "child_index": wallet.child_index,
                "balances": balances,
                "errors": errors,
                "success_count": balances.len(),
                "failure_count": errors.len(),
                "status": status,
                "attempted": true,
                "refreshed": success,
                "addresses_reused": true
            });
            return Ok(WalletResponse { success, data: Some(data), error });
        }

        // Legacy stateless refresh is retained only for hosts with no active
        // registry session. Once a session exists, it always takes precedence over
        // caller-supplied wallet/address maps.
        let wallet_id = params.get("wallet_id")
            .and_then(Value::as_str)
            .unwrap_or("balance_request");

        let chains = params.get("chains")
            .and_then(Value::as_array)
            .ok_or_else(|| EngineError::Validation("refresh_balance requires chains".into()))?;

        let addresses = params.get("addresses")
            .and_then(Value::as_object)
            .ok_or_else(|| EngineError::Validation("refresh_balance requires addresses".into()))?;

        let requested_chain_id = params.get("chain_id").and_then(Value::as_str);

        let requests: Vec<BalanceRequest> = chains.iter()
            .filter_map(|value| {
                let chain_id = value.get("chain_id").and_then(Value::as_str)?.to_string();
                if requested_chain_id.map(|id| id != chain_id).unwrap_or(false) {
                    return None;
                }

                let name = value.get("name").and_then(Value::as_str).unwrap_or(&chain_id);
                let address = addresses.get(name)
                    .or_else(|| addresses.get(&chain_id))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();

                Some(Ok(BalanceRequest {
                    wallet_id: wallet_id.to_string(),
                    provider: value.get("rpc_provider").and_then(Value::as_str)
                        .filter(|provider| !provider.trim().is_empty())
                        .unwrap_or("public").to_string(),
                    rpc_endpoint: resolve_rpc_endpoint(name, &chain_id, value.get("rpc_url").and_then(Value::as_str)),
                    chain: ChainRequest {
                        name: name.to_string(),
                        chain_id,
                        symbol: value.get("symbol").and_then(Value::as_str).map(str::to_string),
                        chain_type: value.get("chain_type").and_then(Value::as_str).unwrap_or("other").to_string(),
                        derivation_path: value.get("derivation_path").and_then(Value::as_str).map(str::to_string),
                        address_type: value.get("address_type").and_then(Value::as_str).map(str::to_string),
                        network: value.get("network").and_then(Value::as_str).map(str::to_string),
                        p2pkh_version: value.get("p2pkh_version").and_then(Value::as_u64).and_then(|v| u8::try_from(v).ok()),
                        p2sh_version: value.get("p2sh_version").and_then(Value::as_u64).and_then(|v| u8::try_from(v).ok()),
                        bech32_hrp: value.get("bech32_hrp").and_then(Value::as_str).map(str::to_string),
                        rpc_provider: value.get("rpc_provider").and_then(Value::as_str).map(str::to_string),
                        rpc_url: value.get("rpc_url").and_then(Value::as_str).map(str::to_string),
                    },
                    address,
                    assets: value.get("tokens")
                        .and_then(Value::as_array)
                        .map(|tokens| tokens.iter().filter_map(|t| t.get("contract_address").and_then(Value::as_str).map(str::to_string)).collect())
                        .unwrap_or_default(),
                }))
            })
            .collect::<Result<Vec<_>, EngineError>>()?;

        if requests.is_empty() {
            return Ok(WalletResponse::ok(json!({ "balances": [], "refreshed": true })));
        }

        let total_requests = requests.len();
        let service = Arc::new(BalanceFetchService::new(
            BalanceProviderRegistry::with_defaults(),
            Arc::new(RpcOrchestrator::with_defaults()?),
        ));
        let mut rx = service.get_balance(requests).await;
        let mut balances = Vec::new();
        let mut errors = Vec::new();
        while let Some(result) = rx.recv().await {
            match result {
                Ok(balance) => balances.push(balance),
                Err(error) => errors.push(json!({
                    "wallet_id": error.wallet_id,
                    "chain_id": error.chain_id,
                    "provider": error.provider,
                    "message": error.message
                })),
            }
        }

        let success = errors.is_empty();
        let status = if success { "success" } else if balances.is_empty() { "failed" } else { "partial_success" };
        let error = if success { None } else {
            Some(format!("balance refresh {}: {} of {} chain(s) failed",
                if balances.is_empty() { "failed" } else { "partially failed" },
                errors.len(), total_requests))
        };
        Ok(WalletResponse {
            success,
            data: Some(json!({
                "balances": balances,
                "errors": errors,
                "success_count": balances.len(),
                "failure_count": errors.len(),
                "status": status,
                "attempted": true,
                "refreshed": success
            })),
            error,
        })
    }

    /// Synchronous request path with the same lifecycle events as async requests.
    pub fn handle_request(&mut self, request: WalletRequest) -> Result<WalletResponse, EngineError> {
        let action = request.action.clone();
        let request_id = request.params.get("request_id").and_then(Value::as_str).map(str::to_owned);
        let mut context = json!({ "action": action, "request_id": request_id });
        if let Some(wallet_id) = request.params.get("wallet_id").and_then(Value::as_str) {
            context["wallet_id"] = json!(wallet_id);
        }
        if let Some(child_id) = request.params.get("child_id").and_then(Value::as_str) {
            context["child_id"] = json!(child_id);
        }
        if let Some(chain_id) = request.params.get("chain_id").and_then(Value::as_str) {
            context["chain_id"] = json!(chain_id);
        }
        EventService::publish("request.started", context.clone());

        let result = self.handle_request_inner(request);
        match &result {
            Ok(response) => {
                let mut safe_data = response.data.clone().unwrap_or(Value::Null);
                Self::redact_event_secrets(&mut safe_data);
                let mut event = context;
                event["success"] = json!(response.success);
                event["status"] = json!(if response.success { "success" } else { "error" });
                event["data"] = safe_data;
                if let Some(error) = response.error.as_deref() {
                    event["message"] = json!(error);
                }
                EventService::publish(
                    if response.success { "request.completed" } else { "request.failed" },
                    event,
                );
            }
            Err(error) => {
                let mut event = context;
                event["success"] = json!(false);
                event["status"] = json!("error");
                event["message"] = json!(error.to_string());
                EventService::publish("request.failed", event);
            }
        }
        result
    }

    fn handle_request_inner(&mut self, request: WalletRequest) -> Result<WalletResponse, EngineError> {
        match request.action.as_str() {
            "create_wallet" | "create_wallet_group" | "create_master_wallet" => self.handle_create_wallet(request.params),
            "add_wallet" => self.handle_create_wallet(request.params),
            "create_child_wallet" | "import_child_wallet" => self.handle_create_child_wallet(request.params),
            "switch_child" | "switch_child_wallet" => self.handle_switch_child(request.params),
            "generate_wallet" => self.handle_create_wallet(request.params),
            "import_wallet" => self.handle_import_wallet(request.params),
            "import_private_key" => self.handle_import_private_key(request.params),
            "restore_persisted_wallet" => self.handle_restore_persisted_wallet(request.params),
            "export_wallet_registry" | "persist_wallet_registry" => self.handle_export_wallet_registry(),
            "restore_wallet_registry" => self.handle_restore_wallet_registry(request.params),
            "delete_wallet" => self.handle_delete_wallet(request.params),
            "switch_wallet" | "switch_master_wallet" => self.handle_switch_wallet(request.params),
            "list_wallets" => self.handle_list_wallets(),
            "wallet_count" => self.handle_wallet_count(request.params),
            "wallet_group" => self.handle_wallet_group(request.params),
            "wallet_groups" => self.handle_wallet_groups(),
            "active_wallet" => self.handle_active_wallet(),
            "wallet_choices" => self.handle_wallet_choices(request.params),
            "resolve_wallets" => self.handle_wallet_choices(request.params),
            "add_blockchain" => self.handle_add_blockchain(request.params),
            "get_address" => self.handle_get_address(request.params),
            "get_wallet_context" | "get_child_context" => self.handle_get_wallet_context(request.params),
            "get_mnemonic" => self.handle_get_mnemonic(request.params),
            "add_token" | "add_token_metadata" => self.handle_add_token(request.params),
            "get_token" | "get_token_metadata" | "get_token_contract_address" => self.handle_get_token(request.params),
            "derive_address" => self.handle_adapter_derivation("derive_address", request.params),
            "derive_addresses" => self.handle_derive_addresses(request.params),
            "derive_private_key" => self.handle_adapter_derivation("derive_private_key", request.params),
            "derive_public_key" => self.handle_adapter_derivation("derive_public_key", request.params),
            "derive_child_key" => self.handle_adapter_derivation("derive_child_key", request.params),
            "backup_wallet" => self.handle_backup_wallet(request.params),
            "restore_wallet" => self.handle_restore_wallet(request.params),
            "encrypt_vault" => self.handle_encrypt_vault(request.params),
            "decrypt_vault" => self.handle_decrypt_vault(request.params),
            "sign_transaction" => self.handle_sign_transaction(request.params),
            "get_balance" => self.handle_get_balance(request.params),
            "derive_address_from_path" => self.handle_derive_address_from_path(request.params),
            other => Ok(WalletResponse::error(format!("unsupported action: {}", other))),
        }
    }


    fn handle_create_wallet(&mut self, params: Value) -> Result<WalletResponse, EngineError> {
        let word_count = params
            .get("word_count")
            .and_then(|value| value.as_u64())
            .map(|count| count as usize)
            .unwrap_or(12);
        let name = params.get("name").and_then(|value| value.as_str());
        let password = params.get("pin").or_else(|| params.get("password")).and_then(|value| value.as_str()).ok_or_else(|| EngineError::Validation("wallet PIN/password is required".into()))?;

        let chains = Self::parse_chains(&params)?;
        let result = CreateWalletOrchestrator::create(word_count, name, password, &chains)?;
        // The registry owns the authorized runtime secret cache. Do not keep
        // a second plaintext mnemonic/seed copy on the engine instance.
        *self = Self {
            wallet_id: result.wallet.child_id.clone(),
            mnemonic: String::new(),
            seed: Vec::new(),
            private_key: None,
            wallet_service: WalletService::new("")?,
        };

        // Wallet creation must remain fast and deterministic. Do not block
        // wallet creation on network/RPC balance calls; callers can invoke
        // refresh_balance explicitly after the wallet has been created.
        Ok(WalletResponse::ok(
            serde_json::to_value(&result.wallet)
                .map_err(|err| EngineError::Unexpected(err.to_string()))?
        ))
    }

    fn handle_import_wallet(&mut self, params: Value) -> Result<WalletResponse, EngineError> {
        let mnemonic = params
            .get("mnemonic")
            .and_then(|value| value.as_str())
            .ok_or_else(|| EngineError::Validation("missing mnemonic".into()))?;
        let passphrase = params.get("passphrase").and_then(|value| value.as_str());
        let name = params.get("name").and_then(|value| value.as_str());
        let password = params.get("pin").or_else(|| params.get("password")).and_then(|value| value.as_str()).ok_or_else(|| EngineError::Validation("wallet PIN/password is required".into()))?;

        let chains = Self::parse_chains(&params)?;
        let result = CreateWalletOrchestrator::import(mnemonic, passphrase, name, password, &chains)?;
        // The registry owns the authorized runtime secret cache. Do not keep
        // a second plaintext mnemonic/seed copy on the engine instance.
        *self = Self {
            wallet_id: result.wallet.child_id.clone(),
            mnemonic: String::new(),
            seed: Vec::new(),
            private_key: None,
            wallet_service: WalletService::new("")?,
        };

        Ok(WalletResponse::ok(serde_json::to_value(&result.wallet).map_err(|err| EngineError::Unexpected(err.to_string()))?))
    }

    fn handle_import_private_key(&mut self, params: Value) -> Result<WalletResponse, EngineError> {
        let key = params.get("private_key").and_then(|v| v.as_str()).ok_or_else(|| EngineError::Validation("missing private_key".into()))?;
        let password = params.get("pin").or_else(|| params.get("password")).and_then(|v| v.as_str()).ok_or_else(|| EngineError::Validation("wallet PIN/password is required".into()))?;

        // Parse the target chain before decoding: Solana commonly exports Base58
        // keypairs, while EVM private keys are hexadecimal.
        let chains_value = params.get("chains").and_then(Value::as_array);
        let mut chains: Vec<ChainRequest> = if let Some(values) = chains_value {
            values.iter().map(|value| serde_json::from_value(value.clone())
                .map_err(|e| EngineError::Validation(format!("invalid chain: {e}"))))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let chain_value = params.get("chain").ok_or_else(|| EngineError::Validation("missing chain or chains".into()))?;
            vec![serde_json::from_value(chain_value.clone()).map_err(|e| EngineError::Validation(format!("invalid chain: {e}")))?]
        };
        let is_solana = chains.len() == 1 && chains[0].chain_type.eq_ignore_ascii_case("solana");
        let private_key = if is_solana {
            let decoded = if key.trim().bytes().all(|b| b.is_ascii_hexdigit()) && (key.trim().len() == 64 || key.trim().len() == 128) {
                hex::decode(key.trim().strip_prefix("0x").unwrap_or(key.trim()))
                    .map_err(|e| EngineError::Validation(format!("invalid Solana private key: {e}")))?
            } else {
                bs58::decode(key.trim()).into_vec()
                    .map_err(|_| EngineError::Validation("invalid Solana key: expected a Base58 secret key or hexadecimal seed".into()))?
            };
            match decoded.len() {
                32 => decoded,
                64 => {
                    let secret = ed25519_dalek::SecretKey::from_bytes(&decoded[..32])
                        .map_err(|_| EngineError::Validation("invalid Solana 64-byte keypair".into()))?;
                    let public = ed25519_dalek::PublicKey::from(&secret);
                    if public.as_bytes() != &decoded[32..] {
                        return Err(EngineError::Validation("invalid Solana keypair: public-key bytes do not match its secret seed".into()));
                    }
                    decoded[..32].to_vec()
                }
                _ => return Err(EngineError::Validation(format!("invalid Solana key length: expected a 32-byte seed or 64-byte keypair, received {} bytes", decoded.len()))),
            }
        } else {
            let key = key.strip_prefix("0x").unwrap_or(key);
            hex::decode(key).map_err(|e| EngineError::Validation(format!("invalid_private_key_hex: {e}")))?
        };
        if chains.is_empty() {
            return Err(EngineError::Validation("select at least one blockchain".into()));
        }
        if chains.len() > 1 && chains.iter().any(|chain| !chain.chain_type.eq_ignore_ascii_case("evm")) {
            return Err(EngineError::Validation("private-key imports can select multiple chains only when every selected chain is EVM; select one non-EVM chain per private-key wallet".into()));
        }
        let first_chain = chains.remove(0);
        let address = AdapterRegistry::address_from_private_key(&first_chain.chain_type, &private_key)?;
        let mut record = WalletRegistry::register_private_key(params.get("name").and_then(|v| v.as_str()), first_chain, private_key.clone(), address, password)?;
        for chain in chains {
            if !chain.chain_type.eq_ignore_ascii_case("evm") {
                return Err(EngineError::Validation("only EVM chains can be added to a private-key wallet".into()));
            }
            let address = AdapterRegistry::address_from_private_key(&chain.chain_type, &private_key)?;
            record = WalletRegistry::add_blockchain(&record.wallet_id, chain, address)?;
        }
        self.wallet_id = record.wallet_id.clone(); self.mnemonic.clear(); self.seed.clear(); self.private_key = None;
        self.wallet_service = WalletService::new("")?;
        Ok(WalletResponse::ok(json!({ "wallet": record })))
    }

    fn handle_export_wallet_registry(&self) -> Result<WalletResponse, EngineError> {
        let snapshot = WalletRegistry::export_snapshot()?;
        let value = serde_json::to_value(snapshot)
            .map_err(|e| EngineError::Unexpected(format!("failed to serialize wallet registry snapshot: {e}")))?;
        Ok(WalletResponse::ok(json!({ "snapshot": value, "version": 1 })))
    }

    fn handle_restore_wallet_registry(&mut self, params: Value) -> Result<WalletResponse, EngineError> {
        let snapshot_value = params.get("snapshot")
            .ok_or_else(|| EngineError::Validation("missing wallet registry snapshot".into()))?;
        let snapshot: crate::services::wallet::registry::WalletRegistrySnapshot =
            serde_json::from_value(snapshot_value.clone())
                .map_err(|e| EngineError::Validation(format!("invalid wallet registry snapshot: {e}")))?;
        WalletRegistry::restore_snapshot(snapshot)?;
        self.wallet_id.clear();
        self.mnemonic.clear();
        self.seed.clear();
        self.private_key = None;
        self.wallet_service = WalletService::new("")?;
        if let Some(active) = WalletRegistry::active()? {
            self.wallet_id = active.wallet_id.clone();
        }
        Ok(WalletResponse::ok(json!({
            "restored": true,
            "wallets": WalletRegistry::list()?,
            "active_wallet_id": WalletRegistry::active_group_id()?,
            "active_child_id": WalletRegistry::active_child_id()?,
            "locked": true
        })))
    }

    fn handle_restore_persisted_wallet(&mut self, params: Value) -> Result<WalletResponse, EngineError> {
        let wallet_value = params.get("wallet").ok_or_else(|| EngineError::Validation("missing wallet".into()))?;
        let record: crate::services::wallet::WalletRecord = serde_json::from_value(wallet_value.clone())
            .map_err(|e| EngineError::Validation(format!("invalid persisted wallet: {e}")))?;
        let password = params.get("pin").or_else(|| params.get("password")).and_then(|v| v.as_str())
            .ok_or_else(|| EngineError::Validation("wallet PIN/password is required".into()))?;
        let activate = params.get("activate").and_then(|v| v.as_bool()).unwrap_or(true);

        match record.wallet_type {
            crate::services::wallet_registry::WalletType::Hd => {
                let mnemonic = params.get("mnemonic").and_then(|v| v.as_str()).ok_or_else(|| EngineError::Validation("missing mnemonic".into()))?.to_string();
                let state = SecretPhraseService::import(&mnemonic, None)?;
                let restored = WalletRegistry::restore_persisted(record, Some(state.mnemonic.clone()), Some(state.seed.clone()), None, password, activate)?;
                self.wallet_id = restored.wallet_id.clone();
                self.mnemonic.clear();
                self.seed.clear();
                self.private_key = None;
                self.wallet_service = WalletService::new("")?;
                Ok(WalletResponse::ok(json!({ "wallet": restored, "restored": true })))
            }
            crate::services::wallet_registry::WalletType::PrivateKey => {
                let key_hex = params.get("private_key").and_then(|v| v.as_str()).ok_or_else(|| EngineError::Validation("missing private_key".into()))?;
                let key = hex::decode(key_hex.strip_prefix("0x").unwrap_or(key_hex))
                    .map_err(|e| EngineError::Validation(format!("invalid private_key hex: {e}")))?;
                let restored = WalletRegistry::restore_persisted(record, None, None, Some(key.clone()), password, activate)?;
                self.wallet_id = restored.wallet_id.clone();
                self.mnemonic.clear();
                self.seed.clear();
                self.private_key = None;
                self.wallet_service = WalletService::new("")?;
                Ok(WalletResponse::ok(json!({ "wallet": restored, "restored": true })))
            }
        }
    }

    fn handle_delete_wallet(&mut self, params: Value) -> Result<WalletResponse, EngineError> {
        let wallet_id = params.get("wallet_id").and_then(|v| v.as_str()).unwrap_or(&self.wallet_id).to_string();
        let selected_group_deleted = WalletRegistry::get(&self.wallet_id)
            .map(|selected| selected.wallet_group_id == wallet_id)
            .unwrap_or(false);
        WalletRegistry::delete(&wallet_id)?;
        if wallet_id == self.wallet_id || selected_group_deleted {
            self.wallet_id.clear();
            self.mnemonic.clear();
            self.seed.clear();
            self.private_key = None;
            self.wallet_service = WalletService::new("")?;
        }
        Ok(WalletResponse::ok(json!({ "wallet_id": wallet_id, "deleted": true })))
    }

    fn handle_create_child_wallet(&mut self, params: Value) -> Result<WalletResponse, EngineError> {
        let parent_wallet_id = params.get("wallet_group_id")
            .or_else(|| params.get("wallet_id"))
            .or_else(|| params.get("parent_wallet_id"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .or(WalletOrchestrator::active_group_id()?)
            .ok_or_else(|| EngineError::Validation(
                "create_child_wallet requires an active master wallet or an explicit wallet_id".into()
            ))?;
        let name = params.get("name").and_then(|v| v.as_str());
        let chains = if params.get("chains").is_some() {
            Self::parse_chains(&params)?
        } else {
            Vec::new()
        };
        let explicit_index = params.get("child_index")
            .or_else(|| params.get("index"))
            .and_then(Value::as_u64)
            .map(|value| u32::try_from(value).map_err(|_| EngineError::Validation("child_index is out of range".into())))
            .transpose()?;

        // An explicit derivation path is a direct child target. If no
        // child_index is supplied, infer the logical index from the final
        // path component. All explicit paths must target the same index.
        // When child_index is supplied it is authoritative: selected chain
        // paths are templates and their final component is rewritten to this
        // index. When no index is supplied, a single consistent explicit path
        // index may identify the child directly.
        let path_index = if explicit_index.is_none() {
            let path_indices = chains.iter()
                .filter_map(|chain| chain.derivation_path.as_deref())
                .map(WalletRegistry::child_index_from_path)
                .collect::<Result<Vec<_>, _>>()?;
            if path_indices.iter().any(|index| Some(*index) != path_indices.first().copied()) {
                return Err(EngineError::Validation(
                    "all explicit derivation paths must target the same child index".into()
                ));
            }
            path_indices.first().copied()
        } else {
            None
        };
        let child_index = explicit_index.or(path_index);

        let password = params.get("pin").or_else(|| params.get("password")).and_then(|v| v.as_str())
            .ok_or_else(|| EngineError::Validation("wallet PIN/password is required to create a child wallet".into()))?;
        // The PIN/password is consumed only by the trusted registry boundary to
        // decrypt the parent group's encrypted seed. It is never sent to adapters.
        let record = WalletRegistry::create_child_at_index_with_session(&parent_wallet_id, child_index, name, &chains, Some(password))?;

        // Child creation activates the new child in the registry. Keep this
        // engine instance aligned with that authoritative session context.
        self.wallet_id = record.wallet_id.clone();
        self.mnemonic.clear();
        self.seed.clear();
        self.private_key = None;
        self.wallet_service = WalletService::new("")?;

        Ok(WalletResponse::ok(json!({
            "wallet_id": record.wallet_group_id,
            "child_id": record.wallet_id,
            "child_index": record.child_index,
            "wallet": record
        })))
    }

    fn handle_switch_child(&mut self, params: Value) -> Result<WalletResponse, EngineError> {
        let requested_group = params.get("parent_wallet_id")
            .or_else(|| params.get("wallet_group_id"))
            .or_else(|| params.get("wallet_id"))
            .and_then(Value::as_str);
        let requested_child_id = params.get("child_id").and_then(Value::as_str);
        let requested_index = params.get("child_index").or_else(|| params.get("index"))
            .and_then(Value::as_u64)
            .map(|value| u32::try_from(value).map_err(|_| EngineError::Validation("child_index is out of range".into())))
            .transpose()?;
        let password = params.get("pin").or_else(|| params.get("password")).and_then(Value::as_str);

        // Child selection is scoped to one master group. An explicit group wins;
        // otherwise use the active group. Never switch to a child in another group.
        let group_id = match requested_group {
            Some(id) => id.to_string(),
            None => WalletOrchestrator::active_group_id()?
                .ok_or_else(|| EngineError::Validation("no active wallet group; create or switch a master wallet first".into()))?,
        };
        let group = WalletRegistry::groups()?.into_iter()
            .find(|group| group.wallet_id == group_id || group.secret_group_id == group_id)
            .ok_or_else(|| EngineError::Validation(format!("WALLET_GROUP_NOT_FOUND: wallet group '{}' does not exist", group_id)))?;

        let child = if let Some(child_id) = requested_child_id {
            group.wallets.iter().find(|child| child.wallet_id == child_id).cloned()
                .ok_or_else(|| EngineError::Validation(format!(
                    "CHILD_NOT_IN_GROUP: child '{}' does not belong to group '{}'",
                    child_id, group.wallet_id
                )))?
        } else if let Some(index) = requested_index {
            group.wallets.iter().find(|child| child.child_index == index).cloned()
                .ok_or_else(|| EngineError::Validation(format!(
                    "CHILD_WALLET_NOT_FOUND: child index {} does not exist in wallet group '{}'; call create_child_wallet first",
                    index, group.wallet_id
                )))?
        } else {
            return Err(EngineError::Validation("switch_child_wallet requires child_id or child_index; optionally provide wallet_id for the parent group".into()));
        };

        // Re-check parent identity before mutating active selection.
        if child.wallet_group_id != group.wallet_id {
            return Err(EngineError::Validation("CHILD_NOT_IN_GROUP: requested child is outside the selected master group".into()));
        }
        if let Some(password) = password {
            WalletRegistry::switch(&child.wallet_id, password)?;
        } else {
            WalletRegistry::switch_child(&child.wallet_id)?;
        }
        self.wallet_id = child.wallet_id.clone();
        self.mnemonic.clear();
        self.seed.clear();
        self.private_key = None;
        self.wallet_service = WalletService::new("")?;
        let active_child = WalletRegistry::get(&child.wallet_id)?;
        Ok(WalletResponse::ok(json!({
            "wallet_id": active_child.wallet_group_id,
            "child_id": active_child.wallet_id,
            "child_index": active_child.child_index,
            "switched": true,
            "unlocked": password.is_some()
        })))
    }

    fn handle_wallet_choices(&self, params: Value) -> Result<WalletResponse, EngineError> {
        let chains = Self::parse_chains(&params)?;
        let choices = WalletRegistry::choices_for_chains(&chains)?;
        Ok(WalletResponse::ok(json!({
            "requires_selection": choices.len() > 1,
            "wallets": choices
        })))
    }

    fn handle_switch_wallet(&mut self, params: Value) -> Result<WalletResponse, EngineError> {
        let wallet_id = params.get("wallet_id").or_else(|| params.get("wallet_group_id"))
            .and_then(Value::as_str).map(str::trim).filter(|id| !id.is_empty())
            .ok_or_else(|| EngineError::Validation("switch_wallet requires wallet_id (master group ID)".into()))?;
        let group = WalletRegistry::groups()?.into_iter()
            .find(|group| group.wallet_id == wallet_id || group.secret_group_id == wallet_id)
            .ok_or_else(|| EngineError::Validation(format!(
                "WALLET_GROUP_NOT_FOUND: wallet group '{}' does not exist; create it before switching", wallet_id
            )))?;
        let root_child = group.wallets.iter().find(|child| child.child_index == 0)
            .ok_or_else(|| EngineError::Validation(format!(
                "WALLET_GROUP_INVALID: wallet group '{}' has no Child 0", group.wallet_id
            )))?.clone();
        let password = params.get("pin").or_else(|| params.get("password")).and_then(Value::as_str);
        let active_child = if let Some(password) = password {
            WalletRegistry::switch(&root_child.wallet_id, password)?;
            WalletRegistry::get(&root_child.wallet_id)?
        } else {
            WalletRegistry::switch_child(&root_child.wallet_id)?
        };
        self.wallet_id = active_child.wallet_id.clone();
        self.mnemonic.clear();
        self.seed.clear();
        self.private_key = None;
        self.wallet_service = WalletService::new("")?;
        Ok(WalletResponse::ok(json!({
            "wallet_id": active_child.wallet_group_id,
            "child_id": active_child.wallet_id,
            "child_index": active_child.child_index,
            "switched": true,
            "unlocked": password.is_some()
        })))
    }

    fn handle_wallet_count(&self, params: Value) -> Result<WalletResponse, EngineError> {
        let group_id = params.get("secret_group_id")
            .or_else(|| params.get("wallet_group_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| EngineError::Validation("missing secret_group_id".into()))?;
        let count = WalletRegistry::wallet_count(group_id)?;
        Ok(WalletResponse::ok(json!({
            "secret_group_id": group_id,
            "wallet_count": count
        })))
    }

    fn handle_wallet_group(&self, params: Value) -> Result<WalletResponse, EngineError> {
        let group_id = params.get("secret_group_id")
            .or_else(|| params.get("wallet_group_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| EngineError::Validation("missing secret_group_id".into()))?;
        Ok(WalletResponse::ok(json!({ "group": WalletRegistry::list_group(group_id)? })))
    }

    fn handle_wallet_groups(&self) -> Result<WalletResponse, EngineError> {
        Ok(WalletResponse::ok(json!({ "groups": WalletRegistry::groups()? })))
    }

    fn handle_list_wallets(&self) -> Result<WalletResponse, EngineError> {
        Ok(WalletResponse::ok(json!({ "wallets": WalletRegistry::groups()? })))
    }

    fn handle_active_wallet(&self) -> Result<WalletResponse, EngineError> {
        Ok(WalletResponse::ok(json!({
            "wallet_id": WalletOrchestrator::active_group_id()?,
            "child_id": WalletOrchestrator::active_child_id()?,
            "child": WalletRegistry::active()?
        })))
    }

    fn handle_add_blockchain(&mut self, params: Value) -> Result<WalletResponse, EngineError> {
        let wallet = self.resolve_wallet_record(&params)?;
        let chain: ChainRequest = serde_json::from_value(params.clone())
            .map_err(|e| EngineError::Validation(format!("invalid blockchain: {e}")))?;
        let pin = params.get("pin").or_else(|| params.get("password")).and_then(|v| v.as_str())
            .ok_or_else(|| EngineError::Validation("wallet PIN/password is required".into()))?;
        let address = if wallet.wallet_type == crate::services::wallet_registry::WalletType::PrivateKey {
            if !chain.chain_type.eq_ignore_ascii_case("evm") {
                return Err(EngineError::Validation("private-key wallets can only add EVM blockchains".into()));
            }
            let key = WalletRegistry::wallet_private_key(&wallet.wallet_id, pin)?;
            AdapterRegistry::address_from_private_key(&chain.chain_type, &key)?
        } else {
            WalletRegistry::derive_address_for_child(&wallet.wallet_id, &chain, pin)?
        };
        let record = WalletRegistry::add_blockchain(&wallet.wallet_id, chain, address)?;
        Ok(WalletResponse::ok(json!({ "wallet": record })))
    }

    fn handle_derive_addresses(&self, params: Value) -> Result<WalletResponse, EngineError> {
        if params.get("chain_type").is_some() {
            return self.handle_adapter_derivation("derive_addresses", params);
        }

        // Backward-compatible multi-chain request. New callers should use the
        // uniform adapter contract: { chain_type, start_index, count }.
        let chains = Self::parse_chains(&params)?;
        let seed = self.resolve_seed_for_request(&params)?;
        let addresses = AddressDerivationService::derive_all_addresses(&seed, &chains)?;
        Ok(WalletResponse::ok(json!({ "addresses": addresses })))
    }

    fn handle_adapter_derivation(&self, action: &str, params: Value) -> Result<WalletResponse, EngineError> {
        let seed = self.resolve_seed_for_request(&params)?;
        let result = AdapterOrchestrator::with_default_rpc()?.derive(&seed, action, &params)?;
        Ok(WalletResponse::ok(result))
    }

    fn resolve_wallet_record(
        &self,
        params: &Value,
    ) -> Result<crate::services::wallet::WalletRecord, EngineError> {
        let group_id = params.get("wallet_group_id").or_else(|| params.get("wallet_id")).and_then(Value::as_str);
        let child_id = params.get("child_id").and_then(Value::as_str);

        // Session-first resolution: absent explicit targets, use the registry's
        // active child. A group-only target resolves to that group's active child
        // if it is the active group, otherwise Child 0. Never use a stale engine ID
        // as an implicit wallet-selection fallback.
        let wallet = if let Some(child_id) = child_id {
            WalletRegistry::get(child_id)?
        } else if let Some(target_id) = group_id {
            match WalletRegistry::groups()?.into_iter()
                .find(|group| group.wallet_id == target_id || group.secret_group_id == target_id) {
                Some(group) => {
                    let active_child_id = WalletOrchestrator::active_child_id()?;
                    if let Some(active) = active_child_id.and_then(|id| group.wallets.iter().find(|child| child.wallet_id == id).cloned()) {
                        active
                    } else {
                        group.wallets.iter().find(|child| child.child_index == 0).cloned()
                            .ok_or_else(|| EngineError::Validation(format!("WALLET_GROUP_INVALID: group '{}' has no Child 0", target_id)))?
                    }
                }
                None => WalletRegistry::get(target_id)?,
            }
        } else if let Some(active_child_id) = WalletOrchestrator::active_child_id()? {
            WalletRegistry::get(&active_child_id)?
        } else {
            return Err(EngineError::Validation("wallet_id, child_id, or an active wallet session is required".into()));
        };

        if let (Some(group_id), Some(_)) = (group_id, child_id) {
            if wallet.wallet_group_id != group_id && wallet.wallet_id != group_id && wallet.secret_group_id != group_id {
                return Err(EngineError::Validation(format!(
                    "CHILD_WALLET_MISMATCH: child '{}' does not belong to wallet group '{}'",
                    wallet.wallet_id, group_id
                )));
            }
        }
        Ok(wallet)
    }

    fn resolve_seed_for_request(&self, params: &Value) -> Result<Vec<u8>, EngineError> {
        let explicit_target = params.get("wallet_group_id").and_then(Value::as_str)
            .or_else(|| params.get("wallet_id").and_then(Value::as_str))
            .or_else(|| params.get("child_id").and_then(Value::as_str));
        let password = params.get("pin").or_else(|| params.get("password")).and_then(Value::as_str);

        // A registered wallet is always authoritative. Never silently fall
        // back to a stale engine seed when a named wallet is missing or locked.
        match self.resolve_wallet_record(params) {
            Ok(wallet) => {
                if wallet.wallet_type != crate::services::wallet_registry::WalletType::Hd {
                    return Err(EngineError::Validation(
                        "seed-based derivation is not available for private-key wallets".into()
                    ));
                }
                if let Some(password) = password {
                    Ok(WalletRegistry::wallet_secret(&wallet.wallet_id, password)?.1)
                } else {
                    WalletRegistry::unlocked_wallet_secret(&wallet.wallet_id)
                        .map(|(_, seed)| seed)
                        .map_err(|_| EngineError::Validation(
                            "wallet is locked; provide pin/password to authorize derivation".into()
                        ))
                }
            }
            Err(error) if explicit_target.is_some() => Err(error),
            Err(_) if self.wallet_id.trim().is_empty() => Ok(self.seed.clone()),
            Err(error) => Err(error),
        }
    }

    fn handle_get_address(&self, params: Value) -> Result<WalletResponse, EngineError> {
        let wallet = self.resolve_wallet_record(&params)?;
        let chain_id = params.get("chain_id").and_then(Value::as_str);
        let chain = params.get("chain")
            .or_else(|| params.get("chain_type"))
            .or_else(|| params.get("chain_name"))
            .and_then(Value::as_str)
            .or(chain_id)
            .ok_or_else(|| EngineError::Validation("chain, chain_type, or chain_id is required".into()))?;

        let matches: Vec<_> = wallet.blockchains.iter().filter(|blockchain| {
            let chain_matches = blockchain.name.eq_ignore_ascii_case(chain)
                || blockchain.chain_type.eq_ignore_ascii_case(chain)
                || blockchain.symbol.as_deref().map(|s| s.eq_ignore_ascii_case(chain)).unwrap_or(false)
                || blockchain.chain_id.eq_ignore_ascii_case(chain);
            chain_matches && chain_id.map(|id| blockchain.chain_id.eq_ignore_ascii_case(id)).unwrap_or(true)
        }).collect();

        if matches.is_empty() {
            return Err(EngineError::Validation(format!(
                "ADDRESS_NOT_REGISTERED: no registered address for chain '{}' in wallet '{}'",
                chain, wallet.wallet_id
            )));
        }
        if matches.len() > 1 {
            return Err(EngineError::Validation(
                "chain selection is ambiguous; specify chain_id to identify the network".into()
            ));
        }

        let blockchain = matches[0];
        Ok(WalletResponse::ok(json!({
            "wallet_id": wallet.wallet_group_id,
            "child_id": wallet.wallet_id,
            "child_index": wallet.child_index,
            "chain": blockchain.name,
            "chain_type": blockchain.chain_type,
            "chain_id": blockchain.chain_id,
            "address": blockchain.address,
            "derivation_path": blockchain.derivation_path
        })))
    }

    /// Resolve a specific registered child and return its cached network
    /// addresses and token metadata. This never derives addresses or unlocks secrets.
    fn handle_get_wallet_context(&self, params: Value) -> Result<WalletResponse, EngineError> {
        let wallet = self.resolve_wallet_record(&params)?;
        let requested_chain_id = params.get("chain_id").and_then(Value::as_str);
        let requested_chain = params.get("chain").or_else(|| params.get("chain_type"))
            .or_else(|| params.get("chain_name")).and_then(Value::as_str);
        let chains: Vec<_> = wallet.blockchains.iter().filter(|chain| {
            requested_chain_id.map(|id| chain.chain_id.eq_ignore_ascii_case(id)).unwrap_or(true)
                && requested_chain.map(|name| {
                    chain.name.eq_ignore_ascii_case(name)
                        || chain.chain_type.eq_ignore_ascii_case(name)
                        || chain.symbol.as_deref().map(|s| s.eq_ignore_ascii_case(name)).unwrap_or(false)
                }).unwrap_or(true)
        }).map(|chain| json!({
            "name": chain.name,
            "symbol": chain.symbol,
            "chain_type": chain.chain_type,
            "chain_id": chain.chain_id,
            "address": chain.address,
            "derivation_path": chain.derivation_path,
            "rpc_provider": chain.rpc_provider,
            "rpc_url": chain.rpc_url,
            "balance": chain.balance,
            "tokens": chain.tokens
        })).collect();
        if (requested_chain_id.is_some() || requested_chain.is_some()) && chains.is_empty() {
            return Err(EngineError::Validation(format!(
                "CHAIN_NOT_REGISTERED: requested network is not configured on child '{}'",
                wallet.wallet_id
            )));
        }
        Ok(WalletResponse::ok(json!({
            "wallet_id": wallet.wallet_group_id,
            "child_id": wallet.wallet_id,
            "child_index": wallet.child_index,
            "wallet_type": wallet.wallet_type,
            "chains": chains
        })))
    }

    fn handle_get_mnemonic(&self, params: Value) -> Result<WalletResponse, EngineError> {
        let wallet = self.resolve_wallet_record(&params)?;
        let password = params.get("pin").or_else(|| params.get("password"))
            .and_then(Value::as_str)
            .ok_or_else(|| EngineError::Validation("wallet PIN/password is required to retrieve mnemonic".into()))?;
        let (mnemonic, _) = WalletRegistry::wallet_secret(&wallet.wallet_id, password)?;
        Ok(WalletResponse::ok(json!({
            "wallet_id": wallet.wallet_group_id,
            "child_id": wallet.wallet_id,
            "mnemonic": mnemonic
        })))
    }

    fn handle_add_token(&self, params: Value) -> Result<WalletResponse, EngineError> {
        let wallet = self.resolve_wallet_record(&params)?;
        let chain_id = params.get("chain_id").and_then(Value::as_str)
            .ok_or_else(|| EngineError::Validation("chain_id is required to add token metadata".into()))?;
        let name = params.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        let symbol = params.get("symbol").and_then(Value::as_str)
            .ok_or_else(|| EngineError::Validation("token symbol is required".into()))?.to_string();
        let contract_address = params.get("contract_address").or_else(|| params.get("token_address"))
            .and_then(Value::as_str)
            .ok_or_else(|| EngineError::Validation("token contract_address is required".into()))?.to_string();
        let decimals = params.get("decimals").and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
            .ok_or_else(|| EngineError::Validation("token decimals must be a non-negative integer".into()))?;
        let balance = params.get("balance").and_then(Value::as_str).map(str::to_owned);
        let token_contract = contract_address.clone();
        let token = crate::services::wallet::registry::WalletToken {
            name: if name.trim().is_empty() { symbol.clone() } else { name },
            symbol,
            decimals,
            contract_address,
            balance,
        };
        let updated = WalletRegistry::add_token(&wallet.wallet_id, chain_id, token)?;
        let chain = updated.blockchains.iter()
            .find(|b| b.chain_id.eq_ignore_ascii_case(chain_id))
            .ok_or_else(|| EngineError::Unexpected("token chain disappeared after update".into()))?;
        let token = chain.tokens.iter()
            .find(|t| t.contract_address.eq_ignore_ascii_case(&token_contract))
            .cloned();
        Ok(WalletResponse::ok(json!({
            "wallet_id": updated.wallet_group_id,
            "child_id": updated.wallet_id,
            "chain_id": chain.chain_id,
            "token": token,
            "persisted_in_registry": true,
            "note": "Call export_wallet_registry and persist its snapshot in host storage to survive process restart."
        })))
    }

    fn handle_get_token(&self, params: Value) -> Result<WalletResponse, EngineError> {
        let wallet = self.resolve_wallet_record(&params)?;
        let chain_id = params.get("chain_id").and_then(Value::as_str)
            .ok_or_else(|| EngineError::Validation("chain_id is required for token metadata lookup".into()))?;
        let token_query = params.get("contract_address")
            .or_else(|| params.get("token_address"))
            .or_else(|| params.get("symbol"))
            .or_else(|| params.get("token"))
            .and_then(Value::as_str)
            .ok_or_else(|| EngineError::Validation("token symbol or contract_address is required".into()))?;

        let blockchain = wallet.blockchains.iter()
            .find(|blockchain| blockchain.chain_id.eq_ignore_ascii_case(chain_id))
            .ok_or_else(|| EngineError::Validation(format!(
                "chain '{}' is not registered on wallet '{}'", chain_id, wallet.wallet_id
            )))?;
        let token = blockchain.tokens.iter().find(|token| {
            token.contract_address.eq_ignore_ascii_case(token_query)
                || token.symbol.eq_ignore_ascii_case(token_query)
                || token.name.eq_ignore_ascii_case(token_query)
        }).ok_or_else(|| EngineError::Validation(format!(
            "token metadata not found for '{}' on chain '{}'", token_query, chain_id
        )))?;

        Ok(WalletResponse::ok(json!({
            "wallet_id": wallet.wallet_group_id,
            "child_id": wallet.wallet_id,
            "chain_id": blockchain.chain_id,
            "chain": blockchain.name,
            "token": token
        })))
    }

    fn handle_backup_wallet(&self, params: Value) -> Result<WalletResponse, EngineError> {
        if params.get("wallet_id").is_some()
            || params.get("wallet_group_id").is_some()
            || params.get("child_id").is_some()
            || WalletOrchestrator::active_child_id()?.is_some()
        {
            let wallet = self.resolve_wallet_record(&params)?;
            let password = params.get("pin").or_else(|| params.get("password"))
                .and_then(Value::as_str)
                .ok_or_else(|| EngineError::Validation("wallet PIN/password is required for backup".into()))?;
            let (mnemonic, _) = WalletRegistry::wallet_secret(&wallet.wallet_id, password)?;
            return Ok(WalletResponse::ok(json!({
                "wallet_id": wallet.wallet_group_id,
                "child_id": wallet.wallet_id,
                "backup": format!("backup:{}", mnemonic)
            })));
        }
        let backup = self.backup_wallet()?;
        Ok(WalletResponse::ok(json!({ "backup": backup })))
    }

    fn handle_restore_wallet(&mut self, params: Value) -> Result<WalletResponse, EngineError> {
        let mnemonic = params
            .get("mnemonic")
            .and_then(|value| value.as_str())
            .ok_or_else(|| EngineError::Validation("missing mnemonic".into()))?;
        let state = WalletCreateService::initialize(mnemonic, None)?;
        self.mnemonic = state.mnemonic;
        self.seed = state.seed;
        self.wallet_service.restore_wallet(&self.mnemonic)?;
        Ok(WalletResponse::ok(json!({ "restored": true })))
    }

    fn handle_encrypt_vault(&self, params: Value) -> Result<WalletResponse, EngineError> {
        let password = params
            .get("password")
            .and_then(|value| value.as_str())
            .ok_or_else(|| EngineError::Validation("missing password".into()))?;
        let mnemonic = if params.get("wallet_id").is_some()
            || params.get("wallet_group_id").is_some()
            || params.get("child_id").is_some()
            || WalletOrchestrator::active_child_id()?.is_some()
        {
            let wallet = self.resolve_wallet_record(&params)?;
            WalletRegistry::wallet_secret(&wallet.wallet_id, password)?.0
        } else {
            self.mnemonic.clone()
        };
        let encrypted = EncryptionService::encrypt(&mnemonic, password)?;
        Ok(WalletResponse::ok(json!({ "encrypted_vault": encrypted })))
    }

    fn handle_decrypt_vault(&self, params: Value) -> Result<WalletResponse, EngineError> {
        let password = params
            .get("password")
            .and_then(|value| value.as_str())
            .ok_or_else(|| EngineError::Validation("missing password".into()))?;
        let encrypted_vault = params
            .get("encrypted_vault")
            .and_then(|value| value.as_str())
            .ok_or_else(|| EngineError::Validation("missing encrypted_vault".into()))?;
        let mnemonic = Self::decrypt_vault(encrypted_vault, password)?;
        Ok(WalletResponse::ok(json!({ "mnemonic": mnemonic })))
    }

    fn handle_sign_transaction(&self, params: Value) -> Result<WalletResponse, EngineError> {
        let tx_data = Self::parse_tx_data(&params)?;
        let chain_type = params
            .get("chain_type")
            .and_then(Value::as_str)
            .ok_or_else(|| EngineError::Validation("missing chain_type".into()))?;

        let explicit_target = params.get("wallet_group_id").and_then(Value::as_str)
            .or_else(|| params.get("wallet_id").and_then(Value::as_str))
            .or_else(|| params.get("child_id").and_then(Value::as_str));
        let password = params.get("pin").or_else(|| params.get("password")).and_then(Value::as_str);

        let signature = match self.resolve_wallet_record(&params) {
            Ok(wallet) => {
                let password = password.ok_or_else(|| EngineError::Validation(
                    "wallet PIN/password is required to sign a transaction".into()
                ))?;
                let chain_id = params.get("chain_id").and_then(Value::as_str);
                let (key_selector, signing_chain_type) = if let Some(chain_id) = chain_id {
                    let configured = wallet.blockchains.iter()
                        .find(|chain| chain.chain_id.eq_ignore_ascii_case(chain_id))
                        .ok_or_else(|| EngineError::Validation(format!(
                            "CHAIN_NOT_REGISTERED: child '{}' has no configured chain_id '{}'",
                            wallet.wallet_id, chain_id
                        )))?;
                    (chain_id, configured.chain_type.as_str())
                } else {
                    (chain_type, chain_type)
                };
                // PIN stays inside the trusted wallet engine. Only the resolved
                // signing key reaches the signing service; it is never broadcast
                // to adapters. The key path is taken from the selected child record.
                let key = WalletOrchestrator::signing_key(&wallet.wallet_id, key_selector, password)?;
                TransactionService::sign_transaction(signing_chain_type, &key, &tx_data)?
            }
            Err(error) if explicit_target.is_some() => return Err(error),
            Err(_) if self.wallet_id.trim().is_empty() => {
                self.sign_transaction_by_chain(chain_type, &tx_data)?
            }
            Err(error) => return Err(error),
        };
        Ok(WalletResponse::ok(json!({ "signature_hex": hex::encode(signature) })))
    }

    fn handle_derive_address_from_path(&self, params: Value) -> Result<WalletResponse, EngineError> {
        self.handle_adapter_derivation("derive_address", params)
    }

    fn parse_chains(params: &Value) -> Result<Vec<ChainRequest>, EngineError> {
        let value = params
            .get("chains")
            .ok_or_else(|| EngineError::Validation("missing chains".into()))?;
        serde_json::from_value(value.clone())
            .map_err(|e| EngineError::Validation(format!("invalid chains: {e}")))
    }

    fn parse_tx_data(params: &Value) -> Result<Vec<u8>, EngineError> {
        if let Some(hex_str) = params.get("tx_data_hex").and_then(|value| value.as_str()) {
            hex::decode(hex_str).map_err(|error| EngineError::Validation(format!("invalid tx_data_hex: {}", error)))
        } else if let Some(raw) = params.get("tx_data").and_then(|value| value.as_str()) {
            Ok(raw.as_bytes().to_vec())
        } else {
            Err(EngineError::Validation("missing tx_data or tx_data_hex".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WalletProvider, WalletRequest};
    use crate::crypto::mnemonic::generate_mnemonic;

    fn provider() -> WalletProvider {
        let mnemonic = generate_mnemonic(12).expect("mnemonic should generate");
        WalletProvider::new(&mnemonic).expect("provider should initialize")
    }

    #[test]
    fn rpc_environment_keys_are_stable_and_safe() {
        assert_eq!(super::rpc_env_key_part("BNB Smart Chain"), "BNB_SMART_CHAIN");
        assert_eq!(super::rpc_env_key_part("ethereum"), "ETHEREUM");
        assert_eq!(super::rpc_env_key_part("cosmoshub-4"), "COSMOSHUB_4");
    }

    #[test]
    fn provider_dispatches_uniform_tron_derivation_contract() {
        let mut provider = provider();
        let request = WalletRequest {
            action: "derive_address".into(),
            params: serde_json::json!({
                "chain_type": "tron",
                "index": 1
            }),
        };

        let response = provider.handle_request(request).expect("TRON derivation request should succeed");
        assert!(response.success);
        let address = response
            .data
            .and_then(|data| data.get("address").and_then(|value| value.as_str()).map(str::to_owned))
            .expect("TRON derivation should return an address");
        assert_eq!(address.len(), 34);
        assert!(address.starts_with('T'));
    }

    #[test]
    fn provider_dispatches_same_contract_for_explicit_tron_path() {
        let mut provider = provider();
        let request = WalletRequest {
            action: "derive_address".into(),
            params: serde_json::json!({
                "chain_type": "tron",
                "path": "m/44'/195'/0'/0/1"
            }),
        };

        let response = provider.handle_request(request).expect("TRON path derivation should succeed");
        assert!(response.success);
        assert!(response.data.unwrap().get("address").and_then(|value| value.as_str()).is_some());
    }

    #[test]
    fn provider_dispatches_generic_chain_requests() {
        let mut provider = provider();
        let request = WalletRequest {
            action: "derive_addresses".into(),
            params: serde_json::json!({
                "chains": [
                    { "name": "Ethereum", "symbol": "ETH", "chain_type": "evm", "chain_id": "1" },
                    { "name": "Solana", "symbol": "SOL", "chain_type": "solana", "chain_id": "solana", "derivation_path": "m/44'/501'/0'/0'" }
                ]
            }),
        };
        let response = provider.handle_request(request).expect("request handler should succeed");
        assert!(response.success);
        assert!(response.data.is_some());
    }

    #[test]
    fn registered_wallet_address_lookup_does_not_require_secret_material() {
        let mut provider = provider();
        let create = WalletRequest {
            action: "create_wallet".into(),
            params: serde_json::json!({
                "word_count": 12,
                "password": "registry-test-password",
                "chains": [
                    { "name": "Ethereum", "symbol": "ETH", "chain_type": "evm", "chain_id": "1", "derivation_path": "m/44'/60'/0'/0/0" }
                ]
            }),
        };
        let created = provider.handle_request(create).expect("wallet should be created");
        let data = created.data.expect("creation data");
        let wallet_id = data.get("wallet_id").and_then(|v| v.as_str()).expect("group id").to_string();
        let response = provider.handle_request(WalletRequest {
            action: "get_address".into(),
            params: serde_json::json!({
                "wallet_id": wallet_id,
                "chain_id": "1",
                "chain": "ethereum"
            }),
        }).expect("registered address lookup should succeed");
        assert!(response.success);
        assert!(response.data.unwrap().get("address").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn child_creation_uses_requested_index_and_persists_its_address() {
        let mut provider = provider();
        let created = provider.handle_request(WalletRequest {
            action: "create_wallet".into(),
            params: serde_json::json!({
                "word_count": 12,
                "password": "child-index-test-password",
                "chains": [
                    { "name": "Ethereum", "symbol": "ETH", "chain_type": "evm", "chain_id": "1", "derivation_path": "m/44'/60'/0'/0/0" }
                ]
            }),
        }).expect("group wallet should be created");
        let root = created.data.expect("root wallet response");
        let group_id = root.get("wallet_id").and_then(|v| v.as_str()).expect("group ID").to_string();

        let child = provider.handle_request(WalletRequest {
            action: "create_child_wallet".into(),
            params: serde_json::json!({
                "wallet_id": group_id,
                "child_index": 3,
                "pin": "child-index-test-password",
                "chains": [
                    { "name": "Ethereum", "symbol": "ETH", "chain_type": "evm", "chain_id": "1", "derivation_path": "m/44'/60'/0'/0/0" }
                ]
            }),
        }).expect("child wallet should be created");
        let child_data = child.data.expect("child response");
        assert_eq!(child_data.get("child_index").and_then(|v| v.as_u64()), Some(3));
        let child_id = child_data.get("child_id").and_then(|v| v.as_str()).expect("child ID").to_string();

        let context = provider.handle_request(WalletRequest {
            action: "get_child_context".into(),
            params: serde_json::json!({ "child_id": child_id, "chain_id": "1" }),
        }).expect("child context should be available");
        let context_data = context.data.expect("context response");
        let chain = context_data.get("chains").and_then(|v| v.as_array())
            .and_then(|chains| chains.first()).expect("saved chain");
        assert_eq!(chain.get("derivation_path").and_then(|v| v.as_str()), Some("m/44'/60'/0'/0/3"));
        assert!(chain.get("address").and_then(|v| v.as_str()).is_some());
    }

    #[test]
    fn switching_to_missing_child_creates_only_that_child_and_activates_it() {
        let mut provider = provider();
        let created = provider.handle_request(WalletRequest {
            action: "create_wallet_group".into(),
            params: serde_json::json!({
                "word_count": 12,
                "pin": "switch-child-test-password",
                "chains": [
                    { "name": "Ethereum", "symbol": "ETH", "chain_type": "evm", "chain_id": "1", "derivation_path": "m/44'/60'/0'/0/0" }
                ]
            }),
        }).expect("master group should be created");
        let root = created.data.expect("root response");
        let group_id = root.get("wallet_id").and_then(|v| v.as_str()).expect("group ID").to_string();

        let switched = provider.handle_request(WalletRequest {
            action: "switch_child_wallet".into(),
            params: serde_json::json!({
                "wallet_id": group_id,
                "child_index": 4,
                "pin": "switch-child-test-password"
            }),
        }).expect("switch should create missing child");
        let switched_data = switched.data.expect("switch response");
        assert_eq!(switched_data.get("child_index").and_then(|v| v.as_u64()), Some(4));
        assert_eq!(switched_data.get("created").and_then(|v| v.as_bool()), Some(true));
        let child_id = switched_data.get("child_id").and_then(|v| v.as_str()).expect("child ID").to_string();

        let context = provider.handle_request(WalletRequest {
            action: "get_wallet_context".into(),
            params: serde_json::json!({ "child_id": child_id, "chain_id": "1" }),
        }).expect("new child context should be available");
        let chain = context.data.expect("context data").get("chains").and_then(|v| v.as_array())
            .and_then(|chains| chains.first()).cloned().expect("saved child chain");
        assert_eq!(chain.get("derivation_path").and_then(|v| v.as_str()), Some("m/44'/60'/0'/0/4"));
        assert!(chain.get("address").and_then(|v| v.as_str()).is_some());

        let active = provider.handle_request(WalletRequest {
            action: "active_wallet".into(),
            params: serde_json::json!({}),
        }).expect("active wallet should be queryable");
        assert_eq!(
            active.data.as_ref().and_then(|data| data.get("child_id")).and_then(|v| v.as_str()),
            Some(child_id.as_str()),
            "switching to child index 4 must update the active child session"
        );

        let group = provider.handle_request(WalletRequest {
            action: "wallet_group".into(),
            params: serde_json::json!({ "wallet_group_id": group_id }),
        }).expect("group should be available");
        let group_data = group.data.expect("group data");
        let children = group_data.get("group").and_then(|g| g.get("wallets")).and_then(|v| v.as_array())
            .expect("group children");
        assert_eq!(children.iter().filter(|child| child.get("child_index").and_then(|v| v.as_u64()) == Some(4)).count(), 1);
        assert!(!children.iter().any(|child| child.get("child_index").and_then(|v| v.as_u64()) == Some(1)));
    }

    #[test]
    fn missing_master_group_cannot_be_switched_into_existence() {
        let mut provider = provider();
        let before = provider.handle_request(WalletRequest {
            action: "active_wallet".into(),
            params: serde_json::json!({}),
        }).expect("active wallet state should be readable").data;
        let result = provider.handle_request(WalletRequest {
            action: "switch_master_wallet".into(),
            params: serde_json::json!({ "wallet_id": "definitely-not-a-master-group" }),
        });
        assert!(result.is_err(), "switching a missing group must fail rather than create it");
        let after = provider.handle_request(WalletRequest {
            action: "active_wallet".into(),
            params: serde_json::json!({}),
        }).expect("active wallet state should remain readable").data;
        assert_eq!(after, before, "a failed master switch must not mutate active selection");
    }

    #[test]
    fn mnemonic_retrieval_requires_wallet_password() {
        let mut provider = provider();
        let result = provider.handle_request(WalletRequest {
            action: "get_mnemonic".into(),
            params: serde_json::json!({ "wallet_id": "not-a-wallet" }),
        });
        assert!(result.is_err(), "mnemonic retrieval without a password must be rejected");
    }

    #[tokio::test]
    async fn create_wallet_request_does_not_require_mnemonic() {
        let mut provider = provider();
        let request = WalletRequest {
            action: "create_wallet".into(),
            params: serde_json::json!({
                "word_count": 12,
                "password": "test-password",
                "chains": [
                    { "name": "Ethereum", "symbol": "ETH", "chain_type": "evm", "chain_id": "1", "derivation_path": "m/44'/60'/0'/0/0" }
                ]
            }),
        };
        let response = provider.handle_request(request).expect("create wallet should succeed");
        assert!(response.success);
        assert!(response.data.unwrap().get("mnemonic").is_some());
    }
}
