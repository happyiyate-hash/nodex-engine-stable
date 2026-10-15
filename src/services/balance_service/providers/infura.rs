use async_trait::async_trait;

use super::{
    balance_method_config, fetch_evm_balances_batch, BalanceProvider, BalanceProviderError,
    BalanceProviderRequest, BalanceProviderResponse,
};

/// Infura-backed EVM balance provider.
///
/// This layer owns the *balance job*: it accepts the wallet/chain requests and
/// builds the native-balance and configured-token RPC calls. It deliberately
/// does not know Infura API keys or perform HTTP itself. The RPC layer owns
/// authentication, transport, retries and key rotation.
pub struct InfuraBalanceProvider;

impl InfuraBalanceProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BalanceProvider for InfuraBalanceProvider {
    async fn fetch_batch(
        &self,
        mut requests: Vec<BalanceProviderRequest>,
    ) -> Result<Vec<BalanceProviderResponse>, BalanceProviderError> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }

        // Infura's registered balance executor currently supports the
        // catalog's shared EVM method set. Never silently send a non-EVM chain
        // through eth_getBalance just because its provider was changed.
        for request in &mut requests {
            let config = balance_method_config(&request.request.chain);
            if config.as_ref().and_then(|value| value.get("executor")).and_then(serde_json::Value::as_str) != Some("evm") {
                return Err(BalanceProviderError::Message(format!(
                    "Infura cannot execute the configured balance method for {} ({})",
                    request.request.chain.name, request.request.chain.chain_id
                )));
            }
            request.request.provider = "infura".to_owned();

            if request.request.rpc_endpoint.trim().is_empty() {
                return Err(BalanceProviderError::Message(format!(
                    "missing Infura RPC endpoint for chain {}",
                    request.request.chain.chain_id
                )));
            }

            if request.request.address.trim().is_empty() {
                return Err(BalanceProviderError::Message(format!(
                    "missing wallet address for chain {}",
                    request.request.chain.chain_id
                )));
            }
        }

        // fetch_evm_balances_batch creates the JSON-RPC balance work:
        //   eth_getBalance(address, latest)
        //   eth_call(balanceOf(address)) for each configured token
        //   eth_call(decimals/symbol/name) for token metadata.
        //
        // It hands those RPC requests to RpcOrchestrator. The Infura RPC
        // provider then supplies the API key, performs HTTP transport, retries
        // failed calls and rotates through configured keys.
        fetch_evm_balances_batch(requests).await
    }
}
