# Aptos Adapter

## Scope

Aptos is a standalone adapter. It is not merged with Sui or another Move-family chain.

The adapter owns Aptos-specific wallet derivation, address construction, and transaction signing. Provider/API-key management and balance fetching remain in the shared provider/balance subsystem.

## Derivation

Aptos HD wallets use BIP-44 coin type 637. The adapter requires an explicit derivation path.

Standard path:

m/44'/637'/account'/change'/address_index'

The current implementation requires exactly five path components and all components must be hardened.

Example:

{
  "name": "Aptos",
  "chain_type": "aptos",
  "chain_id": "1",
  "symbol": "APT",
  "derivation_path": "m/44'/637'/0'/0'/0'"
}

The adapter validates purpose 44' and coin type 637' before deriving the Ed25519 child key.

## Address

Aptos standard Ed25519 accounts use an authentication key derived from the public key and the Ed25519 scheme identifier. The resulting 32-byte account address is represented as a 0x-prefixed 64-hex-character value.

The engine's Aptos address helper performs the Aptos-specific SHA3-256 authentication-key construction.

## Signing

For a normal Ed25519 Aptos account, the adapter expects tx_data to be the canonical BCS serialization of the Aptos RawTransaction.

It creates the Aptos transaction signing digest from:

APTOS::RawTransaction domain separator + BCS RawTransaction

and hashes that signing message with SHA3-256 before Ed25519 signing.

The adapter returns the raw Ed25519 signature bytes. The Aptos transaction/authenticator layer is responsible for assembling the complete signed transaction/authenticator object.

Multi-agent, fee-payer, keyless, multi-key, and account-abstraction transaction modes are not silently implemented by this generic Ed25519 adapter method. They require their corresponding Aptos authenticator flow.

## Balance and normalization

Balance fetching is not performed by AptosAdapter.

The shared balance provider system must query Aptos RPC/indexer data, retrieve APT and supported fungible assets, resolve asset metadata/decimals, normalize raw integer quantities, and return the normalized batch.

Do not default an unknown fungible asset's decimals to zero. Unknown metadata must remain distinguishable from a real zero-decimal asset.

APT uses 8 decimal places (octas).

## RPC

AptosAdapter does not own RPC URLs, API keys, provider fallback, or HTTP clients.

The provider layer handles configured RPC endpoint selection, API-key injection, provider/key fallback, Aptos requests, balance/resource queries, and normalization before storage.

## Developer communication flow

### Create wallet

UI -> Wallet Engine -> AptosAdapter -> derivation_path -> Ed25519 child key -> Aptos authentication address -> wallet storage

### Sign transaction

UI -> Wallet Engine -> AptosAdapter -> canonical RawTransaction bytes -> Aptos signing digest -> Ed25519 signature -> Aptos authenticator/transaction layer

### Fetch balance

UI -> Balance Orchestrator -> Aptos Provider -> RPC/indexer -> APT + fungible assets -> normalization -> storage/UI

## Adapter-family rule

Do not create an ADAAdapter, Sui adapter inside Aptos, or generic Move adapter.

Aptos remains a standalone adapter because its address authentication and transaction signing rules are Aptos-specific.
