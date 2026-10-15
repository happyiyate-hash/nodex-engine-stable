# NodeX Wallet Engine

NodeX Wallet Engine is a Rust wallet engine with a browser-oriented test UI and WebAssembly-friendly library architecture.

## Web test console

The repository includes a developer test console at `static/dev_ui/index.html` and a Rust HTTP server at `src/bin/dev_ui.rs`.

Run locally:

```bash
cargo run --bin dev_ui
```

Then open:

```
http://localhost:3030/
```

Health check:

```
http://localhost:3030/health
```

The server also respects the hosting platform's `PORT` environment variable and binds to `0.0.0.0`, so it can run in a container or hosted service.

## Wallet request format

The browser sends JSON to `POST /api/request`:

```json
{
  "action": "create_wallet",
  "params": {
    "name": "Wallet 1",
    "word_count": 12,
    "pin": "test-pin",
    "chains": [
      {
        "name": "Ethereum",
        "symbol": "ETH",
        "chain_type": "evm",
        "chain_id": "1"
      },
      {
        "name": "Solana",
        "symbol": "SOL",
        "chain_type": "solana",
        "chain_id": "mainnet-beta"
      }
    ]
  }
}
```

The test UI is configured with Ethereum and Solana examples so wallet creation and address derivation can be exercised immediately.

## Important security note

The web console is a development/testing surface. It currently displays recovery phrases in the API response so the wallet-creation flow can be inspected. Do not use this deployment for real funds or real recovery phrases.

Before a public production wallet application is released, recovery phrases and private keys should remain under the wallet's secure secret-handling boundary and should not be returned through a general-purpose HTTP API or persisted in server logs.

## Container deployment

A Dockerfile is included for Rust-capable hosting providers. The application listens on the provider-supplied `PORT` value.
