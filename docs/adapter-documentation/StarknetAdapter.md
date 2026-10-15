# Starknet Adapter

## Account model

Starknet has no EOAs. An account is a smart contract, so a private key/public key does not by itself equal a final on-chain address. The address is counterfactually computed from the account class hash, constructor calldata, salt, and deployer address. citeturn3search0turn4search0

## Default wallet profile

The adapter uses one Starknet adapter with an OpenZeppelin account profile by default. The current documented OpenZeppelin `AccountUpgradeable` preset class hash is:

`0x079a9a12fdfa0481e8d8d46599b90226cd7247b2667358bb00636dd864002314`

The constructor takes the Stark public key. The default salt is the same public key and the deployer address is zero, matching the standard precomputation pattern shown by Starknet.js. citeturn3search2turn4search0

## Derivation

The blockchain JSON must supply a derivation path. A common path is:

`m/44'/9004'/0'/0/0`

The adapter does not silently substitute a path. The derived private key is converted to a Starknet public key and then used to calculate the counterfactual account address.

## Address calculation

The adapter performs the Starknet contract-address calculation:

`pedersen("STARKNET_CONTRACT_ADDRESS", deployer_address, salt, class_hash, pedersen(constructor_calldata))`

The result is normalized into Starknet's L2 address range. The implementation uses `starknet-crypto`'s Pedersen hash implementation. citeturn2search3turn4search8

This means the adapter can return a valid **future account address before the account is deployed**.

## Deployment

Creating the wallet locally does not deploy the account.

The network flow is:

1. Derive the private/public key.
2. Calculate the counterfactual account address.
3. Send funds to that address.
4. Submit `DeployAccount` with the same class hash, constructor calldata, and salt.
5. Starknet validates the deployment and charges the deployment fee.

This counterfactual funding/deployment flow is explicitly documented by Starknet/OpenZeppelin. citeturn4search0turn4search4

The adapter does not fund the account, estimate network fees, or submit `DeployAccount`.

## Signing

The adapter signs a 32-byte Starknet message/transaction hash using the Stark curve ECDSA implementation and returns the `r || s` signature values. The account contract is responsible on-chain for validating the signature according to its account implementation. citeturn3search0turn4search9

## Account variants

Starknet supports multiple account implementations such as OpenZeppelin, ArgentX, Braavos, and Ethereum-flavored accounts. Their class hashes and constructor calldata can differ. citeturn4search0

Therefore they are **profiles of the single Starknet adapter**, not separate blockchain adapters. The adapter exposes `address_from_public_key_with_profile(...)` so a future registry/account configuration can supply another class hash and constructor scheme without creating another `ArgentXAdapter` or `BraavosAdapter`.

## Balance normalization

Balance fetching is not performed by StarknetAdapter. The provider obtains raw STRK/ERC-20 balances and metadata. The balance orchestrator passes them to `normalize_balances(...)`, which converts base units using the supplied decimals without floating-point arithmetic.

## Provider boundary

The Starknet provider owns RPC endpoint selection, API keys, fallback, network calls, balance fetching, transaction submission, fee estimation, deployment status, and account lookup.

## Developer flow

Wallet creation:

`seed → derivation_path → Stark private key → Stark public key → account profile → counterfactual address`

Account deployment:

`counterfactual address → fund address → DeployAccount → deployed account`

Transaction:

`transaction hash → StarknetAdapter signing → account contract validation → provider submission`

Balance:

`Starknet Provider → raw balances → StarknetAdapter::normalize_balances(...) → common balance representation`

## Important rules

- Do not treat Starknet as an EVM EOA chain.
- Do not derive the address from a public key alone.
- Do not deploy an account during ordinary local wallet generation.
- Do not fetch balances inside the adapter.
- Do not put RPC/API-key handling inside the adapter.
- Keep alternative account implementations as profiles of StarknetAdapter.
