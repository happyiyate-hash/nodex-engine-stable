# Cardano Adapter

## Scope

Cardano is a standalone adapter. ADA and Cardano native assets use the same Cardano multi-asset UTxO ledger and do not require separate adapters.

## Derivation

Cardano requires an explicit derivation_path.

CIP-1852 Shelley wallet derivation:

m/1852'/1815'/account'/role/index

For a normal payment address the adapter derives role 0 for the payment key and role 2 for the staking key, then constructs a Shelley base address from both credentials.

## Network

The blockchain configuration supplies the network.

- mainnet -> address network tag 1 and addr prefix
- test networks -> address network tag 0 and addr_test prefix

## Address construction

A Shelley base address contains payment and stake credentials. The adapter hashes both Ed25519 public keys with Blake2b-224 and encodes the serialized address as Cardano Bech32.

The generic pub_key_to_address operation can only produce an enterprise key address because that operation has no staking-key/network configuration. Normal wallet creation uses derive_addresses so the complete base address is generated.

## Signing

Cardano transaction signing signs the transaction-body hash. The adapter hashes the supplied transaction body with Blake2b-256 semantics (the first 32 bytes of Blake2b-512) and passes that digest to the Ed25519 signer.

A transaction/witness layer remains responsible for assembling the complete Cardano witness structure.

## Balance and normalization

Balance fetching is NOT performed by CardanoAdapter.

The shared balance/provider system queries Cardano UTxOs and normalizes the result.

Cardano supports native multi-assets. ADA is represented in lovelace (1 ADA = 1,000,000 lovelace). Other native assets are identified by policy ID plus asset name. Do not assume every native asset has an on-chain decimal field.

## RPC / SDK communication

Provider and API-key management stays in the shared provider layer. Cardano can be accessed through cardano-cli, Ogmios, submit-api, cardano-api, Pallas and managed query layers such as Blockfrost, Koios and Maestro.

## Developer flow

Wallet creation:
UI -> Wallet Engine -> CardanoAdapter -> supplied CIP-1852 path -> payment + staking keys -> Cardano address

Balance:
UI -> Balance Orchestrator -> Cardano provider/RPC -> UTxOs/assets -> normalization -> storage/UI

Signing:
UI -> Wallet Engine -> CardanoAdapter -> transaction-body hash -> Ed25519 signature -> transaction/witness layer

## Rules

- Do not create ADAAdapter.
- Do not create separate native-token adapters.
- Do not fetch balances directly from this adapter.
- Do not hardcode the user's derivation path.
- Do not hardcode the network when using derive_addresses.
