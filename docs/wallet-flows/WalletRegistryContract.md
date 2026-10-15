# Wallet Registry and Session Contract

This document is the canonical wallet-lifecycle contract for Nodex Engine. New wallet and adapter work must follow it rather than introducing separate wallet-selection or secret-access flows.

## Ownership boundaries

- **Wallet Registry** owns wallet-group and child-record identity, their relationships, registered chain addresses, and the authoritative active group/child selection.
- **Wallet storage/persistence integration** exports a versioned registry snapshot containing child records, cached chain addresses, token metadata, active selection, and encrypted secrets only. The host must persist that snapshot and restore it on startup. The developer browser console writes snapshots to localStorage after wallet mutations and restores them on reload; other host applications must call the snapshot actions themselves. Unlocked mnemonic/seed bytes are never included in snapshots.
- **Secret access** is performed by the wallet/registry boundary after authorization. Adapters do not look up mnemonics or decrypt stored secrets themselves.
- **Adapter Orchestrator and chain adapters** receive only the material needed for the requested chain operation. They do not select the active wallet or manage child records.
- **Discovery/balance services** are explicit network operations and are not implicitly invoked by create/import/child derivation/address lookup.

## Identity model

- `wallet_id` in wallet-group APIs identifies the master wallet group.
- `child_id` identifies a specific wallet record within that group.
- `child_index` is the logical derivation index and is not the same as `child_id`.
- Each registered chain address belongs to a child record and a chain/network identity.
- Token contract metadata is chain/network scoped; it is not derived from the wallet mnemonic.

## Session invariants

1. There is one authoritative active wallet group and one active child within that group.
2. Switching a group changes the group context and selects its root child (Child 0) by default.
3. Switching a child changes only the child within its active master group unless a group is explicitly supplied.
4. A failed switch must not partially update active state.
5. A wallet-scoped operation resolves from the active registry session when no target is supplied.
6. Explicit `wallet_id` and/or `child_id` values override the session for that operation only; ordinary reads and writes do not change the active selection.
7. If both group and child IDs are supplied, the engine validates that the child belongs to that group.
8. Registered-wallet resolution must never use stale engine-local IDs, addresses, or seed fields as an implicit fallback.
9. A registered wallet that is locked requires authorization before seed/private-key operations. An explicitly supplied unknown wallet ID is an error, never a reason to fall back to another wallet's secret.
10. Creating a child activates it in the registry; the engine instance must synchronize its selected child context.
11. Switching with a PIN/password unlocks the selected wallet session. Switching without credentials changes selection but leaves secret-dependent operations locked.
12. Switching to a missing child never creates it; use `create_child_wallet` for creation.
13. Resolve the active session once per operation and carry that resolved context through async execution so concurrent switches cannot retarget an in-flight operation.

## Required flows

### Create/import
1. Validate input and PIN/password.
2. Generate or validate mnemonic and derive seed.
3. Derive Child 0 only for the requested/configured chains.
4. Register wallet group, child, chain addresses, and encrypted secret material.
5. Return stable group/child IDs and per-chain derivation status.
6. Do not scan the blockchain or refresh balances automatically.

### Create child
1. Resolve the parent group and validate HD wallet type.
2. If `child_index` is supplied, it is authoritative and the final component of each selected chain's path template is rewritten to that index. If no index is supplied, a consistent explicit path index may select the child; otherwise choose max existing index + 1.
3. Never create intermediate child records.
4. Derive only explicitly requested chains, or inherit configured parent chains when omitted.
5. Persist the child/address records, then update parent-child linkage and active context.
6. Do not trigger network discovery.

### Switch group/child
1. Resolve and validate target identity and parent relationship.
2. If credentials are supplied, verify/decrypt before committing active state.
3. Update active flags and active group/child together.
4. Clear previous session secrets before making the new context available.
5. Keep WalletEngine's cached identity aligned, but resolve registered-wallet secrets through the registry rather than trusting cached seed values.

### Get an address
- `get_address` reads an already-registered address from the selected child record.
- It does not derive, register, create a child, switch wallets, or scan a chain.
- Identify the network with `chain_id` when the chain selection could be ambiguous.
- Missing addresses return an explicit not-registered error; callers must separately request `add_blockchain` or child derivation.

### Get mnemonic
- `get_mnemonic` requires a wallet/group or child identifier and PIN/password.
- It decrypts through the wallet secret boundary.
- It is never an adapter action and must never log mnemonic/seed/private-key material.
- Private-key-only wallets do not have a mnemonic and must return an explicit error.

### Derive/sign
- Address derivation resolves the target wallet's seed from an authorized registry session or PIN/password.
- Signing accepts an explicit `child_id` (preferred), `chain_id`, `chain_type`, transaction bytes, and PIN/password. The exact network is selected by `chain_id`, and the child record's saved derivation path selects the signing key.
- The PIN remains inside the trusted wallet engine/registry boundary. It is not fanned out or broadcast to adapters; only the operation-specific signing material is passed to the signing service.
- A missing/locked/unknown registered wallet must not silently fall back to `WalletEngine.seed`.
- Transaction signing and transaction broadcast remain distinct operations.

### Adapter context
- `adapter_call` / `adapter_target` resolve the active child when no explicit IDs are supplied. Optional `child_id` and/or `wallet_id` override the session for that call; include `chain_id` when multiple configured networks share a `chain_type`.
- The engine routes against that child's registered network and injects public `wallet_context` (child ID/index, chain ID/type, saved address, derivation path) into object request bodies.
- Adapter requests never receive PINs, mnemonics, seeds, or private keys. Read-only address/balance operations use the saved address and do not rederive it.

### Token metadata and balances
- `add_token` / `add_token_metadata` stores token contract metadata on a specific child and `chain_id`; re-adding a contract updates its record. `get_token` / `get_token_metadata` reads that cached metadata and requires `chain_id`.
- A general token registry should ultimately resolve tokens independently of a particular wallet's balance cache.
- Balance refresh, transaction history, and discovery are explicit network operations; they must not run during local wallet creation or address retrieval.

## Endpoint request/response contract (UI and engine callers)

All requests use the same envelope: `{"action":"...","params":{...}}`. The UI may call these actions directly, and engine workflows such as signing/swapping should call the same actions rather than reimplementing wallet selection.

### Create a master wallet group

Action: `create_wallet` (legacy aliases: `create_wallet_group`, `create_master_wallet`, `generate_wallet`).

```json
{
  "action": "create_wallet",
  "params": {
    "name": "Main",
    "pin": "<wallet-password>",
    "word_count": 12,
    "chains": [
      {
        "name": "Ethereum",
        "chain_id": "1",
        "symbol": "ETH",
        "chain_type": "evm",
        "derivation_path": "m/44'/60'/0'/0/0"
      }
    ]
  }
}
```

Creates one master group and Child 0 in a single operation. The encrypted secret is owned by that group; addresses are saved on Child 0. Creation does not call RPCs or fetch balances.

### Create a child in the active master group (or an explicit group)

Action: `create_child_wallet` (alias: `import_child_wallet`; “import” here means deriving another child from the same HD group, not importing a second mnemonic).

```json
{
  "action": "create_child_wallet",
  "params": {
    "wallet_id": "<master-wallet-group-id>",
    "child_index": 3,
    "name": "Trading",
    "pin": "<same-group-wallet-password>",
    "chains": [
      {
        "name": "Ethereum",
        "chain_id": "1",
        "symbol": "ETH",
        "chain_type": "evm",
        "derivation_path": "m/44'/60'/0'/0/0"
      }
    ]
  }
}
```

If `wallet_id` is omitted, the active master group is used. The PIN/password is required to derive a new child. The registry decrypts that group's seed inside the trusted wallet boundary, rewrites the path's final child component to index 3, derives only the requested networks, and saves the new child/address records under the same group. It does not create intermediate indices or send the PIN/seed to adapters. If `chains` is omitted, the child's configured chains are inherited from the parent.

### Switch child within a master group

Action: `switch_child_wallet` (alias: `switch_child`). Provide `child_id` or `child_index`. If `wallet_id` is omitted, the active master group is used. If supplied, it explicitly targets that group. This action only selects an existing child; if the child does not exist, it returns `CHILD_WALLET_NOT_FOUND`. Call `create_child_wallet` to create a missing child.

```json
{
  "action": "switch_child_wallet",
  "params": {
    "child_index": 3
  }
}
```

The engine selects the existing child within the active master group. Supplying `wallet_id` explicitly selects within that group; supplying a PIN/password verifies it and unlocks the session. Without credentials, selection succeeds but secret-dependent actions remain locked. A child belonging to another group is rejected.

### Switch master group

Action: `switch_wallet` (legacy alias: `switch_master_wallet`).

```json
{
  "action": "switch_wallet",
  "params": {
    "wallet_id": "<existing-master-wallet-group-id>",
    "pin": "<optional-wallet-password>"
  }
}
```

The group must already exist. The engine selects that group's Child 0; it never creates a new group during a switch. A missing group returns `WALLET_GROUP_NOT_FOUND`. A supplied PIN/password verifies and unlocks the group; if omitted, the group is selected locked.

### Session-first reads and wallet operations

All wallet-scoped operations use the active registry session when both `wallet_id` and `child_id` are omitted. Explicit IDs override the session for that operation only; ordinary reads/writes do not change the active selection. If both IDs are supplied, the engine validates that the child belongs to that master group.

- `get_address`: `{"chain_id":"1"}`. Returns the stored address and derivation path for the active child. Optional `wallet_id` and `child_id` explicitly target another wallet for this request.
- `add_token`: `{"chain_id":"1","symbol":"USDT","contract_address":"<verified-contract>","decimals":6}`. Saves token metadata to the active child on the specified network. Optional IDs explicitly target another child.
- `get_token` / `get_token_metadata` / `get_token_contract_address`: `{"chain_id":"1","symbol":"USDT"}`. Reads saved metadata for the active child and exact network.
- `get_wallet_context`: `{"chain_id":"1"}`; returns saved addresses, derivation paths, network configuration, and token metadata for executor workflows.
- `sign_transaction`: provide `chain_id`, `chain_type`, `tx_data_hex`, and `pin`/`password`. The engine signs for the active child unless explicit IDs override the session. The wallet boundary resolves the key for that child and network; the PIN is never sent to an adapter.
- `create_child_wallet`: uses the active master group when `wallet_id` is omitted; explicit `wallet_id` overrides it.
- `switch_wallet`: requires an existing `wallet_id`, switches the active master group, and selects its Child 0.
- `switch_child_wallet`: uses the active master group and requires an existing `child_id` or `child_index`; it never creates a child.

These endpoints are the stable wallet-facing contract. Executor and adapter integrations should consume them and the resolved public wallet context rather than implement a parallel group/child/secret store.

## Request actions currently exposed

- Wallet lifecycle: `create_wallet` (legacy aliases: `create_wallet_group`, `create_master_wallet`, `generate_wallet`), `import_wallet`, `create_child_wallet` (or `import_child_wallet` for a child in the same group), `switch_wallet` (legacy alias: `switch_master_wallet`), `switch_child_wallet` (legacy alias: `switch_child`), `restore_persisted_wallet`, `delete_wallet`.
- Wallet reads: `active_wallet`, `list_wallets`, `wallet_groups`, `wallet_group`, `get_address`, `get_wallet_context`, `get_child_context`.
- Secret access: `get_mnemonic` (requires PIN/password).
- Chain/token metadata: `add_blockchain`, `add_token`, `add_token_metadata`, `get_token`, `get_token_metadata`, `get_token_contract_address`.
- Persistence: `export_wallet_registry` (or `persist_wallet_registry`) returns an encrypted snapshot; `restore_wallet_registry` restores it locked, requiring PIN/password for secret operations.
- Derivation/signing: `derive_address`, `derive_addresses`, `derive_private_key`, `derive_public_key`, `derive_child_key`, `sign_transaction`.
- Network operations: `get_balance`, `refresh_balance`, `discover_child_accounts`.

## Remaining verification gate before broad adapter work

The main persistence contract is now implemented: export_wallet_registry serializes wallet records and encrypted secret payloads; restore_wallet_registry validates and restores the snapshot while clearing all unlocked secret material. The developer browser console stores that snapshot in localStorage after wallet mutations and restores it on reload. Other clients must do the same through the two actions.

The following are still not complete and must not be assumed:

1. A global chain/network token catalog independent of each child wallet's token metadata (wallet-specific token add/get is implemented).
2. Automated Rust test execution and confirmed green CI for these changes.
3. Integration tests for concurrent child creation, failed switches, locked sessions, private-key wallets, and browser storage failures.
4. End-to-end verification that every external adapter client invokes snapshot persistence after all mutations; the engine cannot write browser localStorage directly.

Addresses are derived when a chain is registered on a child, stored in that child's record, and get_address reads the stored value. Signing resolves the child's signing key from the registry; it does not need to derive the public address as part of signing. This contract should remain stable while the remaining tests and global token catalog are completed.
