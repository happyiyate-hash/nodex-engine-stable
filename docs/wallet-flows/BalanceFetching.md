# Balance Fetching Flow

## Purpose

This document defines the balance flow after a blockchain has already been saved in the active wallet.

There are two distinct operations:

- `get_balance` — reads the last normalized balance from local wallet storage.
- `refresh_balance` — contacts the saved RPC provider, retrieves fresh data, normalizes it through the correct adapter, persists it, and returns the refreshed result.

The UI does not need to resend the RPC URL, RPC provider, address, or token configuration. Those values come from the saved blockchain record.

## 1. Saved wallet is the starting point

A saved blockchain record is conceptually:

~~~json
{
  "name": "TRON",
  "symbol": "TRX",
  "decimals": 6,
  "chain_type": "tron",
  "chain_id": "tron-mainnet",
  "destination_address": "T...",
  "rpc_provider": "provider_name",
  "rpc_url": "https://...",
  "balance": "0",
  "tokens": []
}
~~~

## 2. Get balance

The UI can request:

~~~json
{
  "action": "get_balance"
}
~~~

Or target one blockchain:

~~~json
{
  "action": "get_balance",
  "chain_id": "tron-mainnet"
}
~~~

This is a local read. It does not contact RPC or an adapter.

Flow:

~~~text
UI
 |
 | get_balance
 v
WalletEngine
 |
 v
Active Wallet
 |
 v
Wallet local storage
 |
 v
Saved blockchain + token balances
 |
 v
UI
~~~

## 3. Refresh balance

For fresh blockchain data:

~~~json
{
  "action": "refresh_balance"
}
~~~

Or one blockchain:

~~~json
{
  "action": "refresh_balance",
  "chain_id": "tron-mainnet"
}
~~~

Flow:

~~~text
UI
 |
 | refresh_balance
 v
WalletEngine
 |
 v
Active Wallet
 |
 v
Saved blockchain configuration
 |
 | chain_type
 | chain_id
 | destination_address
 | rpc_provider
 | rpc_url
 | saved tokens
 v
BalanceFetchService
 |
 v
RPC Orchestrator
 |
 v
Configured RPC Provider
 |
 v
Blockchain RPC
 |
 v
Raw balances
 |
 v
Adapter Orchestrator
 |
 v
Adapter Registry
 |
 v
Target Adapter
 |
 v
Exact normalization
 |
 v
Wallet storage update
 |
 v
UI response / future event
~~~

## 4. Why RPC comes from wallet storage

The UI should not repeatedly send the RPC provider and RPC URL. Those values were saved when the blockchain was added.

## 5. RPC Orchestrator

The balance service sends the request to the RPC orchestration layer. That layer handles the configured provider and supported provider/fallback behavior.

~~~text
BalanceFetchService
       |
       v
RPC Orchestrator
       |
       +-- Provider A
       +-- Provider B
       +-- Provider C
       |
       v
RPC response
~~~

## 6. Raw versus normalized balance

RPC responses can contain raw integer quantities.

~~~text
RPC raw value
     ↓
raw integer
     ↓
correct adapter
     ↓
exact normalization
     ↓
normalized decimal string
     ↓
wallet storage
~~~

For example, a 6-decimal token may return 250000000 and be normalized to 250.000000.

## 7. Adapter normalization

BalanceFetchService does not contain blockchain-specific normalization rules.

~~~text
raw balance
    |
    v
AdapterOrchestrator
    |
    +-- evm   -> EVM adapter
    +-- tron  -> TRON adapter
    +-- solana -> Solana adapter
    +-- ...
~~~

## 8. One chain per fetch and one grouped response

The balance service processes configured chains sequentially. Each adapter invocation receives exactly one blockchain request; results from different chains are never combined into one adapter response.

Within one result, the native asset and all discovered/configured tokens remain grouped under that chain:

~~~json
{
  "wallet_id": "wallet-child-id",
  "chain_id": "polygon",
  "provider": "alchemy",
  "duration_ms": 842,
  "balance": {
    "chain_id": "polygon",
    "native": { "symbol": "POL", "balance": "..." },
    "tokens": [{ "symbol": "USDC", "balance": "..." }]
  }
}
~~~

The engine streams one `balance.result` event for each completed chain. The UI updates that chain's card immediately and waits for the next chain fetch; it does not wait for every blockchain to finish.

## 9. Native balance

A native blockchain balance is stored on its blockchain record:

~~~json
{
  "name": "TRON",
  "symbol": "TRX",
  "decimals": 6,
  "chain_type": "tron",
  "chain_id": "tron-mainnet",
  "destination_address": "T...",
  "balance": "12.500000"
}
~~~

## 10. Token balances

Tokens belong to their blockchain record:

~~~json
{
  "name": "TRON",
  "symbol": "TRX",
  "chain_type": "tron",
  "chain_id": "tron-mainnet",
  "destination_address": "T...",
  "balance": "12.500000",
  "tokens": [
    {
      "name": "Tether USD",
      "symbol": "USDT",
      "decimals": 6,
      "contract_address": "T...",
      "balance": "250.000000"
    }
  ]
}
~~~

## 11. Persistence

After normalization:

~~~text
RPC
 ↓
raw balance
 ↓
adapter normalization
 ↓
normalized balance
 ↓
WalletRegistry.update_balances()
 ↓
local wallet JSON updated
~~~

The next get_balance can return the newly refreshed values without another RPC request.

## 12. Multiple blockchains

Calling refresh_balance can refresh every blockchain saved in the active wallet.

~~~text
Active Wallet
     |
     +-- Ethereum
     +-- BNB Chain
     +-- Polygon
     +-- TRON
     +-- Solana
     |
     v
Balance requests
     |
     v
RPC / adapter orchestration
     |
     v
Normalized results
~~~

These are processed one at a time in the order supplied by the wallet configuration. Each completed chain result is streamed before the next chain starts.

## 13. Same EVM address, different balances

An EVM address can be reused across EVM networks:

~~~text
0xABC...
   |
   +-- Ethereum RPC -> ETH balance
   +-- BNB RPC      -> BNB balance
   +-- Polygon RPC  -> POL balance
   +-- Arbitrum RPC -> ETH balance
~~~

The address can be identical, but each network has independent state. Every saved blockchain record therefore retains its own chain ID, RPC configuration, native balance, token list, and token balances.

## 14. Blockchain added after funds arrive

Funds can exist on a network before that network is configured in the wallet.

After the blockchain is added:

~~~text
Add blockchain
     ↓
save chain configuration
     ↓
associate existing address
     ↓
refresh_balance
     ↓
blockchain RPC
     ↓
discover existing balance
     ↓
normalize
     ↓
save balance
     ↓
UI
~~~

Adding an EVM blockchain does not require generating a new EVM address.

## 15. Errors

An RPC failure must not silently become a fake zero balance.

~~~text
RPC failure
    ↓
refresh error
    ↓
return error/status
    ↓
preserve previously saved balance
~~~

Adapter normalization failures should likewise be reported rather than converted into zero.

## 16. Future real-time events

A future address-monitor/WebSocket layer can trigger refreshes automatically:

~~~text
Transaction detected
       ↓
identify affected blockchain
       ↓
refresh balance
       ↓
RPC
       ↓
adapter normalization
       ↓
update wallet storage
       ↓
balance_updated event
       ↓
UI
~~~

Individual blockchain results can be pushed as soon as they complete.

## 17. Responsibilities

### WalletEngine

- receives get_balance
- receives refresh_balance
- uses the active wallet session
- forwards to the balance service
- does not contain blockchain-specific balance logic

### BalanceFetchService

- builds requests from saved wallet records
- requests fresh balances
- coordinates RPC and adapter work
- collects normalized results
- persists refreshed values

### RPC Orchestrator

- selects the configured RPC provider
- communicates with the RPC endpoint
- handles supported provider/fallback behavior

### AdapterOrchestrator

- receives adapter targets
- routes by chain_type
- uses AdapterRegistry
- forwards normalization work to the correct adapter

### AdapterRegistry

- maps chain_type to the correct adapter

### Blockchain Adapter

- understands blockchain-specific balance representation
- performs exact normalization
- handles token normalization where required

### Wallet Storage

Persists blockchain configuration, destination address, native balance, token configuration, token balances, and RPC configuration.

## 18. Complete architecture

~~~text
                         UI
                          |
              +-----------+-----------+
              |                       |
        get_balance             refresh_balance
              |                       |
              v                       v
        WalletEngine             WalletEngine
              |                       |
              v                       v
        Wallet Storage        BalanceFetchService
        local read                    |
                                      v
                              RPC Orchestrator
                                      |
                                      v
                              Configured Provider
                                      |
                                      v
                                  Blockchain
                                      |
                                      v
                                  Raw Data
                                      |
                                      v
                             AdapterOrchestrator
                                      |
                                      v
                                AdapterRegistry
                                      |
                         +------------+------------+
                         |            |            |
                       EVM         TRON         Other
                         |            |            |
                         +------------+------------+
                                      |
                                      v
                                  Normalize
                                      |
                                      v
                            Wallet Storage Update
                                      |
                                      v
                                      UI
~~~

## Core rule

> get_balance reads what the engine already knows. refresh_balance asks the blockchain what is true now, normalizes the result through the correct adapter, saves it, and returns the refreshed state.

This keeps local reads, RPC communication, adapter routing, normalization, persistence, and future real-time events clearly separated.
