use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use serde::{Deserialize, Serialize};
use crate::services::balance_service::providers::{BalanceProvider, BalanceProviderRequest};
use crate::adapters::registry::AdapterRegistry;
use crate::services::balance_service::registry::BalanceProviderRegistry;
use crate::types::chain::ChainRequest;
use crate::services::rpc::RpcOrchestrator;
use crate::services::wallet::WalletRegistry;
use crate::common::errors::EngineError;
use crate::services::event_service::EventService;
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceAsset { pub address: String, pub symbol: Option<String>, pub name: Option<String>, pub decimals: Option<u32>, pub balance: String, pub is_native: bool }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainBalance { pub wallet_id: String, pub chain_id: String, pub chain_type: String, pub address: String, pub native: Option<BalanceAsset>, pub tokens: Vec<BalanceAsset> }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceError { pub wallet_id: String, pub chain_id: String, pub provider: String, pub message: String }

/// A per-chain result. A batch is successful at the transport level even when
/// one or more chains fail; each chain carries its own success/error state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceBatchResult {
    pub results: Vec<Result<ChainBalance, BalanceError>>,
}
#[derive(Debug, Clone)]
pub struct BalanceRequest {
    pub wallet_id: String,
    pub provider: String,
    pub rpc_endpoint: String,
    pub chain: ChainRequest,
    pub address: String,
    /// Empty means native balance only. Otherwise contains token contract addresses.
    pub assets: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum BalanceUpdateScope {
    All,
    Chain { chain_id: String, tokens: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct BalanceUpdateRequest {
    pub wallet_id: String,
    pub scope: BalanceUpdateScope,
    pub address: Option<String>,
}

/// Balance administrator. It routes requests to balance-specific providers.
/// Provider implementations own balance batching/grouping and use RPC underneath when needed.
pub struct BalanceFetchService { registry: BalanceProviderRegistry, rpc: Arc<RpcOrchestrator> }
impl BalanceFetchService {
    pub fn new(registry: BalanceProviderRegistry, rpc: Arc<RpcOrchestrator>) -> Self { Self { registry, rpc } }
    pub fn registry(&self) -> &BalanceProviderRegistry { &self.registry }
    /// Single public balance endpoint. Every balance operation uses the same
    /// streaming response path. One request may contain one or many chains.
        pub async fn get_balance(
        self: Arc<Self>,
        requests: Vec<BalanceRequest>,
    ) -> UnboundedReceiver<Result<ChainBalance, BalanceError>> {
        let (tx, rx) = mpsc::unbounded_channel();

        // One worker processes one blockchain at a time. Each completed chain
        // is emitted immediately as one grouped ChainBalance event (native +
        // tokens); no cross-chain fan-out or unbounded RPC batch is created.
        tokio::spawn(async move {
            for request in requests {
                let provider_name = request.provider.trim().to_ascii_lowercase();
                let chain_type = request.chain.normalized_type().to_ascii_lowercase();
                let wallet_id = request.wallet_id.clone();
                let chain_id = request.chain.chain_id.clone();
                let asset_count = request.assets.len();
                let started_at = std::time::Instant::now();

                EventService::publish("balance.fetch.started", json!({
                    "wallet_id": wallet_id,
                    "chain_id": chain_id,
                    "provider": provider_name,
                    "asset_count": asset_count
                }));

                let Some(provider) = self.registry.get(&provider_name) else {
                    let failure = BalanceError {
                        wallet_id: request.wallet_id.clone(),
                        chain_id: request.chain.chain_id.clone(),
                        provider: provider_name,
                        message: "no balance provider registered".into(),
                    };
                    EventService::publish("balance.fetch.failed", json!({
                        "wallet_id": failure.wallet_id,
                        "chain_id": failure.chain_id,
                        "provider": failure.provider,
                        "duration_ms": started_at.elapsed().as_millis(),
                        "message": failure.message
                    }));
                    let _ = tx.send(Err(failure));
                    continue;
                };

                // Exactly one chain is passed to the adapter. Chain-specific
                // RPC calls may be internally grouped, but another chain never
                // shares this invocation or its response.
                let wrapped = vec![BalanceProviderRequest {
                    request,
                    rpc: Arc::clone(&self.rpc),
                }];

                match AdapterRegistry::fetch_balance_batch(
                    &chain_type,
                    Arc::clone(&provider),
                    wrapped,
                ).await {
                    Ok(mut responses) if responses.len() == 1 => {
                        let response = responses.remove(0);
                        match response.result {
                            Ok(balance) => {
                                // One event contains one complete chain group,
                                // including its native balance and token list.
                                let payload = json!({
                                    "wallet_id": balance.wallet_id,
                                    "chain_id": balance.chain_id,
                                    "provider": provider_name,
                                    "duration_ms": started_at.elapsed().as_millis(),
                                    "balance": balance
                                });
                                EventService::publish("balance.result", payload);
                                let _ = tx.send(Ok(balance));
                            }
                            Err(error) => {
                                let failure = BalanceError {
                                    wallet_id: response.wallet_id,
                                    chain_id: response.chain_id,
                                    provider: provider_name,
                                    message: error.to_string(),
                                };
                                EventService::publish("balance.fetch.failed", json!({
                                    "wallet_id": failure.wallet_id,
                                    "chain_id": failure.chain_id,
                                    "provider": failure.provider,
                                    "duration_ms": started_at.elapsed().as_millis(),
                                    "message": failure.message
                                }));
                                let _ = tx.send(Err(failure));
                            }
                        }
                    }
                    Ok(responses) => {
                        let failure = BalanceError {
                            wallet_id,
                            chain_id,
                            provider: provider_name,
                            message: format!("adapter returned {} results for one blockchain request", responses.len()),
                        };
                        EventService::publish("balance.fetch.failed", json!({
                            "wallet_id": failure.wallet_id,
                            "chain_id": failure.chain_id,
                            "provider": failure.provider,
                            "duration_ms": started_at.elapsed().as_millis(),
                            "message": failure.message
                        }));
                        let _ = tx.send(Err(failure));
                    }
                    Err(error) => {
                        let failure = BalanceError {
                            wallet_id,
                            chain_id,
                            provider: provider_name,
                            message: error.to_string(),
                        };
                        EventService::publish("balance.fetch.failed", json!({
                            "wallet_id": failure.wallet_id,
                            "chain_id": failure.chain_id,
                            "provider": failure.provider,
                            "duration_ms": started_at.elapsed().as_millis(),
                            "message": failure.message
                        }));
                        let _ = tx.send(Err(failure));
                    }
                }
            }
        });

        rx
    }

    /// Returns the saved balances for the currently active wallet.
    /// This never contacts RPC.
    pub fn get_active_balance(chain_id: Option<&str>) -> Result<Vec<ChainBalance>, EngineError> {
        let wallet = WalletRegistry::active()?
            .ok_or_else(|| EngineError::Validation("no active wallet".into()))?;
        Self::get_saved_balance(&wallet.wallet_id, chain_id)
    }

    /// Returns the balances already persisted in the active/local wallet.
    /// This operation never contacts RPC and never invokes an adapter.
    pub fn get_saved_balance(
        wallet_id: &str,
        chain_id: Option<&str>,
    ) -> Result<Vec<ChainBalance>, EngineError> {
        let wallet = WalletRegistry::get(wallet_id)?;
        let mut results = Vec::new();

        for chain in wallet.blockchains {
            if let Some(requested) = chain_id {
                if chain.chain_id != requested {
                    continue;
                }
            }

            let native = chain.balance.as_ref().map(|balance| BalanceAsset {
                address: chain.address.clone(),
                symbol: chain.symbol.clone(),
                name: Some(chain.name.clone()),
                decimals: None,
                balance: balance.clone(),
                is_native: true,
            });

            let tokens = chain.tokens.into_iter().map(|token| BalanceAsset {
                address: token.contract_address,
                symbol: Some(token.symbol),
                name: Some(token.name),
                decimals: Some(token.decimals),
                balance: token.balance.unwrap_or_else(|| "0".into()),
                is_native: false,
            }).collect();

            results.push(ChainBalance {
                wallet_id: wallet.wallet_id.clone(),
                chain_id: chain.chain_id,
                chain_type: chain.chain_type,
                address: chain.address,
                native,
                tokens,
            });
        }

        Ok(results)
    }

    pub async fn update_balance(
        self: Arc<Self>,
        request: BalanceUpdateRequest,
        stored_chains: Vec<BalanceRequest>,
    ) -> UnboundedReceiver<Result<ChainBalance, BalanceError>> {
        let selected = match request.scope {
            BalanceUpdateScope::All => stored_chains,
            BalanceUpdateScope::Chain { chain_id, tokens } => {
                stored_chains.into_iter()
                    .filter(|item| item.chain.chain_id == chain_id)
                    .map(|mut item| {
                        if !tokens.is_empty() {
                            item.assets = tokens.clone();
                        }
                        if let Some(address) = request.address.clone() {
                            item.address = address;
                        }
                        item
                    })
                    .collect()
            }
        };

        self.get_balance(selected).await
    }

}
