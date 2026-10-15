# UTXO Adapter

## Adapter communication contract

The UTXO adapter is never called directly by the Wallet Engine. The application communicates through the shared `adapter_target` envelope.

The routing contract is:

```text
application
    ↓
Wallet Engine
    ↓
action = adapter_target
    ↓
Adapter Orchestrator
    ↓
Adapter Registry
    ↓
chain_type = utxo
    ↓
UTXO Adapter
    ↓
body.action
```

The responsibilities are deliberately separated:

- `action: "adapter_target"` tells the Wallet Engine/Adapter Orchestrator that this is an adapter request.
- `chain_type: "utxo"` identifies the target adapter.
- `blockchain` contains the complete blockchain configuration required by that adapter.
- `body.action` identifies the operation the UTXO adapter must execute.
- The Wallet Engine must not contain UTXO-specific operation names or derivation/address rules.

### Canonical request shape

```json
{
  "action": "adapter_target",
  "targets": [
    {
      "chain_type": "utxo",
      "blockchain": {
        "name": "Bitcoin",
        "symbol": "BTC",
        "decimals": 8,
        "chain_id": "bitcoin-mainnet",
        "derivation_path": "m/44'/0'/0'/0/0",
        "rpc_provider": "provider_name",
        "rpc_url": "https://example-rpc"
      },
      "body": {
        "action": "derive_address"
      }
    }
  ]
}
```

The Adapter Orchestrator forwards the target without interpreting UTXO-specific fields. The UTXO adapter interprets `blockchain` and `body`.

### The adapter response

The adapter returns a response for each target. The response must retain enough target identity for the calling orchestrator to associate the result with the correct wallet/blockchain. The Wallet Engine then performs storage updates; it does not reinterpret UTXO-specific cryptographic data.

### Batch communication

`targets` is the batch boundary. Multiple UTXO blockchains can be included in one request, and the Adapter Orchestrator can route all `chain_type: "utxo"` targets to the same adapter. Each target remains independent because its own `blockchain` object carries its derivation path, network/address parameters, RPC configuration, and identity.

For example, one request can contain Bitcoin and Litecoin targets without the Wallet Engine having separate Bitcoin/Litecoin functions.

### Action selection

The adapter operation is selected only by `body.action`. Examples documented by this adapter are:

```text
body.action = "derive_address"
body.action = "normalize"
body.action = "sign_transaction"
```

The adapter may expose additional operations as its implementation grows. New UTXO operations should be added to the adapter contract rather than hardcoded into the Wallet Engine.

### Important: `derivation_path` is part of the blockchain target

For UTXO derivation, `derivation_path` belongs inside `blockchain`. It is not a top-level adapter command parameter and is not inferred by the Wallet Engine.


## Adapter communication contract

The UTXO adapter is never called directly by the Wallet Engine. The application communicates through the shared `adapter_target` envelope.

```text
application
    ↓
Wallet Engine
    ↓
action = adapter_target
    ↓
Adapter Orchestrator
    ↓
Adapter Registry
    ↓
chain_type = utxo
    ↓
UTXO Adapter
    ↓
body.action
```

Responsibilities are separated:

- `action: "adapter_target"` identifies an adapter request.
- `chain_type: "utxo"` identifies the target adapter.
- `blockchain` contains the complete blockchain configuration.
- `body.action` identifies the operation the UTXO adapter executes.
- The Wallet Engine does not implement UTXO-specific derivation, address encoding, or transaction rules.

### Canonical request

```json
{
  "action": "adapter_target",
  "targets": [
    {
      "chain_type": "utxo",
      "blockchain": {
        "name": "Bitcoin",
        "symbol": "BTC",
        "decimals": 8,
        "chain_id": "bitcoin-mainnet",
        "derivation_path": "m/44'/0'/0'/0/0",
        "rpc_provider": "provider_name",
        "rpc_url": "https://example-rpc"
      },
      "body": {
        "action": "derive_address"
      }
    }
  ]
}
```

The Adapter Orchestrator routes the target by `chain_type`. It does not need to understand what `derive_address` means. The UTXO adapter interprets the `blockchain` and `body` objects.

### Batch requests

`targets` is the batch boundary. Multiple UTXO blockchains can be sent together. Each target remains independent because its own `blockchain` object contains its derivation path, network/address parameters, RPC configuration, and identity.

The Wallet Engine therefore does not need Bitcoin-specific or Litecoin-specific functions.

### Operation selection

The adapter operation is selected by `body.action`, for example:

```text
derive_address
normalize
sign_transaction
```

New UTXO operations belong in the adapter contract, not as hardcoded operations in the Wallet Engine.

### Response routing

The adapter returns a result for each target. The calling orchestrator uses the target identity to associate each result with the correct wallet/blockchain, then the Wallet Engine performs the appropriate storage update. The engine does not reinterpret UTXO-specific cryptographic rules.

### Derivation-path rule

For UTXO address derivation, `derivation_path` MUST be inside `blockchain`. It is not a top-level adapter parameter and is never silently invented by the Wallet Engine.

## Purpose

`UtxoAdapter` is the adapter responsible for UTXO-family blockchains. The application does not call Bitcoin, Litecoin, Dogecoin, or another UTXO implementation directly. It sends an `adapter_target` request with `chain_type: "utxo"` and the UTXO blockchain configuration.

The wallet engine is intentionally unaware of UTXO-specific derivation, address encoding, transaction construction, or balance normalization rules.

## Important developer rule: derivation_path is required

For **every UTXO address derivation request**, the blockchain object MUST contain `derivation_path`.

Do not rely on a hidden default path.

Example:

```json
{
  "action": "adapter_target",
  "targets": [
    {
      "chain_type": "utxo",
      "blockchain": {
        "name": "Bitcoin",
        "symbol": "BTC",
        "decimals": 8,
        "chain_id": "bitcoin-mainnet",
        "derivation_path": "m/44'/0'/0'/0/0",
        "rpc_provider": "provider_name",
        "rpc_url": "https://example-rpc"
      },
      "body": {
        "action": "derive_address"
      }
    }
  ]
}
```

The important point is that the **blockchain JSON carries the derivation path**. The wallet engine forwards it. The UTXO adapter consumes it.

If the path is missing or empty, address derivation must fail with a validation error.

This rule applies even to known chains such as Bitcoin, Litecoin, Dogecoin, and Bitcoin Cash. The adapter may know common paths as reference/policy information, but it must not silently select one for the caller.

## Why the path belongs in blockchain JSON

A UTXO ecosystem contains many independent blockchains. Different chains can use different BIP44 coin types, account conventions, address formats, and network parameters.

The generic adapter therefore receives the complete blockchain configuration instead of maintaining an exhaustive list of supported coins.

For example:

```json
{
  "name": "ExampleCoin",
  "symbol": "EXC",
  "decimals": 8,
  "chain_id": "example-mainnet",
  "derivation_path": "m/44'/1234'/0'/0/0",
  "address_type": "p2wpkh",
  "p2pkh_version": 30,
  "bech32_hrp": "exc"
}
```

The adapter can derive the address without a new adapter module for ExampleCoin.

## Functions

### 1. derive_address_from_path

**Purpose:** Derive one UTXO address from the wallet seed and the exact derivation path supplied by the caller.

Conceptually:

```text
wallet seed
    ↓
derivation_path
    ↓
master key
    ↓
child key
    ↓
compressed public key
    ↓
selected UTXO address format
    ↓
address
```

The function does not invent the derivation path.

The path comes from:

```text
blockchain.derivation_path
```

Example request:

```json
{
  "chain_type": "utxo",
  "blockchain": {
    "name": "Bitcoin",
    "symbol": "BTC",
    "chain_id": "bitcoin-mainnet",
    "derivation_path": "m/44'/0'/0'/0/0"
  },
  "body": {
    "action": "derive_address"
  }
}
```

The response should contain the derived address and the blockchain identity required by the wallet-creation flow.

### 2. pub_key_to_address

**Purpose:** Convert a public key into the selected UTXO address representation.

The adapter uses the blockchain's address policy rather than the wallet engine deciding how a public key becomes an address.

The supported address policies currently include:

- `p2pkh`
- `p2wpkh`
- `p2sh-p2wpkh`

#### P2PKH

The compressed public key is hashed and encoded using the configured P2PKH network version.

This produces the traditional Base58Check-style address.

#### P2WPKH

The public key is converted to HASH160 and placed into a version-0 native SegWit witness program, encoded using the configured Bech32 human-readable prefix.

#### P2SH-P2WPKH

The adapter constructs the nested SegWit redeem script and encodes the resulting script hash as a P2SH address.

The address type must match the blockchain's actual supported address format.

## 3. address_from_private_key

**Purpose:** Recover the UTXO address associated with an existing private key.

The private key is converted into the corresponding public key and then processed through the same UTXO address policy.

The wallet engine should not implement this logic itself.

## 4. sign_transaction

There is an important distinction here.

The adapter has a low-level signing primitive capable of signing supplied transaction/message bytes with a secp256k1 private key.

That does **not** mean the current generic function is a complete Bitcoin/UTXO transaction-signing system.

A real UTXO transaction requires:

1. Selecting the UTXOs to spend.
2. Creating transaction inputs.
3. Creating outputs.
4. Calculating fees.
5. Creating the correct change output.
6. Determining the script/address type for each input.
7. Calculating the correct signature hash.
8. Producing the required signature.
9. Building the appropriate scriptSig and/or witness.
10. Finalizing the transaction.
11. Serializing the final transaction.
12. Broadcasting it through the RPC/provider layer.

Therefore developers MUST NOT treat the generic signing primitive as a complete `send_transaction` implementation.

## Production signing boundary: PSBT

The production UTXO signing interface should use PSBT where the target chain supports it.

The standard PSBT workflow separates transaction creation, updating with UTXO/script information, signing, finalization, and extraction. Bitcoin Core documents these roles and the corresponding workflow. urlBitcoin Core PSBT documentationhttps://github.com/bitcoin/bitcoin/blob/master/doc/psbt.md

The application-level flow should eventually look like:

```text
create transaction
      ↓
create PSBT
      ↓
add UTXO/input metadata
      ↓
adapter signs applicable inputs
      ↓
partial signatures returned
      ↓
combine/finalize
      ↓
extract raw transaction
      ↓
RPC provider
      ↓
broadcast
```

For multisignature transactions, multiple signers can contribute partial signatures to the same PSBT before finalization. BIP174 defines the partial-signature and derivation-path information carried per input. urlBIP-174 PSBT specificationhttps://github.com/bitcoin/bips/blob/master/bip-0174.mediawiki

## 5. normalize_balance

**Purpose:** Convert a raw UTXO amount into the human-readable balance representation used by the wallet engine.

The RPC/provider layer is responsible for obtaining the raw blockchain value.

The adapter is responsible for normalization.

Example:

```text
raw value: 123456789
decimals: 8

normalized:
1.23456789
```

The adapter must not confuse:

- raw base units;
- normalized display balance.

The normalized result is what the balance flow stores for UI consumption.

## 6. UTXO balance fetching

Balance retrieval follows the application's adapter-target architecture.

The application does not call the UTXO adapter with a special Bitcoin-only method.

It sends an adapter target:

```json
{
  "action": "adapter_target",
  "targets": [
    {
      "chain_type": "utxo",
      "blockchain": {
        "name": "Bitcoin",
        "symbol": "BTC",
        "decimals": 8,
        "chain_id": "bitcoin-mainnet",
        "rpc_provider": "provider_name",
        "rpc_url": "https://example-rpc",
        "destination_address": "..."
      },
      "body": {
        "action": "normalize",
        "balance": "123456789"
      }
    }
  ]
}
```

The adapter orchestrator routes the target by `chain_type`.

The UTXO adapter normalizes the balance and returns the normalized JSON.

The wallet engine then updates the corresponding blockchain balance in local storage.

## 7. Adapter target routing

The complete routing model is:

```text
UI / application
      ↓
Wallet Engine
      ↓
adapter_target
      ↓
Adapter Orchestrator
      ↓
Adapter Registry
      ↓
chain_type = utxo
      ↓
UtxoAdapter
      ↓
function selected by body.action
```

The wallet engine does not need to know:

- Bitcoin's derivation rules;
- Litecoin's address encoding;
- Dogecoin's address version;
- another UTXO chain's Bech32 prefix;
- how a UTXO address is constructed;
- how UTXO balances are normalized.

Those responsibilities belong to the adapter.

## 8. Multiple UTXO blockchains

Multiple UTXO targets can be sent in one adapter-target request.

```json
{
  "action": "adapter_target",
  "targets": [
    {
      "chain_type": "utxo",
      "blockchain": {
        "name": "Bitcoin",
        "symbol": "BTC",
        "decimals": 8,
        "chain_id": "bitcoin-mainnet",
        "derivation_path": "m/44'/0'/0'/0/0",
        "rpc_provider": "provider_name",
        "rpc_url": "https://btc-rpc"
      },
      "body": {
        "action": "derive_address"
      }
    },
    {
      "chain_type": "utxo",
      "blockchain": {
        "name": "Litecoin",
        "symbol": "LTC",
        "decimals": 8,
        "chain_id": "litecoin-mainnet",
        "derivation_path": "m/44'/2'/0'/0/0",
        "rpc_provider": "provider_name",
        "rpc_url": "https://ltc-rpc"
      },
      "body": {
        "action": "derive_address"
      }
    }
  ]
}
```

The adapter orchestrator can batch the targets because they have the same `chain_type`.

The adapter itself interprets each blockchain's configuration independently.

## 9. What must be saved with a UTXO blockchain

When the wallet engine saves a blockchain, it should retain the information needed later by the adapter.

At minimum:

```json
{
  "name": "Bitcoin",
  "symbol": "BTC",
  "decimals": 8,
  "chain_type": "utxo",
  "chain_id": "bitcoin-mainnet",
  "destination_address": "...",
  "derivation_path": "m/44'/0'/0'/0/0",
  "rpc_provider": "provider_name",
  "rpc_url": "https://example-rpc"
}
```

If the blockchain requires custom address metadata, retain that too, for example:

```json
{
  "address_type": "p2wpkh",
  "p2pkh_version": 30,
  "bech32_hrp": "exc"
}
```

This means a later balance refresh does not need the UI to reconstruct blockchain configuration.

## 10. Error behavior

The adapter should fail clearly rather than guessing.

Examples:

```text
missing derivation_path
→ validation error

invalid derivation_path
→ derivation/validation error

unsupported address_type
→ validation error

p2wpkh without bech32_hrp
→ validation error

unsupported network/address parameters
→ validation error

invalid private key
→ crypto validation error
```

The adapter must never silently switch to another derivation path because the caller omitted one.

## 11. Security

Private keys and seeds must remain inside the wallet/cryptographic boundary.

The application should provide the adapter with the minimum secret material required for the requested operation.

The adapter must not place private keys, seed phrases, or raw secret material into:

- balance JSON;
- blockchain configuration JSON;
- UI events;
- RPC request logs;
- normal adapter responses.

## 12. What is implemented versus future work

### Implemented

- Generic UTXO adapter routing.
- Explicit derivation-path based address derivation.
- UTXO public-key/address conversion.
- P2PKH.
- P2WPKH where the network provides a Bech32 HRP.
- P2SH-P2WPKH for configured networks.
- Private-key-to-address conversion.
- Raw balance normalization.
- Adapter-target routing.
- Generic UTXO configuration.

### Still a separate transaction implementation

- UTXO selection.
- Fee calculation.
- Change selection.
- PSBT creation.
- PSBT signing.
- PSBT combination.
- PSBT finalization.
- Raw transaction extraction.
- Transaction broadcast.
- Taproot/BIP86 transaction signing.
- Full UTXO multisig finalization.

These must be implemented explicitly rather than pretending that the generic `sign_transaction` primitive performs the complete UTXO transaction lifecycle.

## Developer rule summary

For UTXO wallet creation:

```text
1. App requests wallet creation.
2. Wallet creation orchestrator creates the wallet/session.
3. It creates adapter_target requests.
4. Every UTXO blockchain includes:
      name
      symbol
      decimals
      chain_type
      chain_id
      derivation_path
      RPC configuration
5. Adapter orchestrator routes chain_type = utxo.
6. UtxoAdapter receives the complete blockchain object.
7. UtxoAdapter uses the supplied derivation_path.
8. UtxoAdapter derives the address.
9. Wallet engine saves the resulting address with the blockchain JSON.
```

**Do not omit `derivation_path` for UTXO derivation requests.**

The only intentional exception in the wider wallet architecture is EVM, whose derivation policy is handled differently as an ecosystem-level adapter policy.
