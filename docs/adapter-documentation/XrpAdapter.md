# XRP Adapter

## Scope

XRP Ledger uses one XRP adapter. The wallet uses secp256k1 and derives the classic XRPL account address from the compressed public key.

## Derivation

Default path: `m/44'/144'/0'/0/0`. The adapter requires a path beginning with `m/44'/144'/account'/change/index`.

## Address

`secp256k1 public key → HASH160 → XRPL Base58Check → r... address`.

## Signing

The adapter uses the existing secp256k1 signer. Transaction serialization/encoding and ledger submission remain outside the adapter.

## Balance normalization

The adapter does not fetch balances. The XRP provider retrieves raw XRP drops and issued-token balances. `XrpAdapter::normalize_balances(...)` converts native XRP drops using 6 decimals; issued-token precision comes from provider metadata.

## Account activation

Address derivation is local. The adapter does not fund or activate an account. XRPL reserve and activation requirements belong to the provider/network transaction layer.

## Provider boundary

The provider owns XRPL RPC/JSON-RPC/WebSocket communication, endpoint fallback, `account_info`, token retrieval, transaction submission, confirmation, reserve/fee queries, and ledger security checks.

## Security checks

Master-key-disabled state, regular-key presence, and signer-list inspection require ledger queries and therefore belong in the provider/service layer, not the adapter.

## Developer flow

Wallet: `seed → BIP-44 path → secp256k1 key → compressed public key → HASH160 → XRPL address`.

Balance: `XRP Provider → raw balances → XrpAdapter::normalize_balances(...) → common balance representation`.

Send: `transaction builder → canonical XRPL serialization → secp256k1 signing → provider submission → ledger confirmation`.

## Important rules

- Keep one XRP adapter.
- Require the derivation path.
- Do not fetch balances inside the adapter.
- Do not keep an RPC client inside the adapter.
- Do not activate/fund accounts during local wallet generation.


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


## Security and Permission Scanning

XrpAdapter::scan_security(account, chain) performs a read-only on-ledger security scan against the XRP RPC URL contained in the saved XRP ChainConfig.

The scan requests account_info with ledger_index=validated and signer_lists=true. XRPL documents account_info as the source for AccountRoot data, account flags, and the account's SignerList. citeturn0search1

The scanner detects:

- Master-key state — reads lsfDisableMaster.
- Regular key — reads AccountRoot.RegularKey.
- Multi-signing — reads quorum, signer addresses, weights, and optional wallet locators.
- Account security/configuration flags — DepositAuth, DisableMaster, RequireDestTag, RequireAuth, DefaultRipple, NoFreeze, GlobalFreeze, DisallowXRP, incoming-transaction restrictions, Clawback, Trust Line Locking, and PasswordSpent.
- Permission delegation — queries Delegate ledger entries and returns every delegate and its permissions.

XRPL SignerLists provide weighted M-of-N authorization. citeturn0search4turn0search5 Permission delegation stores permissions in Delegate ledger entries and supports transaction-type and granular permissions. citeturn0search8turn0search3

The result is normalized as:

account → flags → master-key → regular-key → signer-list → delegates → authorization summary

This lets the engine determine whether an account has a master key, regular key, multisigning, delegates, what quorum is required, and which security flags are enabled.

### RPC boundary

The scanner receives the target XRP ChainConfig and uses its saved RPC URL:

XRP JSON → target XRP adapter → saved RPC URL → validated ledger → security scan → XrpSecurityScan

No XRP RPC URL is hardcoded.

If account_info returns actNotFound, the scanner reports account_exists=false instead of treating the cryptographically valid address as an error. XRPL accounts are created when enough XRP is sent to a valid address. citeturn0search6

### Read-only phase

This phase intentionally does not modify the account. It does not yet change RegularKey, create/update/remove SignerList, enable/disable the master key, change AccountSet security flags, create/update/revoke DelegateSet permissions, or submit transactions.

Those permission-management functions will be implemented in the next phase using the same target-adapter/RPC boundary.


## Security-operation implementation

The adapter now provides a validated prepare_security_operation(...) boundary. The application sends an XRP security request with an action name and structured parameters; the adapter maps that request to the correct XRPL transaction type instead of allowing callers to submit arbitrary transaction JSON.

Supported actions:

- set_regular_key / remove_regular_key → SetRegularKey
- disable_master_key / enable_master_key → AccountSet
- set_account_flag / clear_account_flag → AccountSet
- set_signer_list / remove_signer_list → SignerListSet
- set_delegate / update_delegate / revoke_delegate → DelegateSet
- authorize_depositor / revoke_depositor → DepositPreauth

Example request:

    { "account": "rAccount", "action": "set_account_flag", "params": { "flag": "deposit_authorization" } }

The adapter translates friendly flag names to the correct XRPL SetFlag/ClearFlag values. Callers cannot supply arbitrary numeric flag values.

Signer lists are validated to contain 1–32 signers, positive weights, and a non-zero quorum. Delegate permissions are limited to 10 and are checked against the adapter's supported XRPL permission registry.

The adapter does not hold private keys and does not sign or broadcast the resulting transaction.

### Signing and broadcast boundary

adapter target → prepare_security_operation → unsigned XRPL transaction → local signer → signed tx_blob → XRP provider → submit → validated result → scan_security

The local signer remains responsible for private-key access. The XRP RPC/provider remains responsible for sequence, fee, ledger data, submission, and validation. After a validated submission, the engine should call scan_security again and return the new on-ledger state.


## User-confirmed security transaction flow

Security-changing XRP operations use a mandatory two-confirmation workflow. The adapter must never silently sign or broadcast an account-security transaction.

### Step 1 — Prepare and ask the user

The application sends the adapter-target request. The adapter validates it and constructs the exact XRPL transaction, but stops at `AwaitingUserConfirmation`.

Flow:

`adapter target → validate → build unsigned transaction → show transaction summary → user confirms`

The confirmation UI should clearly identify the XRP account, requested security action, affected address/permission, and the exact transaction type/fields that will be signed.

### Step 2 — Local signing

Only after the first explicit user confirmation does the local signing layer access the user's key and sign the prepared transaction. The XRP adapter never receives the private key.

Flow:

`user confirmation #1 → local signer → signed tx_blob`

The adapter then returns an `XrpSignedSecurityOperation` in `AwaitingBroadcastConfirmation` state.

### Step 3 — Ask again before broadcast

Signing is deliberately not equivalent to broadcasting. The application must show a second confirmation, for example:

`Transaction has been signed. Broadcast this signed XRP transaction to the network?`

Only an explicit affirmative response may call the broadcast/submit layer.

Flow:

`signed tx_blob → show second confirmation → user confirms → XRP RPC provider submit`

If the user rejects the second confirmation, the signed blob must not be submitted.

### Step 4 — Validate on-ledger

After submission, the provider monitors the transaction until the ledger result is validated. The engine then runs `scan_security` again so the returned state comes from the actual XRP ledger rather than assuming that submission means the permission changed.

Final flow:

`prepare → confirm → local sign → confirm again → broadcast → validated → security rescan`

This separation is intentional: **prepare is not approval, signing is not broadcasting, and submission is not validation.**


## Execution pipeline implemented

The XRP RPC layer now has a dedicated shared XrpProvider, registered in the RPC provider registry. XRP adapter transaction execution therefore uses the same RPC boundary as the rest of the engine rather than creating a second RPC client inside the adapter.

The complete security-operation sequence is now:

1. Prepare — prepare_security_transaction asks the XRP RPC provider for validated account_info and fee, then adds the current Sequence, Fee, and LastLedgerSequence to the adapter-built transaction.
2. First confirmation — the application displays the complete unsigned transaction and waits for explicit user confirmation.
3. Local signing — the local signing layer signs the prepared transaction. The adapter never receives a private key.
4. Second confirmation — the application receives the signed transaction state and explicitly asks whether it should be broadcast.
5. Broadcast — broadcast_security_operation sends only the signed tx_blob through the shared XRP RPC provider after the second confirmation.
6. Validation — wait_for_validated polls XRPL tx until the transaction is marked validated.
7. Rescan — after validation, the application should call scan_security again and return the actual new on-ledger security state.

### RPC provider contract

The saved XRP RPC URL is passed as the RpcRequest endpoint to the registered xrp provider. The adapter does not choose an endpoint or bypass the RPC provider registry.

The provider supports the XRP JSON-RPC methods required by this flow, including account_info, fee, submit, and tx.

### State boundary

The security state machine is intentionally:

Prepared → AwaitingUserConfirmation → Signed → AwaitingBroadcastConfirmation → Submitted → Validated

A rejection stops the flow. Signing alone never broadcasts, and successful submission alone is never treated as a final security-state change; validation and a fresh security scan are required.
