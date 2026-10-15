use async_trait::async_trait;
use super::{BalanceProvider, BalanceProviderError, BalanceProviderRequest, BalanceProviderResponse};

pub struct AnkrBalanceProvider;

#[async_trait]
impl BalanceProvider for AnkrBalanceProvider {
    async fn fetch_batch(
        &self,
        _requests: Vec<BalanceProviderRequest>,
    ) -> Result<Vec<BalanceProviderResponse>, BalanceProviderError> {
        Err(BalanceProviderError::Message("ankr balance provider is not implemented yet".into()))
    }
}
