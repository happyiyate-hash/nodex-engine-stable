use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalletEvent {
    WalletCreated { wallet_id: String },
    WalletImported { wallet_id: String },
    WalletEncrypted,
    WalletUnlocked { wallet_id: String },
    WalletDeleted { wallet_id: String },
    AddressGenerated { wallet_id: String, chain_id: String, address: String },
    BalanceFetchStarted { wallet_id: String, chain_id: String },
    BalanceUpdated { wallet_id: String, chain_id: String },
    BalanceFetchFailed { wallet_id: String, chain_id: String, message: String },
    TransactionSigned,
    SwapCompleted,
}
