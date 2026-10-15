# Wallet Creation Flow

This is the developer-facing contract for wallet creation and related wallet lifecycle operations.

**Important:** WalletEngine owns wallet naming and wallet-session state. The UI does not decide the persistent wallet name.

## 1. Create a new wallet

### Request

~~~json
{
  "action": "create_wallet",
  "params": {
    "word_count": 12,
    "pin": "user-secret",
    "chains": [
      {
        "name": "Ethereum",
        "symbol": "ETH",
        "decimals": 18,
        "chain_type": "evm",
        "chain_id": "1",
        "rpc_provider": "provider_name",
        "rpc_url": "https://..."
      },
      {
        "name": "TRON",
        "symbol": "TRX",
        "decimals": 6,
        "chain_type": "tron",
        "chain_id": "tron-mainnet",
        "rpc_provider": "provider_name",
        "rpc_url": "https://..."
      }
    ]
  }
}
~~~

Required: word_count, pin/password, and chains. The blockchain object contains name, symbol, decimals, chain_type, chain_id, RPC provider, and RPC URL.

The UI does not need to provide the persistent wallet name. The engine assigns Wallet 1, Wallet 2, Wallet 3, and internal IDs wallet_1, wallet_2, wallet_3. The registry is authoritative for numbering.

**EVM derivation rule:** the UI does not provide `derivation_path` for EVM. The EVM adapter owns the canonical path `m/44'/60'/0'/0/0`, derives the EVM account once, and the wallet creation flow reuses that address for every requested EVM blockchain.

### Flow

~~~text
UI → WalletEngine → Wallet Creation Orchestrator → AdapterOrchestrator → AdapterRegistry → target adapters → derived addresses → WalletRegistry → local storage → UI
~~~

## 2. What is saved

Each blockchain is stored independently under the wallet.

~~~json
{
  "wallet_id": "wallet_1",
  "name": "Wallet 1",
  "blockchains": [
    {
      "name": "Ethereum",
      "symbol": "ETH",
      "decimals": 18,
      "chain_type": "evm",
      "chain_id": "1",
      "address": "0xABC...",
      "rpc_provider": "provider_name",
      "rpc_url": "https://...",
      "balance": null,
      "tokens": []
    },
    {
      "name": "TRON",
      "symbol": "TRX",
      "decimals": 6,
      "chain_type": "tron",
      "chain_id": "tron-mainnet",
      "address": "T...",
      "rpc_provider": "provider_name",
      "rpc_url": "https://...",
      "balance": null,
      "tokens": []
    }
  ]
}
~~~

The actual record also contains wallet/session metadata and derivation information.

## 3. EVM address reuse

One EVM key can produce the same address on Ethereum, BNB Chain, Polygon and other EVM networks. The blockchain records remain separate because chain ID, RPC, balances, tokens and network state are separate.

## 4. Import an existing mnemonic wallet

### Request

~~~json
{
  "action": "import_wallet",
  "params": {
    "mnemonic": "word1 word2 word3 ...",
    "pin": "user-secret",
    "chains": [
      {
        "name": "Ethereum",
        "symbol": "ETH",
        "decimals": 18,
        "chain_type": "evm",
        "chain_id": "1",
        "rpc_provider": "provider_name",
        "rpc_url": "https://..."
      },
      {
        "name": "TRON",
        "symbol": "TRX",
        "decimals": 6,
        "chain_type": "tron",
        "chain_id": "tron-mainnet",
        "rpc_provider": "provider_name",
        "rpc_url": "https://..."
      }
    ]
  }
}
~~~

The engine imports the mnemonic, derives requested addresses, creates the wallet record, assigns the next Wallet N name, stores encrypted secret material, and activates the wallet.

## 5. Import a private-key wallet

~~~json
{
  "action": "import_private_key",
  "params": {
    "private_key": "0x...",
    "pin": "user-secret",
    "chain": {
      "name": "Ethereum",
      "symbol": "ETH",
      "decimals": 18,
      "chain_type": "evm",
      "chain_id": "1",
      "rpc_provider": "provider_name",
      "rpc_url": "https://..."
    }
  }
}
~~~

The engine validates the key, derives the address through the adapter, encrypts and stores the private key, creates the wallet record, assigns the next Wallet N name, and activates it. Standalone private-key wallets cannot create child wallets.

## 6. Create a child wallet

### Request using the active wallet

~~~json
{
  "action": "create_child_wallet",
  "params": {
    "pin": "user-secret"
  }
}
~~~

A specific parent can be supplied:

~~~json
{
  "action": "create_child_wallet",
  "params": {
    "wallet_id": "wallet_1",
    "pin": "user-secret"
  }
}
~~~

If chains are omitted, the parent blockchain configurations are inherited. The child uses the parent's encrypted mnemonic/seed material but derives different child paths.

### Child naming

The engine assigns Child 1, Child 2, Child 3, and so on. The number is calculated within the same secret group. The UI must not calculate child names.

### Flow

~~~text
Parent wallet → decrypt parent secret → same secret group → calculate next child index → derive child paths → AdapterOrchestrator → target adapters → child addresses → WalletRegistry → Child N → local storage
~~~

## 7. Add a blockchain to an existing wallet

~~~json
{
  "action": "add_blockchain",
  "params": {
    "wallet_id": "wallet_1",
    "name": "BNB Smart Chain",
    "symbol": "BNB",
    "decimals": 18,
    "chain_type": "evm",
    "chain_id": "56",
    "rpc_provider": "provider_name",
    "rpc_url": "https://..."
  }
}
~~~

The engine determines the correct address for the wallet and saves a new blockchain record. For EVM, adding another network does not require a new EVM address.

## 8. Derive addresses without creating a wallet record

~~~json
{
  "action": "derive_addresses",
  "params": {
    "chains": [
      {
        "name": "Ethereum",
        "symbol": "ETH",
        "chain_type": "evm"
      },
      {
        "name": "TRON",
        "symbol": "TRX",
        "chain_type": "tron",
        "derivation_path": "m/44'/195'/0'/0/0"
      }
    ]
  }
}
~~~

This performs derivation only. It does not persist a new wallet.

## 9. Restore a persisted wallet

HD wallet:

~~~json
{
  "action": "restore_persisted_wallet",
  "params": {
    "wallet": {},
    "mnemonic": "word1 word2 word3 ...",
    "pin": "user-secret",
    "activate": true
  }
}
~~~

Private-key wallet:

~~~json
{
  "action": "restore_persisted_wallet",
  "params": {
    "wallet": {},
    "private_key": "0x...",
    "pin": "user-secret",
    "activate": true
  }
}
~~~

## 10. Switch the active wallet

~~~json
{
  "action": "switch_wallet",
  "params": {
    "wallet_id": "wallet_2",
    "pin": "user-secret"
  }
}
~~~

After a successful switch, Wallet 2 becomes active for normal wallet-scoped operations.

## 11. List wallets

~~~json
{
  "action": "list_wallets",
  "params": {}
}
~~~

The response groups child wallets beneath their root wallet.

## 12. Get active wallet

~~~json
{
  "action": "active_wallet",
  "params": {}
}
~~~

Returns the currently active wallet.

## 13. Delete wallet

~~~json
{
  "action": "delete_wallet",
  "params": {
    "wallet_id": "wallet_2"
  }
}
~~~

The engine removes the wallet and adjusts active-wallet state when necessary.

## 14. Wallet count

~~~json
{
  "action": "wallet_count",
  "params": {
    "secret_group_id": "seed_..."
  }
}
~~~

Returns the number of wallet records in the secret group.

## 15. Adapter communication during wallet creation

Blockchain-specific derivation is delegated through the adapter boundary.

~~~json
{
  "action": "adapter_target",
  "targets": [
    {
      "chain_type": "evm",
      "blockchain": {
        "name": "Ethereum",
        "symbol": "ETH",
        "decimals": 18,
        "chain_id": "1",
        "rpc_provider": "provider_name",
        "rpc_url": "https://..."
      },
      "body": {
        "action": "derive_address"
      }
    },
    {
      "chain_type": "tron",
      "blockchain": {
        "name": "TRON",
        "symbol": "TRX",
        "decimals": 6,
        "chain_id": "tron-mainnet",
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

The routing is: WalletEngine → Wallet Creation Orchestrator → AdapterOrchestrator → AdapterRegistry → specific adapter.

## 16. Wallet naming rule

Root wallets: Wallet 1, Wallet 2, Wallet 3, ...

Child wallets: Child 1, Child 2, Child 3, ...

The engine determines both names. The UI only displays the returned names.

## 17. Complete lifecycle

~~~text
UI
 ↓
WalletEngine
 ↓
Wallet Creation Orchestrator
 ↓
AdapterOrchestrator
 ↓
AdapterRegistry
 ↓
target adapter(s)
 ↓
derived addresses
 ↓
WalletRegistry
 ↓
Wallet N / Child N
 ↓
local storage
 ↓
active wallet
~~~

## 18. Developer rules

1. Do not generate wallet names in the UI.
2. Do not implement blockchain derivation inside WalletEngine.
3. Use chain_type to select the adapter.
4. Keep each blockchain as its own saved blockchain record.
5. Reuse the same EVM address across EVM networks when it comes from the same key.
6. Keep RPC provider and RPC URL with the saved blockchain configuration.
7. Use the active wallet session for normal wallet-scoped operations.
8. Use wallet_id only when explicitly targeting another wallet.
9. Child wallets belong to the same secret group but have their own child derivation index and blockchain records.
10. Private-key wallets are separate from HD wallet/child-wallet flows.

## 19. Main wallet operations

~~~text
create_wallet
import_wallet
import_private_key
create_child_wallet
add_blockchain
derive_addresses
restore_persisted_wallet
switch_wallet
list_wallets
active_wallet
delete_wallet
wallet_count
~~~

WalletEngine owns wallet identity and naming. The wallet-creation orchestrator owns creation/import/child workflow. AdapterOrchestrator owns adapter routing. AdapterRegistry selects the blockchain implementation. The adapter performs blockchain-specific work. WalletRegistry persists the resulting wallet and blockchain records.