# Hedera Adapter

## Scope

Hedera is registered as one adapter. The initial wallet flow uses Hedera's EVM-compatible account model: secp256k1 with the Ethereum BIP-44 derivation family. Hedera-native Ed25519 account creation is not triggered implicitly by wallet creation.

## Derivation

Use an explicit EVM-style path:

`m/44'/60'/account'/change/address_index`

Example: `m/44'/60'/0'/0/0`.

The adapter validates the 44' purpose, 60' Ethereum coin type, hardened purpose/coin/account components, and non-hardened change/address components.

Hedera's official documentation describes EVM accounts as ECDSA/secp256k1 accounts and demonstrates `m/44'/60'/0'/0` as the base path. citeturn0search0

## Address

The adapter derives the EVM-compatible 20-byte address from the secp256k1 public key using Keccak-256 and returns it as a `0x`-prefixed hexadecimal address.

Hedera supports native Account IDs as well as EVM address aliases. The current adapter deliberately returns the EVM-compatible address rather than inventing a native `0.0.x` account ID. citeturn0search0turn0search6

## Signing

Signing uses the existing secp256k1 signing implementation. The adapter validates the private key and signs the supplied transaction payload. Network submission and fee payment remain outside the adapter.

## Balance normalization

The adapter does not fetch balances. The Hedera provider retrieves HBAR/HTS raw balances, and the balance orchestrator calls `HederaAdapter::normalize_balances(...)`. Base-unit amounts are converted using the decimals supplied by the provider without floating-point arithmetic.

## Account creation

Wallet generation is local and does not automatically submit a Hedera account-creation transaction.

If an application later wants to create/fund an on-chain account, that should be a separate transaction flow. Hedera supports ECDSA aliases and can create accounts through transfers to unused ECDSA aliases/EVM addresses, as well as normal `CryptoCreate` flows. citeturn0search6

## RPC/provider boundary

The provider layer owns RPC/mirror-node communication, API keys, endpoint fallback, balance retrieval, transaction submission, and account lookup. HederaAdapter does not create an HTTP client or fetch balances.

The Hedera Mirror Node API can resolve accounts by ID, alias, or EVM address, which is useful to the provider layer. citeturn0search4

## Developer flow

Wallet: `seed → HederaAdapter → EVM derivation path → secp256k1 key → EVM-compatible Hedera address`.

Balance: `Hedera Provider → raw HBAR/HTS balances → HederaAdapter::normalize_balances(...) → common balance representation`.

Send/withdraw: `transaction builder → HederaAdapter signing → provider/network submission`.

## Important rules

- Do not automatically create an on-chain Hedera account during wallet generation.
- Do not fetch balances inside the adapter.
- Do not put RPC/API-key handling inside the adapter.
- Keep native Ed25519 account creation as a future explicit capability rather than mixing it into the basic wallet flow.
