# Sui Adapter

## Scope

Sui is a standalone adapter.

The adapter owns Ed25519 key derivation, Sui address construction, Sui transaction signing, and Sui-specific balance normalization. It does not own RPC clients, API keys, endpoint fallback, or balance fetching.

## Derivation

Sui uses SLIP-44 coin type 784.

The adapter requires an explicit five-level hardened path:

m/44'/784'/account'/change'/address_index'

Example blockchain JSON:

    {
      "name": "Sui",
      "chain_type": "sui",
      "chain_id": "mainnet",
      "symbol": "SUI",
      "derivation_path": "m/44'/784'/0'/0'/0'"
    }

The adapter validates purpose 44', coin type 784', exactly five segments, and that every segment is hardened. It does not silently replace a missing or caller-supplied derivation path.

## Address

Sui addresses are derived from the Ed25519 public key using the Sui address scheme and Blake2b-256. The resulting 32-byte value is represented as a 0x-prefixed hexadecimal address.

## Signing

The transaction input must be the canonical BCS serialization of Sui TransactionData.

The adapter constructs the Sui transaction intent: TransactionData scope = 0, Sui application = 0, V0 = 0. It then computes Blake2b-256(intent || BCS(TransactionData)) and signs that digest with Ed25519.

The returned serialized Ed25519 signature is scheme flag 0x00 + 64-byte signature + 32-byte Ed25519 public key.

The adapter does not build or submit the full transaction.

## Balance normalization

Balance fetching is not performed by SuiAdapter.

The Sui provider obtains raw balances from the configured Sui RPC/indexer provider. The balance orchestrator invokes the adapter normalization function. The adapter converts integer base-unit amounts into decimal quantities using the supplied asset decimals, without floating-point arithmetic.

Example: 1234567890 with 9 decimals becomes 1.23456789. SUI uses 9 decimals. Unknown decimals remain None; the adapter does not invent token metadata.

## RPC/provider boundary

The Sui provider owns endpoint selection, API-key handling, provider fallback, network requests, raw balance retrieval, and token metadata retrieval. SuiAdapter does not create an HTTP client or call RPC.

## Developer communication flow

### Wallet creation

UI -> Wallet Engine -> SuiAdapter -> derivation_path -> Ed25519 child key -> Sui address -> wallet storage

### Signing

UI -> transaction builder -> BCS TransactionData -> SuiAdapter -> Sui intent digest -> Ed25519 signature -> Sui signature envelope -> provider -> network

### Balance

UI -> Balance Orchestrator -> Sui Provider -> RPC/indexer -> raw SUI/token balances -> SuiAdapter::normalize_balances(...) -> common balance representation -> storage/UI

## Important rules

- Do not put balance fetching inside SuiAdapter.
- Do not put RPC/API-key management inside SuiAdapter.
- Do not create a generic Move adapter.
- Do not merge Sui with Aptos merely because both use Move.
- Require the derivation path in blockchain JSON.
