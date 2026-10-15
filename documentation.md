# Wallet Engine Request and Adapter Contract

## Purpose

The wallet engine exposes a small, chain-agnostic request contract.

The service/orchestrator owns the action. The adapter registry owns routing. The selected adapter owns the blockchain-specific implementation.

The caller does **not** call a TRON, EVM, Solana, or other adapter function directly.

The flow is:

```text
Caller JSON
   |
   | action + params
   v
WalletProvider / service orchestrator
   |
   | chain_type
   v
AdapterRegistry
   |
   | selected adapter
   v
Chain Adapter
   |
   | adapter-owned implementation
   v
Derivation / signing / transaction modules
```

## Canonical request shape

Every adapter-facing derivation request uses the same outer shape:

```json
{
  "action": "derive_address",
  "params": {
    "chain_type": "tron",
    "index": 0
  }
}
```

Only `chain_type` selects the adapter.

The `action` is the operation requested from the adapter.

The remaining fields in `params` are operation parameters.

The wallet seed is **not** sent by the caller. The wallet service already holds the unlocked seed and injects it into the adapter boundary internally.

## Derivation actions

### Derive one address

Indexed form:

```json
{
  "action": "derive_address",
  "params": {
    "chain_type": "tron",
    "index": 0
  }
}
```

Explicit path form:

```json
{
  "action": "derive_address",
  "params": {
    "chain_type": "tron",
    "path": "m/44'/195'/0'/0/0"
  }
}
```

The adapter decides how the chain-specific path and address algorithm work.

If neither `index` nor `path` is supplied, an adapter may use its canonical default derivation path when it declares one.

Response:

```json
{
  "success": true,
  "data": {
    "address": "..."
  },
  "error": null
}
```

### Derive multiple addresses

Indexed batch form:

```json
{
  "action": "derive_addresses",
  "params": {
    "chain_type": "tron",
    "start_index": 0,
    "count": 5
  }
}
```

Response:

```json
{
  "success": true,
  "data": {
    "addresses": ["...", "...", "...", "...", "..."]
  },
  "error": null
}
```

Explicit paths are also supported:

```json
{
  "action": "derive_addresses",
  "params": {
    "chain_type": "tron",
    "paths": [
      "m/44'/195'/0'/0/0",
      "m/44'/195'/0'/0/1"
    ]
  }
}
```

### Derive a public key

```json
{
  "action": "derive_public_key",
  "params": {
    "chain_type": "tron",
    "path": "m/44'/195'/0'/0/0"
  }
}
```

Response:

```json
{
  "success": true,
  "data": {
    "public_key_hex": "..."
  },
  "error": null
}
```

### Derive a private key

```json
{
  "action": "derive_private_key",
  "params": {
    "chain_type": "tron",
    "path": "m/44'/195'/0'/0/0"
  }
}
```

This operation is a sensitive engine operation. A network-facing product API should normally keep it behind a trusted/local key-management boundary and should never log the returned key.

### Derive child key

```json
{
  "action": "derive_child_key",
  "params": {
    "chain_type": "tron",
    "path": "m/44'/195'/0'/0/0"
  }
}
```

The exact returned key material is adapter-defined. For TRON the current implementation returns the BIP-32 depth, private key, and chain code.

## Adapter responsibilities

Every adapter receives the same action contract.

For example, these requests have the same shape:

```json
{
  "action": "derive_address",
  "params": {
    "chain_type": "tron",
    "index": 0
  }
}
```

```json
{
  "action": "derive_address",
  "params": {
    "chain_type": "evm",
    "index": 0
  }
}
```

```json
{
  "action": "derive_address",
  "params": {
    "chain_type": "solana",
    "index": 0
  }
}
```

The registry does not implement the cryptography.

It resolves:

```text
tron   -> TronAdapter
evm    -> EvmAdapter
solana -> SolanaAdapter
...
```

The selected adapter then applies its own:

- mnemonic/seed rules
- curve
- BIP-32/BIP-44 or chain-specific derivation
- hardened/non-hardened rules
- derivation path policy
- public-key format
- address encoding
- network/address prefix
- validation rules

Therefore the same request contract can be used across all adapters without pretending that all blockchains derive addresses in the same way.

## Unsupported operations

An adapter must return an explicit `Unsupported` error when it cannot perform an operation.

The engine must not silently substitute another chain's derivation algorithm.

This is important because different chains can have different derivation models.

## TRON example

Current TRON wallet documentation specifies:

- BIP-39 mnemonic to seed
- BIP-32/BIP-44 derivation
- `m/44'/195'/0'/0/<index>`
- secp256k1
- uncompressed public key
- Keccak-256 of the public-key payload
- last 20 bytes
- `0x41` TRON address prefix
- Base58Check user-facing address beginning with `T`

The adapter therefore maps:

```text
derive_address + chain_type=tron + index=N
        |
        v
TronAdapter
        |
        v
tron::derivation::derive_address_at_index()
        |
        v
m/44'/195'/0'/0/N
        |
        v
TRON Base58Check address
```

The wallet service never needs to know those TRON-specific details.

## Architectural rule

The public/service layer should know only:

1. the action;
2. the `chain_type`;
3. the operation parameters;
4. the normalized response/error.

The adapter registry knows how to select the adapter.

The adapter knows how to execute the operation.

The adapter's internal modules know how to perform the actual cryptography and blockchain-specific work.

This keeps the engine chain-agnostic while allowing every adapter to implement the same high-level contract correctly.
