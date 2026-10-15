# Substrate Adapter

## Purpose

SubstrateAdapter is the single adapter for compatible Substrate/SS58 networks such as Polkadot, Kusama, Westend and Asset Hub.

The wallet flow must not contain network-specific Substrate logic. It passes the blockchain configuration and the requested operation to the adapter.

## Derivation

Substrate is not one of the adapters with a hardcoded derivation path.

The blockchain configuration must provide derivation_path.

Example JSON:

{
  "name": "Polkadot",
  "symbol": "DOT",
  "chain_type": "substrate",
  "derivation_path": "<caller-selected-path>",
  "rpc_url": "<configured-rpc>"
}

The adapter validates that the path is present and valid before deriving the address.

## Network selection

The adapter uses chain configuration to select the SS58 prefix. The same adapter can therefore produce the correct address representation for different Substrate networks.

Supported configurations currently include Polkadot, Kusama, Westend and Asset Hub.

Adding another compatible network should normally be a configuration/registry change, not a new adapter.

## Address derivation

The adapter receives the seed/private derivation material, derivation_path, and target blockchain configuration. It validates the supplied path, derives the key material, and encodes the resulting public key using the target network's SS58 representation.

The generic wallet creation flow does not decide the SS58 prefix or create a second Substrate adapter.

## Signing

Transaction signing is handled by SubstrateAdapter using the Substrate/SR25519 signing implementation. The caller supplies the private signing material and transaction payload, and the adapter returns the signed payload.

## Balance normalization

Balance fetching is not owned by this adapter. The RPC/provider balance system is responsible for retrieving balances. The adapter can normalize Substrate base units using the asset's decimals.

## Architecture rule

Do not create PolkadotAdapter, KusamaAdapter, WestendAdapter or AssetHubAdapter when the network can be represented by SubstrateAdapter plus configuration.

Do not put RPC-provider-specific balance fetching into this adapter.

## Developer flow

UI -> Wallet Engine -> SubstrateAdapter -> derive using blockchain.derivation_path -> SS58 address

Signing:

UI -> Wallet Engine -> SubstrateAdapter -> SR25519 signing -> signed transaction
