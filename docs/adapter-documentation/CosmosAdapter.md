# Cosmos Adapter

## Purpose

The Cosmos adapter owns Cosmos-family address derivation, Cosmos account-address encoding, balance normalization, and secp256k1 transaction-signing primitives.

The Wallet Engine does not hardcode Cosmos derivation rules. Non-EVM derivation requests must provide the derivation path in the target blockchain JSON.

## Adapter communication

~~~json
{
  "action": "adapter_target",
  "targets": [{
    "chain_type": "cosmos",
    "blockchain": {
      "name": "Cosmos Hub",
      "symbol": "ATOM",
      "decimals": 6,
      "chain_id": "cosmoshub-4",
      "derivation_path": "m/44'/118'/0'/0/0",
      "rpc_provider": "provider_name",
      "rpc_url": "https://..."
    },
    "body": { "action": "derive_address" }
  }]
}
~~~

Routing: App -> Wallet Engine -> Adapter Orchestrator -> Adapter Registry -> chain_type=cosmos -> Cosmos Adapter -> body.action.

## Address derivation

body.action = derive_address. blockchain.derivation_path is required. The adapter parses the supplied path, derives the secp256k1 child key from the wallet seed, and converts the compressed public key into a Cosmos account Bech32 address.

Cosmos Hub commonly uses m/44'/118'/0'/0/0. The engine must not silently insert this path; the request remains authoritative.

## Mnemonic derivation

The mnemonic helper receives the derivation path explicitly: derive_from_mnemonic(mnemonic, derivation_path). It does not invent an account index or hardcode a path.

## Balance fetching

The adapter retrieves the complete Cosmos bank coin list from the configured endpoint. It does not assume every Cosmos chain is ATOM-only. Each requested asset is matched by its denomination identifier, raw base-unit amounts remain integer strings, and normalization uses the asset's configured decimals. Native ATOM on Cosmos Hub uses uatom when no explicit denomination identifier is supplied. No floating-point arithmetic is used for monetary amounts.

## Balance normalization

body.action = normalize.

~~~json
{
  "action": "adapter_target",
  "targets": [{
    "chain_type": "cosmos",
    "blockchain": {
      "name": "Cosmos Hub",
      "symbol": "ATOM",
      "decimals": 6,
      "chain_id": "cosmoshub-4",
      "destination_address": "cosmos1..."
    },
    "body": {
      "action": "normalize",
      "assets": [{ "name": "Cosmos", "symbol": "ATOM", "decimals": 6, "balance": "1234567" }]
    }
  }]
}
~~~

1234567 base units becomes 1.234567 after normalization.

## Signing

The adapter exposes secp256k1 signing through the common adapter interface: sign_transaction(private_key, tx_data). This is the cryptographic signing primitive. Complete Cosmos transaction data must already contain the applicable chain ID, account number, sequence, fee/gas, messages and signing mode. The primitive is not a generic Cosmos transaction builder.

## Batch requests

Multiple Cosmos targets can be included in one adapter_target request. The Adapter Orchestrator routes every chain_type=cosmos target to this adapter, with each target retaining its own blockchain configuration and derivation path.

## Production rules

1. chain_type selects the Cosmos adapter.
2. body.action selects the adapter operation.
3. derivation_path is required for non-EVM derivation.
4. The Wallet Engine must not hardcode Cosmos derivation paths.
5. Balance fetching must not assume ATOM/uatom for every Cosmos asset.
6. Monetary normalization must not use floating-point arithmetic.
7. Normal wallet derivation produces a Cosmos account address, not validator-operator or consensus addresses.
8. Cosmos transaction construction/signing rules remain inside the Cosmos transaction layer/adapter boundary.
