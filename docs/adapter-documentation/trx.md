# TRX Adapter API

This document is the contract between the application/orchestrator and the TRX adapter.

The adapter is responsible for **TRON-specific work**:

- account permission inspection
- owner/active/witness permission validation
- permission proposal construction
- permission-update transaction preparation
- TRON permission-aware signing helpers
- signed transaction broadcasting

The adapter does **not** choose a public RPC provider. It uses the saved TRX blockchain configuration:

- `rpc_provider`
- `rpc_url`

The RPC orchestrator selects the provider and performs the network communication.

> **Important:** JSON shown below is the exact structured request body expected by the current JSON entry point. Keep field names and action names exactly as shown.

---

# 1. Common call structure

The application calls:

```text
TRX Adapter
  -> permission_json(rpc_orchestrator, request_bytes)
```

The request body is JSON.

The general structure is:

```json
{
  "action": "<ACTION>",
  "...": "action-specific fields"
}
```

The adapter returns JSON bytes. After decoding, every successful action returns an object containing:

```json
{
  "ok": true,
  "action": "<ACTION>"
}
```

An unsupported action or invalid body returns an adapter error instead of pretending the operation succeeded.

---

# 2. How RPC is supplied

The TRX adapter receives its RPC configuration from the saved TRX chain configuration.

Example:

```json
{
  "name": "TRON",
  "symbol": "TRX",
  "destination_address": "T...",
  "rpc_provider": "your_provider_name",
  "rpc_url": "https://your-provider.example"
}
```

The application does **not** put a provider URL inside every permission request.

Instead:

```text
Saved TRX configuration
        |
        +-- rpc_provider
        +-- rpc_url
        |
        v
TRX Adapter
        |
        v
RPC Orchestrator
        |
        v
Selected provider
        |
        v
TRON RPC
```

Never hardcode TronGrid, Alchemy, or another public endpoint inside the TRX adapter.

---

# 3. Get permissions

Use this when an address is imported or when the UI needs to inspect its current TRON permissions.

## Function

```text
permission_json(rpc_orchestrator, request_bytes)
```

## Exact request body

```json
{
  "action": "get_permissions",
  "address": "TYourTronAddress"
}
```

## Fields

| Field | Type | Required | Meaning |
|---|---|---:|---|
| `action` | string | yes | Must be `get_permissions` |
| `address` | string | yes | TRON address to inspect |

## What happens

The adapter sends the equivalent TRON account request through the shared RPC layer:

```text
POST /wallet/getaccount
```

with:

```json
{
  "address": "TYourTronAddress",
  "visible": true
}
```

The adapter extracts the permission information and returns it to the application.

## Example response shape

```json
{
  "ok": true,
  "action": "get_permissions",
  "address": "TYourTronAddress",
  "permissions": {
    "owner": {
      "id": 0,
      "permission_name": "owner",
      "permission_type": 0,
      "threshold": 2,
      "operations": null,
      "keys": [
        {
          "address": "TSignerA",
          "weight": 1
        },
        {
          "address": "TSignerB",
          "weight": 1
        }
      ]
    },
    "witness": null,
    "active": [
      {
        "id": 2,
        "permission_name": "payments",
        "permission_type": 2,
        "threshold": 1,
        "operations": "7fff...",
        "keys": [
          {
            "address": "TSignerA",
            "weight": 1
          }
        ]
      }
    ]
  }
}
```

---

# 4. Validate a permission update

This validates a proposed permission configuration **without changing the blockchain**.

## Exact request body

```json
{
  "action": "validate_permission_update",
  "address": "TYourTronAddress",
  "update": {
    "owner": {
      "id": 0,
      "permission_name": "owner",
      "permission_type": 0,
      "threshold": 2,
      "operations": null,
      "keys": [
        {
          "address": "TSignerA",
          "weight": 1
        },
        {
          "address": "TSignerB",
          "weight": 1
        }
      ]
    },
    "witness": null,
    "active": [
      {
        "permission_name": "payments",
        "permission_type": 2,
        "threshold": 1,
        "operations": "7fff...",
        "keys": [
          {
            "address": "TSignerA",
            "weight": 1
          }
        ]
      }
    ]
  }
}
```

The `update` object represents the proposed permission state.

The adapter checks the TRON-specific rules before allowing the proposal to continue.

This call does **not** broadcast anything.

---

# 5. Add an owner signer

This reads the current owner permission, adds one signer, validates the resulting permission configuration, and returns the proposed configuration.

## Exact request body

```json
{
  "action": "add_owner_key",
  "address": "TYourTronAddress",
  "key": {
    "address": "TNewSignerAddress",
    "weight": 1
  }
}
```

## Fields

| Field | Type | Required |
|---|---|---:|
| `action` | string | yes |
| `address` | string | yes |
| `key.address` | string | yes |
| `key.weight` | integer | yes |

This operation prepares a new permission state.

**It does not itself modify the blockchain.**

---

# 6. Add an active permission

Use this when the application wants a separate active permission with its own signer set and operations bitmap.

## Exact request body

```json
{
  "action": "add_active_permission",
  "address": "TYourTronAddress",
  "permission": {
    "permission_name": "payments",
    "threshold": 2,
    "operations": "7fff...",
    "keys": [
      {
        "address": "TSignerA",
        "weight": 1
      },
      {
        "address": "TSignerB",
        "weight": 1
      }
    ]
  }
}
```

Do not supply an invented active permission ID in this request.

The adapter assigns the active permission type and validates the proposed permission.

The final on-chain permission ID is handled as part of the TRON permission-update process.

---

# 7. Set witness permission

Witness permission is a special TRON permission and is separate from active permissions.

## Exact request body

```json
{
  "action": "set_witness_permission",
  "address": "TYourTronAddress",
  "permission": {
    "permission_name": "witness",
    "threshold": 1,
    "keys": [
      {
        "address": "TWitnessSigner",
        "weight": 1
      }
    ]
  }
}
```

The current implementation requires exactly one witness key.

This also creates a proposal; it does not by itself update the blockchain.

---

# 8. Prepare the permission-update transaction

Once the application has a permission configuration it wants to apply, it can request an unsigned TRON permission-update transaction.

## Exact request body

```json
{
  "action": "prepare_permission_update",
  "address": "TYourTronAddress",
  "update": {
    "owner": {
      "id": 0,
      "permission_name": "owner",
      "permission_type": 0,
      "threshold": 2,
      "operations": null,
      "keys": [
        {
          "address": "TSignerA",
          "weight": 1
        },
        {
          "address": "TSignerB",
          "weight": 1
        }
      ]
    },
    "witness": null,
    "active": [
      {
        "permission_name": "payments",
        "permission_type": 2,
        "threshold": 1,
        "operations": "7fff...",
        "keys": [
          {
            "address": "TSignerA",
            "weight": 1
          }
        ]
      }
    ]
  }
}
```

The adapter communicates with:

```text
POST /wallet/accountpermissionupdate
```

through the RPC orchestrator.

The response contains an unsigned transaction.

**At this point the blockchain has not changed.**

---

# 9. Signing / authorization stage

The application should not treat `prepare_permission_update` as signing.

The intended flow is:

```text
prepare_permission_update
        |
        v
unsigned TRON transaction
        |
        v
TRX signing/authorization layer
        |
        +-- select permission_id
        +-- check operations
        +-- identify authorized signer
        +-- prevent duplicate signer
        +-- calculate collected weight
        +-- check threshold
        +-- create TRON signature
        |
        v
signed transaction
```

For a multisignature permission, the threshold is reached only when the **unique authorized signer weight** is sufficient.

A signer must not be counted twice.

---

# 10. Broadcast a signed transaction

Only call this after the transaction has been properly signed.

## Exact request body

```json
{
  "action": "broadcast_signed_transaction",
  "transaction": {
    "txID": "TRANSACTION_ID",
    "raw_data": {},
    "raw_data_hex": "RAW_TRANSACTION_HEX",
    "signature": [
      "SIGNATURE_HEX"
    ]
  }
}
```

The adapter sends the supplied transaction to:

```text
POST /wallet/broadcasttransaction
```

through the RPC orchestrator.

This action does not create private-key signatures.

## Important

A successful broadcast response means the transaction was submitted to TRON. The application should subsequently call:

```json
{
  "action": "get_permissions",
  "address": "TYourTronAddress"
}
```

to inspect the resulting on-chain permission state after the transaction is confirmed.

---

# 11. Complete add-permission flow

For the UI, adding a permission should be treated as a sequence rather than one unsafe call.

```text
1. get_permissions
       |
       v
2. Build desired permission in UI
       |
       v
3. validate_permission_update
       |
       v
4. prepare_permission_update
       |
       v
5. Sign / authorize transaction
       |
       v
6. broadcast_signed_transaction
       |
       v
7. get_permissions
       |
       v
8. Display confirmed permission state
```

For adding an owner signer:

```text
get_permissions
      |
      v
add_owner_key
      |
      v
validate_permission_update
      |
      v
prepare_permission_update
      |
      v
sign
      |
      v
broadcast
      |
      v
get_permissions
```

For adding an active permission:

```text
get_permissions
      |
      v
add_active_permission
      |
      v
validate_permission_update
      |
      v
prepare_permission_update
      |
      v
sign
      |
      v
broadcast
      |
      v
get_permissions
```

---

# 12. Permission IDs

The TRON permission model uses:

```text
Owner    = 0
Witness  = 1
Active   = 2..9
```

The adapter validates permission IDs and permission types rather than allowing arbitrary numeric permission types to be treated as valid.

---

# 13. Operations bitmap

Active permissions have an operations bitmap.

The application should provide the bitmap as the TRON permission representation expected by the adapter.

The adapter validates the bitmap and maps supported TRON contract types explicitly.

The UI should not be allowed to bypass this by supplying an arbitrary contract-type number and expecting it to be accepted.

---

# 14. Current JSON actions

The current JSON action list is:

```text
get_permissions
validate_permission_update
add_owner_key
add_active_permission
set_witness_permission
prepare_permission_update
broadcast_signed_transaction
```

These names are the action names the application should send in the `action` field.

---

# 15. What this API does NOT do

The permission API does not:

- fetch balances
- fetch TRC-20 balances
- fetch TRC-10 balances
- choose an RPC provider
- hardcode a public TRON RPC URL
- put private keys into permission-inspection requests
- treat a permission proposal as an on-chain change

Those concerns remain separated from the permission JSON API.

---

# 16. Application rule

The UI/orchestrator should never assume that an `add_*` response means the blockchain was changed.

The safe interpretation is:

```text
add_* 
  = build/validate a proposed permission state

prepare_permission_update
  = obtain unsigned on-chain transaction

sign/authorize
  = authorize the transaction

broadcast_signed_transaction
  = submit the authorized transaction

get_permissions
  = verify the resulting on-chain state
```

This separation is intentional and prevents a UI action from accidentally being treated as an immediate blockchain mutation.


---

# 17. Entire TRX adapter: what it actually contains

This section describes the rest of the adapter code, including functions that are not part of the JSON permission API.

## 17.1 Adapter construction

### Rust call

```rust
TronAdapter::new(ChainConfig {
    rpc_url,
    rpc_provider,
    name,
})
```

### What it does

Creates the TRX adapter and stores:

- the configured RPC URL
- the configured RPC provider name
- the chain name
- an HTTP client used by the adapter's existing direct-fetch methods

The permission RPC path uses the shared `RpcOrchestrator`.

---

# 18. Address derivation

TRON addresses are derived from a secp256k1 public key.

The adapter implements:

```rust
WalletAdapter::pub_key_to_address(public_key)
WalletAdapter::derive_address_from_path(seed, path)
```

## 18.1 Public key -> TRON address

The adapter:

1. accepts a compressed 33-byte or uncompressed 65-byte secp256k1 public key;
2. obtains the uncompressed public-key point;
3. removes the public-key prefix byte;
4. calculates Keccak-256;
5. takes the last 20 bytes;
6. prefixes TRON's `0x41` address byte;
7. encodes the result as Base58Check.

TRON's documented HD derivation uses BIP-39/BIP-44 with coin type 195 and a path such as:

```text
m/44'/195'/0'/0/<address-index>
```

The exact seed/mnemonic handling remains in the engine's shared derivation layer; the TRX adapter supplies the TRON address conversion. citeturn0search10

## 18.2 Seed + derivation path

```rust
TronAdapter::derive_address_from_path(seed, path)
```

The adapter:

```text
seed
 ↓
shared master-key derivation
 ↓
shared child-key derivation
 ↓
secp256k1 private key
 ↓
public key
 ↓
Keccak-256
 ↓
TRON 0x41 payload
 ↓
Base58Check
 ↓
T...
```

This is how the engine can derive a TRON destination address without asking the TRON network.

---

# 19. Balance functionality already present in the adapter

Although the permission JSON API deliberately does not fetch balances, the adapter itself currently contains a separate balance implementation.

### Rust call

```rust
fetch_balances(owner_address, assets)
```

It currently handles:

- native TRX
- TRC-10 balances
- TRC-20 balances

## TRX

The adapter reads the account's native `balance` in SUN and normalizes it using the asset's decimals, normally 6 for TRX.

```text
SUN integer
   ↓
exact integer normalization
   ↓
TRX decimal string
```

## TRC-10

The adapter reads `assetV2` from the account response and matches the asset identifier supplied by the asset row.

The adapter recognizes its internal TRC-10 representation:

```text
trc10:<numeric-id>
```

## TRC-20

The adapter calls the token contract using TRON's constant-contract API.

It calculates:

```text
Keccak-256("balanceOf(address)")
        ↓
4-byte function selector
        ↓
ABI-encoded owner address
        ↓
triggerconstantcontract
        ↓
raw token balance
```

It then calls:

```text
Keccak-256("decimals()")
        ↓
4-byte selector
        ↓
triggerconstantcontract
        ↓
token decimals
```

The raw integer is normalized using those decimals.

The TRON developer documentation confirms that TRC-20 contract calls use Ethereum-compatible ABI encoding, with the TRON 0x41 address prefix removed for the ABI's 20-byte address value. citeturn0search7

> The balance path is separate from permission management. The permission JSON actions do not call `fetch_balances`.

---

# 20. Permission inspection

There are two account-permission inspection paths currently present in the adapter.

## Shared RPC path

```rust
inspect_account_permissions_via_rpc(&rpc, address)
```

This is the preferred path for the new permission JSON API.

It sends:

```text
POST /wallet/getaccount
```

through the shared RPC orchestrator.

It extracts:

```text
owner_permission
witness_permission
active_permission
```

TRON's account API exposes these permission structures. citeturn0search2

## Existing direct HTTP helper

```rust
inspect_account_permissions(address)
```

This older helper directly uses the adapter's configured `rpc_url`.

It should not be used as the application's preferred permission path when the shared RPC orchestrator is available, because the architecture requires provider selection to remain in the RPC layer.

---

# 21. Understanding TRON permissions

TRON has three permission types.

## Owner

```text
ID = 0
```

Owner is the account's main authority and can modify the account's permission configuration.

## Witness

```text
ID = 1
```

Witness permission is for Super Representative block production. It is not a normal transaction permission.

The adapter therefore rejects witness permission when validating a normal transaction.

## Active

```text
ID = 2..9
```

An account can have up to eight active permissions.

Each active permission has:

```text
name
threshold
keys
weights
operations bitmap
```

The network assigns active permission IDs sequentially starting at 2. citeturn0search2

---

# 22. How a permission actually works

Imagine:

```text
Active permission
threshold = 3

Alice = weight 2
Bob   = weight 1
Carol = weight 1
```

Alice alone:

```text
2 < 3
```

Not enough.

Bob + Carol:

```text
1 + 1 = 2 < 3
```

Not enough.

Alice + Bob:

```text
2 + 1 = 3
```

Authorized.

TRON's network checks the recovered signer addresses against the permission keys and adds their weights until the threshold is reached. citeturn0search2

The adapter's `has_reached_threshold` helper performs the corresponding local calculation and deliberately deduplicates signer addresses.

---

# 23. Selecting which permission signs a transaction

TRON transactions can specify:

```text
Permission_id
```

The IDs are:

```text
0     Owner
1     Witness
2..9  Active
```

For normal contract transactions, witness permission cannot be used.

An active permission is usable only if its operations bitmap allows the transaction's contract type. citeturn0search0turn0search9

For example:

```text
transaction
   |
   +-- contract type = TransferContract
   |
   +-- Permission_id = 2
             |
             v
       Active permission #2
             |
             +-- signer key?
             +-- sufficient weight?
             +-- TransferContract allowed?
```

---

# 24. Operations bitmap

The operations bitmap is a 32-byte bitmap for an active permission.

Each contract type corresponds to a bit.

The adapter's:

```rust
permission_allows_contract(permission, contract_type)
```

does:

```text
hex bitmap
   ↓
32 bytes
   ↓
contract_type / 8 = byte
contract_type % 8 = bit
   ↓
test bit
```

If the bit is set, that permission can authorize that contract type.

If it is not set, the transaction must not be authorized using that active permission.

TRON's documentation describes this bitmap as the mechanism that restricts active permissions to selected contract types. citeturn0search2

---

# 25. Signer lookup

The adapter provides:

```rust
signer_weight(permission, signer_address)
```

It searches the permission's key list for the signer address and returns its configured weight.

Example:

```text
Alice = 2
Bob   = 1

signer_weight(Alice) = 2
signer_weight(Bob)   = 1
```

If the signer is not in the permission, the result is zero.

---

# 26. Local signing authorization check

The adapter provides:

```rust
validate_permission_for_signing(
    permissions,
    permission_id,
    contract_type,
    signer_address
)
```

It checks:

1. the requested permission exists;
2. witness is not being used for a normal transaction;
3. the active permission ID is valid;
4. the contract type is allowed;
5. the signer exists in that permission;
6. the signer has non-zero weight.

This is an authorization pre-check.

It does **not** itself produce a cryptographic signature.

---

# 27. Threshold calculation

The adapter provides:

```rust
has_reached_threshold(permission, signer_addresses)
```

Example:

```text
threshold = 3

Alice = 2
Bob   = 1
Carol = 1
```

Input:

```text
["Alice", "Bob"]
```

Result:

```text
2 + 1 = 3
threshold reached = true
```

If the input contains:

```text
["Alice", "Alice", "Bob"]
```

Alice is counted only once.

This is important because simply counting signatures would be unsafe.

---

# 28. How the adapter creates a permission on-chain

This is one of the most important concepts.

The adapter does **not** directly mutate the account's permission state.

Instead:

```text
Desired permission
       ↓
validate
       ↓
AccountPermissionUpdateContract
       ↓
unsigned transaction
       ↓
sign with an existing authorized permission
       ↓
broadcast
       ↓
TRON validates it
       ↓
new permission becomes on-chain state
```

TRON's `AccountPermissionUpdateContract` replaces the account's permission configuration. It carries the owner permission, witness permission where applicable, and active permissions. citeturn0search2turn0search8

That is why a permission update is dangerous: the transaction must be authorized by a permission that already exists and remains usable after the change.

---

# 29. Why the permission update must contain the whole configuration

A permission update is not:

```text
"add Alice"
```

It is effectively:

```text
"replace the account's permission configuration with this configuration"
```

Therefore the transaction includes:

```text
owner
witness
actives[]
```

Even if the application only wants to change one active permission, the other required permissions must be supplied correctly. citeturn0search2

This is why the adapter first reads the current permissions before its `add_*` operations.

---

# 30. Example: create a 2-of-3 owner permission

Suppose the current account owner is:

```text
Alice
```

The application wants:

```text
threshold = 2

Alice = 1
Bob   = 1
Carol = 1
```

The conceptual permission is:

```json
{
  "id": 0,
  "permission_name": "owner",
  "permission_type": 0,
  "threshold": 2,
  "operations": null,
  "keys": [
    {"address": "TAlice...", "weight": 1},
    {"address": "TBob...", "weight": 1},
    {"address": "TCarol...", "weight": 1}
  ]
}
```

The application should then:

```text
get current permissions
       ↓
construct desired owner
       ↓
validate_permission_update
       ↓
prepare_permission_update
       ↓
sign using the CURRENT authorized permission
       ↓
broadcast
       ↓
verify with get_permissions
```

The current owner must be able to authorize the permission-update transaction. A badly constructed update can permanently make the account's permissions unusable. TRON explicitly recommends testing permission changes and verifying signer control before submitting them. citeturn0search4

---

# 31. Transaction creation

TRON state-changing operations follow:

```text
construct
   ↓
sign
   ↓
broadcast
```

The adapter's permission-update preparation follows that model.

TRON transaction data contains `raw_data` and `signature[]`. The signature is made over the SHA-256 hash of the serialized transaction raw data. citeturn0search1turn0search5

The current generic adapter trait exposes:

```rust
WalletAdapter::sign_transaction(private_key, tx_data)
```

and the TRX implementation delegates to the engine's shared secp256k1 signer:

```rust
sign_secp256k1_payload(private_key, tx_data)
```

This means the current adapter has the cryptographic signing primitive, but **the permission-aware multisignature orchestration is not yet a complete public JSON action**.

That distinction is important.

---

# 32. Multi-signature signing

For a 2-of-3 permission:

```text
Alice weight 1
Bob   weight 1
Carol weight 1

threshold 2
```

The flow is:

```text
unsigned transaction
       ↓
Alice signs
       ↓
signature[0]
       ↓
Bob signs the SAME transaction
       ↓
signature[0], signature[1]
       ↓
network recovers both signer addresses
       ↓
1 + 1 = 2
       ↓
threshold reached
       ↓
transaction accepted
```

TRON's documented multisignature flow collects signatures over the same transaction ID before broadcasting. citeturn0search1turn0search3

The adapter already contains the permission-selection, signer-weight, contract-authorization, and threshold helper logic needed to support this, but the complete public `collect_signature` / `add_signature` JSON orchestration still needs to be implemented before we should claim multisig signing is finished.

---

# 33. TRX normal send transaction

A normal TRX transfer follows the same transaction lifecycle.

Conceptually:

```text
sender
recipient
amount in SUN
permission_id
       ↓
TRON transfer transaction
       ↓
sign
       ↓
broadcast
```

TRON's transaction creation API uses the sender, recipient, amount in SUN, and optional `Permission_id`. citeturn0search6

For a normal single-key account, permission ID 0 is normally used.

For an active permission, the transaction must specify the appropriate active permission ID and that permission must allow the transfer contract.

**The current generic `WalletAdapter::sign_transaction` can sign transaction bytes, but the TRX adapter does not yet expose a dedicated `send_trx` JSON action in `permission_json`.** That should be added separately rather than pretending `broadcast_signed_transaction` is a complete send-TRX builder.

---

# 34. Security inspection

The adapter also has:

```rust
check_account_security(address)
```

It examines the account's permissions and reports:

- whether permissions look customized;
- whether the configuration is considered a risk by the current heuristic;
- owner threshold;
- owner key count;
- owner/witness/active permission details.

The current heuristic considers custom owner settings, custom active settings, and witness configuration.

This is an inspection/security feature, not a permission-changing function.

---

# 35. Important implementation boundary

The current TRX adapter therefore has four different categories of functionality:

## A. Identity

```text
public key -> TRON address
seed + derivation path -> TRON address
```

## B. Read

```text
account information
permissions
TRX balance
TRC-10 balance
TRC-20 balance
security inspection
```

## C. Permission / transaction preparation

```text
validate permission
add owner key
add active permission
set witness permission
prepare AccountPermissionUpdateContract
check permission_id
check operations
calculate signer weight
check threshold
```

## D. Transaction execution

```text
sign transaction bytes
broadcast signed transaction
```

The permission-aware signing path is now implemented: the caller selects a permission ID, the adapter verifies that the transaction uses the same ID, verifies the signer against the selected permission and operations, signs the transaction, and supports additional authorized signers through the same transaction object.

---

# 36. What the application should ultimately be able to ask TRX

The clean long-term JSON API should conceptually cover:

```text
READ
├── get_permissions
├── get_account
├── check_security
└── get balances

PERMISSION
├── validate_permission
├── add_owner_key
├── add_active_permission
├── set_witness_permission
└── prepare_permission_update

TRANSACTION
├── build/send TRX
├── build token transaction
├── select permission_id
├── sign
├── add signature
├── check threshold
└── broadcast

IDENTITY
└── derive address
```

The existing adapter already covers significant portions of these categories, but the JSON layer currently exposes only the permission/broadcast subset documented above.

---

# 37. The most important mental model

Think of the TRX adapter as the **TRON brain**, not the RPC provider.

The RPC layer answers:

```text
"Where/how do I communicate with TRON?"
```

The TRX adapter answers:

```text
"What does this TRON request mean?"
"Which permission is allowed?"
"Which contract can this permission authorize?"
"Who can sign?"
"How much signer weight is available?"
"Has the threshold been reached?"
"How is this TRON transaction constructed?"
"How is the TRON address derived?"
```

Then the engine coordinates the complete lifecycle:

```text
UI
 ↓
Engine/orchestrator
 ↓
TRX adapter
 ↓
RPC orchestrator
 ↓
configured provider
 ↓
TRON
```

That separation should remain intact as the adapter is completed.


---

# 38. Final transaction JSON API

The transaction layer is now exposed in the same JSON entry point.

## send_trx

Creates an unsigned TRON TRX transfer.

### Exact request

```json
{
  "action": "send_trx",
  "address": "TSenderAddress",
  "to": "TRecipientAddress",
  "amount_sun": 1000000,
  "permission_id": 2
}
```

`amount_sun` is an integer. `1 TRX = 1,000,000 SUN`.

The adapter calls:

```text
/wallet/createtransaction
```

The returned transaction is unsigned. It must go through the signing flow before broadcast.

TRON documents `TransferContract` as contract type 1 and `wallet/createtransaction` as its transaction builder. citeturn2search0

---

# 39. sign_transaction

Signs an existing TRON transaction using a supplied secp256k1 private key.

### Exact request

```json
{
  "action": "sign_transaction",
  "permission_id": 2,
  "private_key": "32_BYTE_PRIVATE_KEY_HEX",
  "transaction": {
    "txID": "...",
    "raw_data": {},
    "raw_data_hex": "...",
    "signature": []
  }
}
```

The adapter:

1. decodes `raw_data_hex`;
2. identifies the transaction contract;
3. reads `Permission_id`;
4. derives the signing address from the private key;
5. loads the account's current permissions through the RPC orchestrator;
6. verifies that the signer belongs to the selected permission;
7. verifies that the active permission allows the contract type;
8. checks the existing signature set through TRON's signature-weight endpoint when signatures already exist;
9. prevents the same signer from signing twice;
10. signs `SHA-256(raw_data)`;
11. creates the canonical TRON `r || s || v` 65-byte signature;
12. appends the signature to `transaction.signature`.

TRON specifies that the signed message is `SHA-256(raw_data)` and that signatures are 65 bytes in `r || s || v` order, with `v` equal to 0 or 1. citeturn2search0turn2search1

**Important:** the private key should normally be supplied only by the trusted signing layer. The UI should not store or transmit private keys unnecessarily.

---

# 40. Multisignature flow

The same `sign_transaction` action can be called by different authorized signers on the same transaction.

Example:

```text
2-of-3 permission

Alice = 1
Bob   = 1
Carol = 1

threshold = 2
```

First call:

```json
{
  "action": "sign_transaction",
  "permission_id": 2,
  "private_key": "ALICE_PRIVATE_KEY",
  "transaction": {
    "raw_data_hex": "...",
    "signature": []
  }
}
```

The returned transaction contains Alice's signature.

Then pass that **same returned transaction** to Bob:

```json
{
  "action": "sign_transaction",
  "permission_id": 2,
  "private_key": "BOB_PRIVATE_KEY",
  "transaction": {
    "raw_data_hex": "...",
    "signature": [
      "ALICE_SIGNATURE..."
    ]
  }
}
```

The result now contains both signatures.

The transaction's `raw_data` is not changed between signatures. Only the `signature[]` array grows. Every signer call uses the same explicit `permission_id`.

TRON's signature model intentionally signs `raw_data`, not the signatures, so additional signatures can be added without changing the transaction payload. citeturn2search1

---

# 41. get_sign_weight

Checks whether the currently collected signatures satisfy the selected permission.

### Exact request

```json
{
  "action": "get_sign_weight",
  "transaction": {
    "txID": "...",
    "raw_data": {},
    "raw_data_hex": "...",
    "signature": [
      "SIGNATURE_A",
      "SIGNATURE_B"
    ]
  }
}
```

The adapter calls:

```text
/wallet/getsignweight
```

The important response fields are:

```json
{
  "approved_list": [],
  "current_weight": 2,
  "permission": {
    "threshold": 2
  },
  "result": {
    "code": "ENOUGH_PERMISSION"
  }
}
```

Possible normal states include:

```text
ENOUGH_PERMISSION
NOT_ENOUGH_PERMISSION
```

TRON's endpoint also rejects duplicate signatures, unknown signers, and invalid signature sets rather than silently assigning them zero weight. citeturn3search3turn3search2

---

# 42. get_approved_list

Returns the signers that TRON recognizes on the supplied partial transaction.

### Exact request

```json
{
  "action": "get_approved_list",
  "transaction": {
    "txID": "...",
    "raw_data": {},
    "raw_data_hex": "...",
    "signature": [
      "SIGNATURE_A"
    ]
  }
}
```

The adapter calls:

```text
/wallet/getapprovedlist
```

This is useful to the UI when displaying:

```text
Alice     ✓
Bob       ✓
Carol     waiting
-------------------
Weight: 2 / 3
```

---

# 43. broadcast_signed_transaction

This remains the final submission step.

### Exact request

```json
{
  "action": "broadcast_signed_transaction",
  "transaction": {
    "txID": "...",
    "raw_data": {},
    "raw_data_hex": "...",
    "signature": [
      "SIGNATURE_A",
      "SIGNATURE_B"
    ]
  }
}
```

The adapter calls:

```text
/wallet/broadcasttransaction
```

A successful broadcast response is **not final confirmation**. The application should continue checking the transaction until it is included and solidified. citeturn3search9

---

# 44. Complete TRX send flow

The final intended application flow is:

```text
send_trx
   ↓
unsigned transaction
   ↓
sign_transaction
   ↓
signature A
   ↓
sign_transaction
   ↓
signature B
   ↓
get_sign_weight
   ↓
ENOUGH_PERMISSION
   ↓
broadcast_signed_transaction
   ↓
query transaction status
   ↓
solidified confirmation
```

For a normal one-key account, only one signing call is needed.

For a multisig account, continue signing until the permission threshold is reached.

---

# 45. Complete permission-change flow

For permission changes:

```text
get_permissions
      ↓
add_owner_key / add_active_permission
      ↓
validate_permission_update
      ↓
prepare_permission_update
      ↓
sign_transaction
      ↓
additional signers if threshold requires them
      ↓
get_sign_weight
      ↓
ENOUGH_PERMISSION
      ↓
broadcast_signed_transaction
      ↓
get_permissions
      ↓
verify new permission state
```

TRON's `AccountPermissionUpdateContract` replaces the complete permission configuration. Therefore the adapter keeps the existing permissions and changes the requested portion before constructing the update. citeturn4search0turn4search1

---

# 46. Active permission example

A useful security design is:

```text
Owner
2-of-3
Alice 1
Bob   1
Carol 1

Active #2
1-of-1
DailyHotKey 1

operations:
TransferContract only
```

The hot key can perform TRX transfers, but it cannot use the same active permission to perform operations whose contract types are not enabled.

TRON defines `TransferContract` as ID 1 and `AccountPermissionUpdateContract` as ID 46. citeturn1search0turn1search1

This allows the UI to offer a safer model:

```text
Daily transfers
    ↓
Active permission

Changing wallet permissions
    ↓
Owner permission
    ↓
2-of-3 approval
```

---

# 47. Final JSON action list

The implemented TRX JSON entry point covers:

```text
READ
├── get_permissions

PERMISSION
├── validate_permission_update
├── add_owner_key
├── add_active_permission
├── set_witness_permission
└── prepare_permission_update

TRANSACTION
├── send_trx
├── sign_transaction
├── get_sign_weight
├── get_approved_list
└── broadcast_signed_transaction
```

Balance retrieval remains separate from this permission/transaction API.

---

# 48. Important security rule

Never treat:

```text
sign_transaction = broadcast
```

They are separate.

The correct sequence is:

```text
build
  ↓
validate
  ↓
sign
  ↓
check weight
  ↓
broadcast
  ↓
confirm
```

This allows multisignature signers to exchange the same transaction without changing its signed payload and allows the application to stop before broadcasting if the collected weight is insufficient. citeturn2search1turn3search0


---

# 49. Permission ID is explicit during signing

The application must explicitly provide the permission it intends to use when signing.

### Request

```json
{
  "action": "sign_transaction",
  "permission_id": 2,
  "private_key": "32_BYTE_PRIVATE_KEY_HEX",
  "transaction": {
    "raw_data_hex": "...",
    "signature": []
  }
}
```

The adapter compares:

```text
request.permission_id
        ==
transaction Permission_id
```

If they differ, signing is rejected.

This prevents the application from accidentally presenting a transaction prepared for one permission while asking the adapter to authorize it with another.

The adapter then derives the signer address from the private key and verifies that the signer belongs to the selected permission and that the selected permission can authorize the transaction's contract type.

For ordinary transactions, permission ID 1 (Witness) is rejected.

The application should therefore treat the permission ID as part of the transaction authorization decision, not as a cosmetic field.

---

# 50. Recommended application sequence

For any transaction that should use a particular permission:

```text
1. get_permissions
2. choose permission_id
3. build transaction with that Permission_id
4. sign_transaction(permission_id + private key + transaction)
5. repeat step 4 for additional authorized signers
6. get_sign_weight
7. broadcast_signed_transaction
8. confirm transaction
```

For a single-key Active permission:

```text
permission #2
threshold = 1
signer weight = 1

one successful sign
        ↓
ENOUGH_PERMISSION
        ↓
broadcast
```

For a 2-of-3 permission:

```text
permission #2
threshold = 2

signer A → weight 1
signer B → weight 1

A + B = 2
       ↓
ENOUGH_PERMISSION
       ↓
broadcast
```

The same unsigned transaction must be passed between signers. Each signer adds a signature; the transaction's `raw_data` does not change.



## Current unified transaction boundary

The transaction service now treats the transaction message as completely opaque.

The application sends the transaction executor:

```json
{
  "action": "sign_transaction",
  "chain_type": "tron",
  "message": {
    "sub_action": "sign_token_transaction",
    "wallet_id": "wallet_1",
    "blockchain": "tron",
    "chain_id": 0,
    "token": "USDT",
        "to": "T...",
    "token_contract": "T...",
    "amount": "1000000",
    "fee_limit": 100000000
  }
}
```

The transaction service does **not** read, validate, split, construct, sign, or broadcast anything inside `message`. It forwards:

```text
action + message
      ↓
AdapterRegistry
      ↓
TRON adapter
```

The TRON adapter owns the complete lifecycle:

```text
message
  ↓
interpret operation
  ↓
validate TRON-specific rules
  ↓
construct transaction
  ↓
sign transaction
  ↓
permission/signature validation
  ↓
broadcast signed transaction
  ↓
return adapter response
```

For a native TRX transfer:

```json
{
  "action": "sign_transaction",
  "chain_type": "tron",
  "message": {
    "sub_action": "sign_network_transaction",
    "wallet_id": "wallet_1",
    "blockchain": "tron",
    "chain_id": 0,
        "to": "T...",
    "amount_sun": 1000000,
    "permission_id": 0
  }
}
```

For a TRC-20 token transfer:

```json
{
  "action": "sign_transaction",
  "chain_type": "tron",
  "message": {
    "operation": "token_transfer",
        "to": "T...",
    "token_contract": "T...",
    "amount": "1000000",
    "fee_limit": 100000000
  }
}
```

The adapter constructs the appropriate TRON transaction, performs the permission-aware signing checks, signs it, broadcasts it through the configured RPC provider, and returns the final adapter response.

A future batch transaction can keep the same boundary:

```json
{
  "action": "sign_transaction",
  "chain_type": "tron",
  "message": {
    "sub_action": "sign_batch_transaction",
    "wallet_id": "wallet_1",
    "blockchain": "tron",
    "chain_id": 0,
    "transfers": [
      { "...": "adapter-specific transfer 1" },
      { "...": "adapter-specific transfer 2" }
    ]
  }
}
```

The transaction service must never inspect the batch contents. If TRON does not support the requested batch semantics, the TRON adapter returns the error.

This is the required extensibility boundary for all adapters: adding a new chain or transaction type must not require adding chain-specific logic to the transaction service.

---

## Engine adapter-call boundary

The public engine entry point for adapter-specific operations is:

```json
{
  "action": "adapter_call",
  "params": {
    "chain_type": "tron",
    "wallet_id": "wallet_1",
    "body": {
      "action": "get_permissions",
      "address": "T..."
    }
  }
}
```

`WalletEngine` does not interpret the TRON body. It resolves the selected wallet's saved blockchain record, obtains its saved RPC provider and RPC URL, and forwards the opaque body to the generic `AdapterOrchestrator`.

The flow is:

```text
WalletEngine
  -> AdapterOrchestrator
  -> AdapterRegistry
  -> TRX adapter
  -> RPC Orchestrator
  -> saved TRX RPC provider
```

The same boundary is intended for future adapter operations such as address creation/derivation, balance fetching and normalization, transaction preparation, signing, permission management and broadcasting. Each adapter owns the chain-specific JSON contract; WalletEngine remains chain-agnostic.

The saved blockchain record now supports `rpc_provider` and `rpc_url`, so the application does not need to hardcode a public TRON endpoint and does not need to repeat RPC configuration in every adapter call.


## Transaction signing and send semantics

TRON uses secp256k1 transaction signatures. The signing operation is the same cryptographic primitive for ordinary transfers, contract calls, staking, and permission updates; authorization is selected by the transaction's `Permission_id`.

Supported permission IDs in the adapter:
- `0`: owner permission.
- `2` through `9`: active permissions.
- `1` (witness) is rejected for ordinary wallet transaction signing.

For multi-signature accounts, the same transaction may carry multiple signatures. Each signer must belong to the selected permission, must be authorized for the transaction's contract type, and must not sign the same transaction twice. The adapter validates the existing signature set before appending a new signature. The transaction is ready for broadcast only when the selected permission's accumulated signer weight reaches its threshold.

The transaction service keeps signing and broadcasting separate:
1. build/obtain an unsigned transaction;
2. call the adapter signing operation;
3. optionally append additional signatures;
4. broadcast the fully authorized transaction.

### Unified send action

The TRON permission API accepts a `send` action. A single output is treated as a native TRX `TransferContract`.

If more than one output is supplied, the adapter returns an explicit unsupported error. Native TRON transactions do not provide a generic multi-recipient `TransferContract` batch. A true one-transaction multi-recipient transfer requires a configured smart-contract batch mechanism, which is intentionally not assumed by the engine.

Example single send request:

```json
{
  "action": "send",
  "address": "T...",
  "permission_id": 0,
  "to": "T...",
  "amount_sun": 50000000
}
```

Example requested batch:

```json
{
  "action": "send",
  "address": "T...",
  "permission_id": 0,
  "outputs": [
    {"to": "T...", "amount_sun": 50000000},
    {"to": "T...", "amount_sun": 70000000}
  ]
}
```

The second request is deliberately rejected rather than silently converting it into an arbitrary contract call.
