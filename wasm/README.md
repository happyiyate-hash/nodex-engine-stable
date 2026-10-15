# NodeX browser WASM runtime

This crate is the first incremental step toward running NodeX wallet logic on
the user's device. It compiles independently of the server binary so server-only
dependencies (HTTP server, RPC clients, Tokio runtime) do not get pulled into
the browser bundle.

## Current scope

- Generate a 12-word English BIP-39 phrase using the target's secure randomness.
- Validate English BIP-39 phrases and checksums locally.
- Expose a small WebAssembly interface through `wasm-bindgen`.

This is **not yet a complete local wallet engine**. Address derivation, local
vault encryption, signing, chain adapters, and the Sable integration must be
ported and tested before local WASM can replace the NodeX API for wallet
creation. Do not use this milestone to secure real funds.

## Build

Install Rust, the `wasm32-unknown-unknown` target, and wasm-pack, then run:

```sh
rustup target add wasm32-unknown-unknown
cargo test --manifest-path wasm/Cargo.toml
wasm-pack build wasm --target web --out-dir ../frontend/src/lib/wasm/pkg
```

The phrase returned by `generate_recovery_phrase` is secret material. Never
log it, send it to analytics, or store it unencrypted. Browser WASM does not
protect secrets from a compromised device or malicious page script.
