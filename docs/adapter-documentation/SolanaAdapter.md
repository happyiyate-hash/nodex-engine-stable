# Solana Adapter

## Purpose

The Solana adapter owns Solana address derivation, Ed25519 signing, SOL/SPL balance handling, and Solana-specific operations.

The Wallet Engine does not need to understand Solana's cryptography. It routes the request by `chain_type: "solana"`; the Solana adapter interprets `body.action`.

## Adapter communication

~~~json
{
  "action": "adapter_target",
  "targets": [
    {
      "chain_type": "solana",
      "blockchain": {
        "name": "Solana",
        "symbol": "SOL",
        "decimals": 9,
        "chain_id": "solana-mainnet",
        "rpc_provider": "provider_name",
        "rpc_url": "https://..."
      },
      "body": {
        "action": "derive_address"
      }
    }
  ]
}
~~~

Flow:

~~~text
App
  -> Wallet Engine
  -> Adapter Orchestrator
  -> Adapter Registry
  -> chain_type = solana
  -> Solana Adapter
  -> body.action
~~~

The UI does not provide a derivation path for Solana.

## Address derivation

Solana uses Ed25519 keys and represents the public key as a Base58 address.

The adapter uses the internally defined Solana wallet derivation path:

~~~text
m/44'/501'/0'/0'
~~~

This path is an adapter/engine rule and is not included in the blockchain JSON.

Therefore the developer sends:

~~~json
{
  "chain_type": "solana",
  "blockchain": {
    "name": "Solana",
    "symbol": "SOL",
    "decimals": 9,
    "chain_id": "solana-mainnet",
    "rpc_provider": "provider_name",
    "rpc_url": "https://..."
  },
  "body": {
    "action": "derive_address"
  }
}
~~~

The adapter derives the Ed25519 child key, obtains the public key, and Base58-encodes the 32-byte public key as the wallet address.

## Private-key address recovery

The adapter can derive the Solana address directly from a 32-byte Ed25519 private key.

This is an adapter-level operation. The Wallet Engine does not need to know how Solana converts its public key into an address.

## Signing

The adapter provides Ed25519 signing through the common adapter interface:

~~~text
sign_transaction(private_key, tx_data)
~~~

The current primitive signs the supplied transaction/message bytes and returns the 64-byte Ed25519 signature.

This primitive must not be confused with complete Solana transaction construction. A complete Solana transaction/message must be constructed and serialized according to Solana's transaction format before it is passed to the signing primitive.

## Balance fetching

The balance request is routed to the configured Solana RPC endpoint.

Native SOL is represented in lamports and uses 9 decimals.

The adapter must preserve the raw integer balance until normalization. The normalized value is produced from the asset's configured decimals.

Example request:

~~~json
{
  "action": "adapter_target",
  "targets": [
    {
      "chain_type": "solana",
      "blockchain": {
        "name": "Solana",
        "symbol": "SOL",
        "decimals": 9,
        "chain_id": "solana-mainnet",
        "destination_address": "..."
      },
      "body": {
        "action": "get_balance"
      }
    }
  ]
}
~~~

## SPL tokens

SPL tokens are identified by their mint address. The adapter should not assume that every Solana asset is SOL.

For an SPL-token asset, the blockchain/token configuration should retain the token mint address, symbol, decimals, and balance.

Example asset:

~~~json
{
  "name": "USD Coin",
  "symbol": "USDC",
  "decimals": 6,
  "contract_address": "<USDC_MINT_ADDRESS>",
  "balance": "..."
}
~~~

The Adapter Orchestrator can send multiple Solana assets in one target. The Solana adapter is responsible for resolving the appropriate native/SPL balance information.

## Normalization

Normalization is an adapter operation because Solana has different raw units:

- SOL: lamports
- SPL tokens: token base units determined by the mint's decimals

The adapter converts the integer base-unit balance to the configured decimal representation without floating-point arithmetic.

Example:

~~~json
{
  "action": "adapter_target",
  "targets": [
    {
      "chain_type": "solana",
      "blockchain": {
        "name": "Solana",
        "symbol": "SOL",
        "decimals": 9
      },
      "body": {
        "action": "normalize",
        "assets": [
          {
            "name": "Solana",
            "symbol": "SOL",
            "decimals": 9,
            "balance": "1234567890"
          }
        ]
      }
    }
  ]
}
~~~

The normalized balance is:

~~~text
1.23456789 SOL
~~~

## Account security operation

The adapter also contains Solana-specific account/token security inspection.

This operation can inspect:

- token accounts belonging to an owner;
- token mint authorities;
- freeze authorities;
- token-account delegates;
- empty token accounts;
- related security information.

This is Solana-specific functionality and should remain inside the Solana adapter rather than being added to the generic Wallet Engine.

## Batch requests

Multiple Solana targets may be included in one `adapter_target` request.

The Adapter Orchestrator selects the Solana adapter using:

~~~text
chain_type = solana
~~~

The adapter then processes each target according to its `body.action`.

The Wallet Engine does not need to know how Solana groups or processes those requests.

## What the developer provides

For wallet creation, the developer provides:

~~~text
name
symbol
decimals
chain_id
rpc_provider
rpc_url
chain_type
~~~

The developer does **not** provide:

~~~text
derivation_path
~~~

because Solana's derivation path is an internal adapter rule.

## Production rules

1. `chain_type` selects the Solana adapter.
2. `body.action` selects the Solana operation.
3. Solana derivation uses the internally defined path `m/44'/501'/0'/0'`.
4. The derivation path is not required in the public blockchain JSON.
5. Solana addresses are Base58-encoded Ed25519 public keys.
6. Native SOL uses lamports as its raw unit and 9 decimals.
7. SPL tokens are identified by mint address and use their own decimals.
8. Raw monetary amounts must remain integer strings until normalization.
9. Floating-point arithmetic must not be used for wallet balances.
10. Raw Ed25519 signing is not the same thing as complete Solana transaction construction.
11. Solana-specific functionality stays inside the Solana adapter so new adapters do not require changes to generic wallet flows.
