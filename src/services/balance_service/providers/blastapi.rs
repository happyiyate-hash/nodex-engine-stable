use async_trait::async_trait;
use super::{BalanceProvider, BalanceProviderError, BalanceProviderRequest, BalanceProviderResponse};

pub struct BlastapiBalanceProvider;

#[async_trait]
impl BalanceProvider for BlastapiBalanceProvider {
    async fn fetch_batch(
        &self,
        _requests: Vec<BalanceProviderRequest>,
    ) -> Result<Vec<BalanceProviderResponse>, BalanceProviderError> {
        Err(BalanceProviderError::Message("blastapi balance provider is not implemented yet".into()))
    }
}
