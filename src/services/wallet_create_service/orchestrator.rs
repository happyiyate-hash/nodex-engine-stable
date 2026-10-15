use std::sync::Arc;

use crate::common::errors::EngineError;
use crate::services::balance_service::{BalanceFetchService, BalanceRequest, ChainBalance};
use crate::services::wallet_create_service::{WalletCreateResult, WalletCreateService};
use crate::types::chain::ChainRequest;

#[derive(Debug, Clone)]
pub struct WalletChainRuntime {
    pub chain: ChainRequest,
    pub provider: String,
    pub rpc_endpoint: String,
    pub assets: Vec<String>,
}

pub struct CreateWalletOrchestrator;

impl CreateWalletOrchestrator {
    pub fn create(word_count: usize, name: Option<&str>, pin: &str, chains: &[ChainRequest]) -> Result<WalletCreateResult, EngineError> {
        WalletCreateService::create_named(word_count, Some(pin), chains, name)
    }

    pub fn import(mnemonic: &str, passphrase: Option<&str>, name: Option<&str>, pin: &str, chains: &[ChainRequest]) -> Result<WalletCreateResult, EngineError> {
        WalletCreateService::import_named(mnemonic, passphrase, Some(pin), chains, name)
    }

    /// Creates/imports the wallet, derives the requested addresses, then immediately
    /// sends one balance request per configured chain through Balance Service.
    /// Balance providers do not hold API keys; they use the shared RPC orchestrator.
    pub async fn create_and_fetch_balances(
        word_count: usize,
        name: Option<&str>,
        pin: &str,
        runtime: &[WalletChainRuntime],
        balance_service: Arc<BalanceFetchService>,
    ) -> Result<(WalletCreateResult, Vec<Result<ChainBalance, crate::services::balance_service::BalanceError>>), EngineError> {
        let chains: Vec<ChainRequest> = runtime.iter().map(|r| r.chain.clone()).collect();
        let wallet = Self::create(word_count, name, pin, &chains)?;
        let requests = Self::balance_requests(&wallet, runtime)?;
        let mut receiver = balance_service.get_balance(requests).await;
        let mut balances = Vec::new();
        while let Some(result) = receiver.recv().await { balances.push(result); }
        Ok((wallet, balances))
    }

    /// Imports the wallet, derives the requested addresses, then immediately
    /// sends the balance requests through Balance Service.
    pub async fn import_and_fetch_balances(
        mnemonic: &str,
        passphrase: Option<&str>,
        name: Option<&str>,
        pin: &str,
        runtime: &[WalletChainRuntime],
        balance_service: Arc<BalanceFetchService>,
    ) -> Result<(WalletCreateResult, Vec<Result<ChainBalance, crate::services::balance_service::BalanceError>>), EngineError> {
        let chains: Vec<ChainRequest> = runtime.iter().map(|r| r.chain.clone()).collect();
        let wallet = Self::import(mnemonic, passphrase, name, pin, &chains)?;
        let requests = Self::balance_requests(&wallet, runtime)?;
        let mut receiver = balance_service.get_balance(requests).await;
        let mut balances = Vec::new();
        while let Some(result) = receiver.recv().await { balances.push(result); }
        Ok((wallet, balances))
    }

    fn balance_requests(
        wallet: &WalletCreateResult,
        runtime: &[WalletChainRuntime],
    ) -> Result<Vec<BalanceRequest>, EngineError> {
        let addresses = wallet.wallet.addresses.as_object()
            .ok_or_else(|| EngineError::Validation("derived addresses must be a JSON object".into()))?;

        runtime.iter().map(|item| {
            let address = addresses.get(&item.chain.name)
                .and_then(|v| v.as_str())
                .or_else(|| addresses.get(&item.chain.chain_id).and_then(|v| v.as_str()))
                .ok_or_else(|| EngineError::Validation(format!(
                    "no derived address found for chain {}",
                    item.chain.name
                )))?;

            if item.provider.trim().is_empty() {
                return Err(EngineError::Validation(format!("provider is required for {}", item.chain.name)));
            }
            if item.rpc_endpoint.trim().is_empty() {
                return Err(EngineError::Validation(format!("rpc_endpoint is required for {}", item.chain.name)));
            }

            Ok(BalanceRequest {
                wallet_id: wallet.wallet.wallet_id.clone(),
                provider: item.provider.clone(),
                rpc_endpoint: item.rpc_endpoint.clone(),
                chain: item.chain.clone(),
                address: address.to_owned(),
                assets: item.assets.clone(),
            })
        }).collect()
    }
}
