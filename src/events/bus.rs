use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::VecDeque,
    sync::{atomic::{AtomicU64, Ordering}, Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::broadcast;

const EVENT_HISTORY_LIMIT: usize = 512;
const LIVE_CHANNEL_CAPACITY: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineEvent {
    pub event_id: u64,
    pub event_type: String,
    pub timestamp_ms: u128,
    pub request_id: Option<String>,
    pub wallet_id: Option<String>,
    pub child_id: Option<String>,
    pub chain_id: Option<String>,
    pub data: Value,
}

fn sequence() -> &'static AtomicU64 {
    static SEQUENCE: AtomicU64 = AtomicU64::new(1);
    &SEQUENCE
}

fn history() -> &'static Mutex<VecDeque<EngineEvent>> {
    static HISTORY: OnceLock<Mutex<VecDeque<EngineEvent>>> = OnceLock::new();
    HISTORY.get_or_init(|| Mutex::new(VecDeque::with_capacity(EVENT_HISTORY_LIMIT)))
}

fn live_sender() -> &'static broadcast::Sender<EngineEvent> {
    static SENDER: OnceLock<broadcast::Sender<EngineEvent>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (sender, _) = broadcast::channel(LIVE_CHANNEL_CAPACITY);
        sender
    })
}

/// Shared event broker for every engine subsystem. Publishers do not need to
/// know which clients are listening or how those clients receive the events.
pub struct EventBus;

impl EventBus {
    pub fn publish(event_type: impl Into<String>, data: Value) -> EngineEvent {
        let event = EngineEvent {
            event_id: sequence().fetch_add(1, Ordering::Relaxed),
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            request_id: data.get("request_id").and_then(Value::as_str).map(str::to_owned),
            wallet_id: data.get("wallet_id").and_then(Value::as_str).map(str::to_owned),
            child_id: data.get("child_id").and_then(Value::as_str).map(str::to_owned),
            chain_id: data.get("chain_id").and_then(Value::as_str).map(str::to_owned),
            event_type: event_type.into(),
            data,
        };

        if let Ok(mut events) = history().lock() {
            if events.len() >= EVENT_HISTORY_LIMIT {
                events.pop_front();
            }
            events.push_back(event.clone());
        }

        // No subscribers is normal: events remain in bounded history.
        let _ = live_sender().send(event.clone());
        event
    }

    /// Subscribe to live events. Each subscriber receives an independent stream.
    pub fn subscribe() -> broadcast::Receiver<EngineEvent> {
        live_sender().subscribe()
    }

    /// Return retained events, oldest first.
    pub fn snapshot() -> Vec<EngineEvent> {
        history().lock().map(|events| events.iter().cloned().collect()).unwrap_or_default()
    }
}
