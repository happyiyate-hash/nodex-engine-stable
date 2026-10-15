use crate::common::errors::EngineError;
use crate::events::{bus::EventBus, event::WalletEvent};
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};

const MAX_BUFFERED_EVENTS: usize = 256;

fn event_buffer() -> &'static Mutex<Vec<WalletEvent>> {
    static BUFFER: OnceLock<Mutex<Vec<WalletEvent>>> = OnceLock::new();
    BUFFER.get_or_init(|| Mutex::new(Vec::with_capacity(MAX_BUFFERED_EVENTS)))
}

fn event_type(event: &WalletEvent) -> &'static str {
    match event {
        WalletEvent::WalletCreated { .. } => "wallet.created",
        WalletEvent::WalletImported { .. } => "wallet.imported",
        WalletEvent::WalletEncrypted => "wallet.encrypted",
        WalletEvent::WalletUnlocked { .. } => "wallet.unlocked",
        WalletEvent::WalletDeleted { .. } => "wallet.deleted",
        WalletEvent::AddressGenerated { .. } => "address.generated",
        WalletEvent::BalanceFetchStarted { .. } => "balance.fetch.started",
        WalletEvent::BalanceUpdated { .. } => "balance.updated",
        WalletEvent::BalanceFetchFailed { .. } => "balance.fetch.failed",
        WalletEvent::TransactionSigned => "transaction.signed",
        WalletEvent::SwapCompleted => "swap.completed",
    }
}

fn event_payload(event: &WalletEvent) -> Value {
    match event {
        WalletEvent::WalletCreated { wallet_id }
        | WalletEvent::WalletImported { wallet_id }
        | WalletEvent::WalletUnlocked { wallet_id }
        | WalletEvent::WalletDeleted { wallet_id } => json!({ "wallet_id": wallet_id }),
        WalletEvent::AddressGenerated { wallet_id, chain_id, address } => json!({
            "wallet_id": wallet_id,
            "chain_id": chain_id,
            "address": address
        }),
        WalletEvent::BalanceFetchStarted { wallet_id, chain_id }
        | WalletEvent::BalanceUpdated { wallet_id, chain_id } => json!({
            "wallet_id": wallet_id,
            "chain_id": chain_id
        }),
        WalletEvent::BalanceFetchFailed { wallet_id, chain_id, message } => json!({
            "wallet_id": wallet_id,
            "chain_id": chain_id,
            "message": message
        }),
        WalletEvent::WalletEncrypted | WalletEvent::TransactionSigned | WalletEvent::SwapCompleted => json!({}),
    }
}

pub struct EventService;

impl EventService {
    /// Publish a typed wallet event through the engine-wide event bus while
    /// retaining the legacy bounded event buffer for existing polling callers.
    pub fn emit(event: WalletEvent) -> Result<(), EngineError> {
        let payload = event_payload(&event);
        EventBus::publish(event_type(&event), payload);

        let mut buffer = event_buffer()
            .lock()
            .map_err(|_| EngineError::Unexpected("event buffer lock poisoned".into()))?;
        if buffer.len() >= MAX_BUFFERED_EVENTS {
            buffer.remove(0);
        }
        buffer.push(event);
        Ok(())
    }

    /// Publish a structured event from any engine subsystem.
    pub fn publish(event_type: impl Into<String>, data: Value) {
        EventBus::publish(event_type, data);
    }

    pub fn snapshot() -> Result<Vec<WalletEvent>, EngineError> {
        let buffer = event_buffer()
            .lock()
            .map_err(|_| EngineError::Unexpected("event buffer lock poisoned".into()))?;
        Ok(buffer.clone())
    }

    pub fn drain() -> Result<Vec<WalletEvent>, EngineError> {
        let mut buffer = event_buffer()
            .lock()
            .map_err(|_| EngineError::Unexpected("event buffer lock poisoned".into()))?;
        Ok(std::mem::take(&mut *buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_is_available_on_live_bus_and_legacy_buffer() {
        let _ = EventService::drain().unwrap();
        let mut receiver = EventBus::subscribe();
        EventService::emit(WalletEvent::WalletCreated { wallet_id: "test-wallet".into() }).unwrap();

        let live = loop {
            let candidate = receiver.try_recv().expect("event should be published to subscribers");
            if candidate.event_type == "wallet.created"
                && candidate.data["wallet_id"] == "test-wallet"
            {
                break candidate;
            }
        };
        assert_eq!(live.event_type, "wallet.created");

        let buffered = EventService::drain().unwrap();
        assert_eq!(buffered.len(), 1);
        assert!(matches!(buffered[0], WalletEvent::WalletCreated { .. }));
    }

    #[test]
    fn generic_events_can_be_published() {
        let mut receiver = EventBus::subscribe();
        EventService::publish("test.event", json!({"wallet_id":"w1","chain_id":"tron"}));
        let event = loop {
            let candidate = receiver.try_recv().unwrap();
            if candidate.event_type == "test.event" {
                break candidate;
            }
        };
        assert_eq!(event.event_type, "test.event");
        assert_eq!(event.wallet_id.as_deref(), Some("w1"));
        assert_eq!(event.chain_id.as_deref(), Some("tron"));
    }
}
