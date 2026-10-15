# NEAR Adapter

## Scope

NEAR is a standalone adapter.

The adapter owns:
- Ed25519 HD key derivation
- NEAR implicit-account address construction
- NEAR transaction signing

The adapter does not own RPC clients, API keys, balance fetching, token discovery, or balance normalization.

## Derivation

NEAR uses Ed25519 and registered SLIP-44 coin type 397.

Canonical seed-phrase path:

m/44'/397'/0'

The adapter requires a derivation_path from the blockchain configuration. It validates purpose 44', coin type 397', and requires all path segments to be hardened.

Longer wallet-specific paths are supported, for example:

m/44'/397'/0'/0'/1'

This is important because hardware wallets can use longer NEAR paths. The adapter must not silently replace a supplied path.

Example blockchain JSON:

{
  "name": "NEAR",
  "chain_type": "near",
  "chain_id": "mainnet",
  "symbol": "NEAR",
  "derivation_path": "m/44'/397'/0'"
}

## Address derivation

For a stateless wallet primitive, the adapter produces a NEAR implicit account ID.

The implicit account ID is the 32-byte Ed25519 public key represented as 64 lowercase hexadecimal characters.

Example conceptual flow:

seed
-> supplied NEAR derivation_path
-> Ed25519 private key
-> Ed25519 public key
-> lowercase 64-character hex
-> implicit NEAR account ID

Named accounts such as alice.near are different: they are on-chain account identifiers and require an existing account/key relationship. The adapter therefore does not invent named account IDs.

## Signing

For a normal NEAR transaction, tx_data passed to the adapter must be the canonical Borsh serialization of the unsigned transaction.

The adapter computes:

SHA-256(canonical Borsh transaction)

and signs that 32-byte digest with the Ed25519 private key.

The returned bytes are the Ed25519 signature. The transaction builder/authenticator layer remains responsible for attaching the signature and public-key representation required by NEAR.

Do not pass JSON transaction text or an already signed transaction as tx_data.

## Balance and normalization

Balance fetching is intentionally not implemented in NearAdapter.

The shared Balance Orchestrator and NEAR provider are responsible for:
- querying view_account over NEAR RPC;
- retrieving native NEAR balance;
- discovering/querying NEP-141 fungible-token balances;
- obtaining token metadata/decimals;
- normalizing raw integer amounts;
- returning normalized assets in the common balance schema.

NEAR native amounts are denominated in yoctoNEAR, with 24 decimal places.

The adapter must never create its own RPC client for balance fetching.

## RPC/provider communication

The provider layer owns endpoint selection, API keys, retries/fallback, HTTP/RPC requests, and provider-specific response parsing.

The adapter receives only chain/wallet data required for derivation and signing.

## Developer flow

### Create wallet

UI
-> Wallet Engine
-> NEARAdapter
-> derivation_path
-> Ed25519 key derivation
-> implicit account address
-> wallet storage

### Sign transaction

UI
-> transaction builder
-> canonical Borsh transaction
-> NEARAdapter
-> SHA-256 transaction digest
-> Ed25519 signature
-> NEAR transaction/authenticator
-> provider
-> network

### Fetch balance

UI
-> Balance Orchestrator
-> NEAR Provider
-> NEAR RPC
-> raw balances/token metadata
-> common normalization
-> storage/UI

## Important rules

- Do not create a separate NEAR balance implementation inside the adapter.
- Do not silently replace a caller-supplied derivation path.
- Do not treat named accounts as derived addresses.
- Do not assume every NEP-141 token has the same decimals as NEAR.
- Do not create another adapter for NEAR tokens.
