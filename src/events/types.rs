//! Public event types shared by internal publishers and external subscribers.
pub use super::bus::EngineEvent;

/// Event names used by the first engine-wide event producers.
pub mod names {
    pub const WALLET_CREATED: &str = "wallet.created";
    pub const BALANCE_RESULT: &str = "balance.result";
    pub const BALANCE_UPDATED: &str = "balance.updated";
    pub const BALANCE_FETCH_FAILED: &str = "balance.fetch.failed";
    pub const BALANCE_REFRESH_COMPLETED: &str = "balance.refresh.completed";
}
