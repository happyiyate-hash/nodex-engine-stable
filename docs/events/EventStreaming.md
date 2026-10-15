# Engine event streaming

NodeX exposes a shared event broker for engine subsystems. Internal modules publish
structured events through `EventService::publish` or typed wallet events through
`EventService::emit`. Publishers do not know which client is listening.

## Event flow

- `src/events/bus.rs` owns the bounded event history and Tokio broadcast channel.
- `src/events/event.rs` retains typed wallet event definitions.
- `src/services/event_service.rs` is a compatibility facade for typed events and
  the generic publishing interface.
- The balance service processes one chain at a time and publishes
  `balance.fetch.started` only when that chain begins.
- Each success produces one `balance.result` event with `wallet_id`, `chain_id`,
  `provider`, `duration_ms`, and a nested `balance` object containing that
  blockchain's native balance and complete token list. The event is never a
  per-token event and never groups multiple chains together.
- Each failure produces `balance.fetch.failed` with the same chain/provider
  identity, elapsed time, and error message. A final `balance.refresh.completed`
  event reports aggregate success/failure counts.
- Async engine requests emit `request.started`, followed by `request.completed`
  or `request.failed`. The streamed response data is sanitized to exclude recovery
  phrases, seeds, private keys, PINs, passwords, backup material and encrypted vaults.
- The frontend applies each `balance.result` immediately as it arrives, rather than
  waiting for the full multi-chain balance request to finish. It identifies the
  target using canonical chain IDs/names only; wallet addresses are never used to
  decide which blockchain owns a balance, because EVM addresses are shared across networks.
- The development HTTP server exposes `GET /api/events` as a Server-Sent Events
  stream. It requires `Authorization: Bearer <token>`, where the server's
  `NODEX_EVENT_STREAM_TOKEN` environment variable is set to a non-empty secret.
  The UI is only one possible client; it is not part of the event broker.

## Event envelope

Each streamed event contains `event_id`, `event_type`, `timestamp_ms`, optional
`request_id`, `wallet_id`, `child_id`, and `chain_id`, plus a `data` payload.
The event bus keeps up to 512 events in memory and each live subscriber has a
bounded broadcast channel. A slow subscriber may miss events if it falls behind;
clients should connect before issuing an operation and use the completion event
to determine when an operation has finished.

## Local-first and remote connection modes

The UI should subscribe when it mounts and keep the connection open for the lifetime
of the app. It does not need to initiate a separate request for each event. A request
can produce multiple progress/result events, and background work can publish events
without a UI request being active.

For an embedded/on-device engine, set `NODEX_LOCAL_MODE=true`. The engine binds
only to `127.0.0.1` and permits token-free event streaming for the local UI. This
mode is intended for a packaged app running on the same device; it must not be used
for a publicly exposed service.

For remote hosting, keep `NODEX_LOCAL_MODE` disabled and configure
`NODEX_EVENT_STREAM_TOKEN`. The UI can send that bearer token when connecting to
the remote stream. Do not embed a permanent server secret in a public frontend bundle.

Native `EventSource` does not support custom request headers, so the current browser
client uses a fetch-based SSE reader.

Example request:

```js
const response = await fetch("/api/events", {
  headers: { Authorization: `Bearer ${eventStreamToken}` },
});
if (!response.ok || !response.body) throw new Error("Event stream unavailable");
// Read response.body with a streaming SSE parser.
```

Only publish data safe for the receiving application. Never put recovery phrases,
private keys, seeds, PINs, or other secret material on the event bus. The event
token is a simple host-configured gate, not a substitute for per-wallet authorization
when multiple users share one engine process.
