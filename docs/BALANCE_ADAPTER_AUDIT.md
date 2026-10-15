# Balance Adapter Audit (2026-10-14)

## Scope

This audit covers the Rust balance engine and `static/dev_ui/blockchains.json`, not the frontend. It is a code/catalog audit; it does not claim that every upstream provider was live-tested with a funded address.

## Catalog snapshot

- 285 network entries in the catalog.
- 229 entries have a `balance_method`.
- 56 entries are marked `adapter_supported: false` and have no balance method. They must not be presented as supported balance adapters until implemented and tested.
- Among configured entries: 158 use the EVM executor; 29 Cosmos REST balance parsers; 18 UTXO address parsers; the remainder use chain-specific parsers.
- A configured native-balance method is not proof of token-discovery support. Token coverage is chain- and provider-specific.

## Confirmed findings

### 1. EVM token discovery used a deprecated Alchemy option and did not paginate

The EVM provider called `alchemy_getTokenBalances(address, "DEFAULT_TOKENS")`, ignored discovery errors, and read only the first page. Alchemy documents `DEFAULT_TOKENS` as deprecated and supports `"erc20"` plus pageKey pagination. This was corrected in the provider code: use `"erc20"`, request pages (100 records/page, bounded at 20 pages), and publish a `balance.token_discovery.failed` event when the request fails or hits the safety limit.

Documentation: https://www.alchemy.com/docs/data/token-api/token-api-endpoints/alchemy-get-token-balances

### 2. Standard EVM JSON-RPC cannot discover arbitrary ERC-20 contracts

For non-Alchemy RPC providers, the current adapter can query token balances only when token contract addresses are already in `BalanceRequest.assets`. An empty asset list therefore produces a valid native balance with `tokens: []`. This is a provider capability gap, not evidence that JSON parsing failed.

Possible approaches:
- Use an indexed token/portfolio API where supported.
- Maintain a vetted per-chain token list and query known contracts.
- Clearly mark a provider/chain as native-only when neither capability is configured.

Documentation: https://www.alchemy.com/docs/reference/token-api-overview

### 3. Several non-EVM discovery paths silently discard upstream failures

The Aptos and Tezos token-discovery calls, XRP `account_lines`, and individual NEAR `ft_balance_of` calls previously ignored request errors. The engine now publishes `balance.token_discovery.failed` events for these failures while retaining a valid native balance. Empty token lists can still mean no holdings or an unsupported discovery path, so chain-specific fixture tests remain necessary. Do not turn a token-indexer failure into a fake zero balance.

References:
- Aptos account resources: https://aptos.guide/rest-api/operations/get_account_resources
- Tezos TzKT API: https://api.tzkt.io/
- XRP `account_lines`: https://xrpl.org/docs/references/http-websocket-apis/public-api-methods/account-methods/account_lines
- NEAR fungible-token standard: https://github.com/near/NEPs/blob/master/neps/nep-0141.md

### 4. Chain parsers intentionally return native-only balances in multiple families

The `utxo_address`, `sum_fields`, and `u256_limbs` modes currently return empty token lists. For UTXO networks this is usually correct because assets are not ERC-20-style tokens; for other parsers it must be treated as native-only unless an asset endpoint is added. The frontend should not be used to paper over this backend limitation.

### 5. Sui token parser has a useful indexed path, but pagination needs explicit validation

The Sui GraphQL query reads `address.balances.nodes` and extracts each coin type. The Sui API documents `balances` as a paginated connection; the current query does not specify pagination arguments or follow cursors. Accounts with many coin types may be truncated depending on endpoint defaults.

Documentation: https://www.docs.sui.io/onchain-finance/asset-custody/address-balances/using-address-balances

## Adapter acceptance criteria

A chain/provider should only be called fully supported after tests verify all of the following with realistic fixtures and, where possible, a live provider:
1. Correct native balance and decimals/base-unit handling.
2. Token enumeration strategy documented: indexed discovery, all-assets endpoint, configured contract list, or explicitly native-only.
3. Correct pagination and non-zero filtering.
4. Token metadata is optional and cannot erase a confirmed token balance.
5. HTTP status errors, malformed JSON, provider error payloads, and timeouts remain observable.
6. Per-chain tests cover empty accounts, token-bearing accounts, missing fields, and upstream failure.
7. Provider and endpoint combinations in the catalog are verified against the provider's actual API.

## Next work recommended

1. Add Sui GraphQL pagination.
2. Decide which of the 56 catalog entries marked unsupported should be implemented, and keep the rest explicitly unsupported.
3. Build a fixture-driven test matrix for each `response_mode` before declaring the catalog production-ready.
4. Run `cargo fmt --check`, `cargo test`, and a live smoke test for each provider family; a successful Render build alone is not sufficient evidence.


## Newly fixed: cross-chain UI mixing and excessive concurrency

### 6. UI matching by address caused EVM balances to cross network boundaries

The home screen and Zustand store previously used an address match as a fallback when identifying a chain result. Ethereum, Polygon, Arbitrum, Base, and other EVM networks commonly share the same address for the same key. The address is not a chain identity. Refresh merging and live-event upserts now use the resolved canonical chain ID only, and the UI display no longer falls back to address matching. Events whose chain envelope and grouped balance disagree are rejected.

### 7. Balance fetching was fanned out to 12 chains concurrently

The balance service previously spawned one task per chain with a concurrency cap of 12. It now runs a single worker that fetches one configured blockchain at a time and emits that chain's complete native-plus-token result before starting the next chain. This reduces provider bursts and makes progress ordering predictable. Token/native RPC calls inside one chain adapter may still be grouped when the provider supports it; results are never combined across different chains.

### 8. Streaming events now carry per-chain diagnostics

- `balance.fetch.started`: wallet ID, chain ID, provider, and configured asset count.
- `balance.result`: wallet ID, chain ID, provider, elapsed milliseconds, and one nested `balance` object containing that chain's native balance and tokens.
- `balance.fetch.failed`: wallet ID, chain ID, provider, elapsed milliseconds, and error message.
- `balance.refresh.completed`: final overall success/failure counts.

The frontend event reducer accepts the grouped `data.balance` payload, canonicalizes its chain ID to the selected wallet chain, and upserts only that chain. The Logs screen can show/copy/download each event as JSON.

## Validation status

GitHub Actions is creating the Rust CI run but the run exits in roughly two seconds with no runner assigned and no steps reported (`runner_id: 0`, empty `steps`). Therefore this environment has not yet provided a Rust compile/test result for these commits. Do not treat the commits as compile-verified until CI runs on a functioning runner.
