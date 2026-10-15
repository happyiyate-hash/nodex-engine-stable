pub mod event_service;
pub mod address_service;
pub mod registry;
pub mod rpc;
pub mod wallet;
pub mod wallet_create_service;
pub mod transaction;
pub mod balance_service;

// Compatibility exports for services that use the stable service names.
pub mod encryption_service { pub use crate::crypto::encryption::EncryptionService; }
pub mod address_derivation_service { pub use crate::crypto::address_derivation::AddressDerivationService; }
pub mod wallet_registry { pub use crate::services::wallet::registry::*; }
