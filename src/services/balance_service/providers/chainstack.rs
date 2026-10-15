use async_trait::async_trait;
use super::{BalanceProvider, BalanceProviderError, BalanceProviderRequest, BalanceProviderResponse};

pub struct ChainstackBalanceProvider;

#[async_trait]
impl BalanceProvider for ChainstackBalanceProvider {
    async fn fetch_batch(
        &self,
        _requests: Vec<BalanceProviderRequest>,
    ) -> Result<Vec<BalanceProviderResponse>, BalanceProviderError> {
        Err(BalanceProviderError::Message("chainstack balance provider is not implemented yet".into()))
    }
}
