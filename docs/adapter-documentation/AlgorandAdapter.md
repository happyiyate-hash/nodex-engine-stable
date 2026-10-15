# Algorand Adapter

## Scope

Algorand is a standalone adapter. It owns Ed25519 derivation, Algorand address construction, transaction signing, and chain-specific balance normalization. RPC/API access and balance fetching remain in the provider layer.

## Derivation

Algorand uses SLIP-44 coin type 283. The adapter requires an explicit five-level hardened path:

m/44'/283'/account'/change'/address_index'

Example:

    {
      "name": "Algorand",
      "chain_type": "algorand",
      "chain_id": "mainnet",
      "symbol": "ALGO",
      "derivation_path": "m/44'/283'/0'/0'/0'"
    }

The adapter validates purpose 44', coin type 283', five components, and hardened components.

## Address

An Algorand address is Base32 without padding of the 32-byte Ed25519 public key followed by the first four bytes of SHA-512/256(public key). The result is the standard 58-character uppercase address.

## Signing

The transaction input must be the canonical msgpack encoding of the unsigned Algorand transaction. The adapter prepends the Algorand transaction domain separator `TX` and signs the resulting bytes with Ed25519.

The adapter returns the raw 64-byte Ed25519 signature. Construction of the signed transaction envelope and network submission remain outside the adapter.

## Balance normalization

Balance fetching is not performed by AlgorandAdapter. The provider retrieves raw microALGO/ASA integer amounts and metadata. The balance orchestrator invokes `AlgorandAdapter::normalize_balances(...)`.

Normalization uses the asset's supplied decimals and never uses floating-point arithmetic. ALGO normally has 6 decimals. Missing token decimals are rejected rather than silently assuming a value.

## RPC/provider boundary

The provider owns endpoint selection, API keys, fallback, HTTP requests, account/asset queries, and raw response parsing. The adapter does not create an HTTP client or call Algorand RPC directly.

## Developer flow

### Wallet
UI -> Wallet Engine -> AlgorandAdapter -> derivation_path -> Ed25519 key -> Algorand address -> storage

### Signing
UI -> transaction builder -> canonical msgpack transaction -> AlgorandAdapter -> `TX` domain tag -> Ed25519 signature -> signed transaction layer -> provider

### Balance
UI -> Balance Orchestrator -> Algorand Provider -> RPC -> raw balances/metadata -> AlgorandAdapter::normalize_balances(...) -> common balance representation

## Important rules

- Do not fetch balances inside AlgorandAdapter.
- Do not put RPC/API-key handling inside AlgorandAdapter.
- Require the derivation path in blockchain JSON.
- Do not invent decimals for an unknown Algorand asset.
