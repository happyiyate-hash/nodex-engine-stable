use async_trait::async_trait;
use super::{BalanceProvider, BalanceProviderError, BalanceProviderRequest, BalanceProviderResponse};

pub struct QuicknodeBalanceProvider;

#[async_trait]
impl BalanceProvider for QuicknodeBalanceProvider {
    async fn fetch_batch(
        &self,
        _requests: Vec<BalanceProviderRequest>,
    ) -> Result<Vec<BalanceProviderResponse>, BalanceProviderError> {
        Err(BalanceProviderError::Message("quicknode balance provider is not implemented yet".into()))
    }
}
