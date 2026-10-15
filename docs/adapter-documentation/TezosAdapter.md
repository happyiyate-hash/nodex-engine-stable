# Tezos Adapter

## Scope

Tezos remains one adapter. Its user-account address type is determined by the signing curve:

- tz1 = Ed25519
- tz2 = secp256k1
- tz3 = P-256
- tz4 = BLS

The current Rust crypto dependencies support the first three. tz4/BLS is intentionally not silently mapped to another curve and should be added as a dedicated curve capability when BLS support is introduced. Tezos documentation confirms these account families and their curve mapping. citeturn1search2turn1search3

## Derivation path

Tezos uses coin type 1729. The default Octez path is:

m/44'/1729'/0'/0'

Octez documents this as the default Tezos key path and allows the signing curve to be selected separately. citeturn1search1

The adapter requires an explicit derivation path and validates that it begins with:

m/44'/1729'/account'/change'

The path is therefore part of the blockchain JSON/wallet request; it is not silently invented by the adapter.

## Address construction

A Tezos user-account address is a Base58Check-encoded Blake2b hash of the public key. The binary address prefix selects the curve/account type. citeturn1search0turn1search2

The adapter provides curve-specific address construction:

- tz1: Ed25519 public key → Blake2b-160 → tz1 prefix
- tz2: secp256k1 public key → Blake2b-160 → tz2 prefix
- tz3: P-256 public key → Blake2b-160 → tz3 prefix

All three are handled by the same TezosAdapter.

## Signing

Tezos does not simply sign arbitrary transaction JSON. The transaction/operation must first be forged into the canonical binary representation. The operation is then signed with the appropriate curve and operation watermark. Octez describes the generic manager-operation signing flow as signing the forged operation bytes with the Generic_operation watermark. citeturn2search0turn2search2

The adapter's curve-specific signing helper prepends the generic-operation watermark byte (0x03) to the supplied forged operation bytes before signing.

Therefore:

provider/operation builder → forge operation → TezosAdapter signing → provider injection

The adapter does not perform RPC forging or injection.

## Balance normalization

Balance fetching is not performed by TezosAdapter.

The provider retrieves raw XTZ/FA2/token amounts and their metadata. The balance orchestrator then calls:

TezosAdapter::normalize_balances(...)

XTZ uses 6 decimal places because one tez equals 1,000,000 mutez. citeturn0search0turn0search2

The adapter uses integer arithmetic and does not silently assume decimals for non-native assets.

## RPC/provider boundary

The provider layer is responsible for:

- RPC endpoint selection
- API keys/fallback
- Tezos RPC calls
- balance retrieval
- token/indexer queries
- operation forging
- fee estimation
- operation injection
- confirmation/receipt monitoring

Taquito's architecture similarly separates RPC access from signer configuration and operation preparation. citeturn0search0turn0search4

## Account activation/reveal

A newly generated Tezos user account can exist locally before it has been revealed on-chain. Tezos documentation notes that user accounts are unrevealed until they make a transaction or use a reveal operation. citeturn1search3

The adapter does not automatically submit a reveal operation. Reveal/activation is a network operation handled by the transaction/provider layer.

## Developer flow

### Wallet

seed → derivation_path → curve-specific key → Tezos public key → tz1/tz2/tz3 address

### Balance

Tezos Provider → raw mutez/FA2 amounts → TezosAdapter::normalize_balances → common balance representation

### Send

transaction builder → forge operation → TezosAdapter signing → provider injection → confirmation

## Important rules

- Keep one TezosAdapter.
- Do not create separate tz1/tz2/tz3 adapters.
- Require the derivation path.
- Do not fetch balances from inside the adapter.
- Do not put RPC/API-key handling inside the adapter.
- Do not treat a Tezos transaction as arbitrary JSON during signing; the operation must be correctly forged first.
- Do not silently treat tz4/BLS as tz1/tz2/tz3.
