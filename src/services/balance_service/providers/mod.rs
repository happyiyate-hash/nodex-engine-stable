use async_trait::async_trait;
use crate::services::balance_service::service::{BalanceAsset, BalanceRequest, ChainBalance};
use crate::services::rpc::RpcOrchestrator;
use tokio::sync::mpsc::UnboundedSender;


use serde_json::{json, Value};
use ethers::types::U256;
use crate::services::rpc::providers::RpcRequest;


fn report_token_discovery_failure(request: &BalanceProviderRequest, message: impl Into<String>) {
    crate::services::event_service::EventService::publish(
        "balance.token_discovery.failed",
        json!({
            "wallet_id": request.request.wallet_id,
            "chain_id": request.request.chain.chain_id,
            "provider": request.request.provider,
            "message": message.into()
        }),
    );
}

fn pad_address(address: &str) -> Result<String, BalanceProviderError> {
    let raw = address.trim().strip_prefix("0x").unwrap_or(address.trim());
    if raw.len() != 40 || !raw.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(BalanceProviderError::Message(format!("invalid EVM address: {}", address)));
    }
    Ok(format!("{:0>64}", raw.to_ascii_lowercase()))
}

fn decode_uint(value: &Value) -> Result<U256, BalanceProviderError> {
    let hex = value.as_str().ok_or_else(|| BalanceProviderError::Message("eth_call result was not a hex string".into()))?;
    U256::from_str_radix(hex.trim_start_matches("0x"), 16)
        .map_err(|_| BalanceProviderError::Message("invalid EVM uint result".into()))
}

fn decode_string(value: &Value) -> Option<String> {
    let hex = value.as_str()?.trim_start_matches("0x");
    if hex.len() < 64 { return None; }
    let bytes = hex::decode(hex).ok()?;
    if bytes.len() >= 64 {
        let offset = u32::from_be_bytes(bytes[28..32].try_into().ok()?) as usize;
        if offset + 32 <= bytes.len() {
            let len = u32::from_be_bytes(bytes[offset + 28..offset + 32].try_into().ok()?) as usize;
            let start = offset + 32;
            if start + len <= bytes.len() {
                return String::from_utf8(bytes[start..start + len].to_vec()).ok().filter(|s| !s.is_empty());
            }
        }
    }
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8(bytes[..end].to_vec()).ok().filter(|s| !s.is_empty())
}

fn encode_balance_of(address: &str) -> Result<String, BalanceProviderError> {
    Ok(format!("0x70a08231{}", pad_address(address)?))
}

fn build_token_calls(
    request: &BalanceRequest,
) -> Result<Vec<RpcRequest>, BalanceProviderError> {
    let mut calls = Vec::new();
    for contract in &request.assets {
        let contract = contract.trim().to_string();
        if contract.is_empty() { continue; }
        // balanceOf(address), decimals(), symbol(), name().
        calls.push(RpcRequest {
            provider: request.provider.clone(),
            chain_type: request.chain.normalized_type(),
            chain_id: request.chain.chain_id.clone(),
            endpoint: request.rpc_endpoint.clone(),
            method: "eth_call".into(),
            params: json!([{"to": contract, "data": encode_balance_of(&request.address)?}, "latest"]),
        });
        for (selector, label) in [("0x313ce567", "decimals"), ("0x95d89b41", "symbol"), ("0x06fdde03", "name")] {
            calls.push(RpcRequest {
                provider: request.provider.clone(),
                chain_type: request.chain.normalized_type(),
                chain_id: request.chain.chain_id.clone(),
                endpoint: request.rpc_endpoint.clone(),
                method: "eth_call".into(),
                params: json!([{"to": contract, "data": selector}, "latest"]),
            });
            let _ = label;
        }
    }
    Ok(calls)
}

pub mod alchemy;
pub mod blockfrost;
pub mod infura;
pub mod public;

pub use alchemy::AlchemyBalanceProvider;
pub use blockfrost::BlockfrostBalanceProvider;
pub use infura::InfuraBalanceProvider;
pub use public::PublicBalanceProvider;

pub(super) async fn fetch_evm_balances_batch(
    requests: Vec<BalanceProviderRequest>,
) -> Result<Vec<BalanceProviderResponse>, BalanceProviderError> {
    if requests.is_empty() { return Ok(Vec::new()); }

    let mut rpc_requests = Vec::new();
    let mut ranges = Vec::with_capacity(requests.len());

    for request in &requests {
        let start = rpc_requests.len();
        rpc_requests.push(RpcRequest {
            provider: request.request.provider.clone(),
            chain_type: request.request.chain.normalized_type(),
            chain_id: request.request.chain.chain_id.clone(),
            endpoint: request.request.rpc_endpoint.clone(),
            method: "eth_getBalance".into(),
            params: json!([request.request.address, "latest"]),
        });
        rpc_requests.extend(build_token_calls(&request.request)?);
        ranges.push((start, rpc_requests.len()));
    }

    // RpcOrchestrator re-groups this single logical submission by
    // provider + endpoint, so different chain endpoints are never mixed.
    let responses = requests[0].rpc.execute(rpc_requests).await;
    let mut output = Vec::with_capacity(requests.len());

    for (request, (start, end)) in requests.into_iter().zip(ranges) {
        let slice = &responses[start..end];
        let native_result = match slice.get(0) {
            Some(Ok(value)) => value,
            Some(Err(error)) => {
                output.push(BalanceProviderResponse::error(
                    &request.request,
                    BalanceProviderError::Message(error.clone()),
                ));
                continue;
            }
            None => {
                output.push(BalanceProviderResponse::error(
                    &request.request,
                    BalanceProviderError::Message("RPC returned no native balance".into()),
                ));
                continue;
            }
        };

        let native_hex = match native_result.result.as_str() {
            Some(value) => value,
            None => {
                output.push(BalanceProviderResponse::error(
                    &request.request,
                    BalanceProviderError::Message("native balance was not a hex string".into()),
                ));
                continue;
            }
        };
        let native_value = match U256::from_str_radix(native_hex.trim_start_matches("0x"), 16) {
            Ok(value) => value,
            Err(_) => {
                output.push(BalanceProviderResponse::error(
                    &request.request,
                    BalanceProviderError::Message("invalid native EVM balance".into()),
                ));
                continue;
            }
        };

        let mut tokens = Vec::new();
        let mut offset = 1usize;
        let mut token_error = None;
        for contract in &request.request.assets {
            if contract.trim().is_empty() { continue; }
            let balance_result = match slice.get(offset) {
                Some(Ok(value)) => decode_uint(&value.result),
                Some(Err(error)) => Err(BalanceProviderError::Message(error.clone())),
                None => Err(BalanceProviderError::Message("RPC omitted token balance response".into())),
            };
            let balance = match balance_result {
                Ok(value) => value,
                Err(error) => {
                    token_error = Some(format!("token balance RPC/decode failure for {}: {}", contract, error));
                    break;
                }
            };
            // Metadata is optional; failure to read it must not change a confirmed balance.
            let decimals = slice.get(offset + 1)
                .and_then(|r| r.as_ref().ok())
                .and_then(|r| decode_uint(&r.result).ok())
                .and_then(|v| if v <= U256::from(u32::MAX) { Some(v.as_u32()) } else { None });
            let symbol = slice.get(offset + 2).and_then(|r| r.as_ref().ok()).and_then(|r| decode_string(&r.result));
            let name = slice.get(offset + 3).and_then(|r| r.as_ref().ok()).and_then(|r| decode_string(&r.result));
            tokens.push(BalanceAsset {
                address: contract.clone(),
                symbol,
                name,
                decimals,
                balance: balance.to_string(),
                is_native: false,
            });
            offset += 4;
        }
        if let Some(error) = token_error {
            output.push(BalanceProviderResponse::error(
                &request.request,
                BalanceProviderError::Message(error),
            ));
            continue;
        }

        // Alchemy exposes an indexed token-discovery RPC in addition to the
        // standard Ethereum JSON-RPC methods. Use it only when the configured
        // provider/endpoint identifies Alchemy; ordinary RPC nodes cannot
        // enumerate arbitrary ERC-20 contracts from an address alone.
        let endpoint_is_alchemy = request.request.provider.eq_ignore_ascii_case("alchemy")
            || request.request.rpc_endpoint.to_ascii_lowercase().contains("alchemy");
        if endpoint_is_alchemy {
            // Alchemy documents "erc20" as the supported token specification.
            // "DEFAULT_TOKENS" is deprecated and can produce incomplete results.
            // Follow pageKey so wallets with many token holdings are not silently truncated.
            let mut page_key: Option<String> = None;
            let mut discovered_items: Vec<Value> = Vec::new();
            let mut discovery_error: Option<String> = None;
            const MAX_TOKEN_DISCOVERY_PAGES: usize = 20;

            for _ in 0..MAX_TOKEN_DISCOVERY_PAGES {
                let token_spec = if let Some(key) = page_key.as_ref() {
                    json!([request.request.address, "erc20", {"pageKey": key, "maxCount": 100}])
                } else {
                    json!([request.request.address, "erc20", {"maxCount": 100}])
                };
                match request.rpc.execute_one(RpcRequest {
                    provider: request.request.provider.clone(),
                    chain_type: request.request.chain.normalized_type(),
                    chain_id: request.request.chain.chain_id.clone(),
                    endpoint: request.request.rpc_endpoint.clone(),
                    method: "alchemy_getTokenBalances".into(),
                    params: token_spec,
                }).await {
                    Ok(discovery) => {
                        if let Some(items) = discovery.result.get("tokenBalances").and_then(Value::as_array) {
                            discovered_items.extend(items.iter().cloned());
                        } else {
                            discovery_error = Some(format!(
                                "Alchemy token discovery response omitted tokenBalances: {}",
                                discovery.result
                            ));
                            break;
                        }
                        page_key = discovery.result.get("pageKey")
                            .and_then(Value::as_str)
                            .filter(|value| !value.is_empty())
                            .map(str::to_owned);
                        if page_key.is_none() {
                            break;
                        }
                    }
                    Err(error) => {
                        discovery_error = Some(error);
                        break;
                    }
                }
            }

            if page_key.is_some() && discovery_error.is_none() {
                discovery_error = Some(format!(
                    "Alchemy token discovery reached the {}-page safety limit; results may be incomplete",
                    MAX_TOKEN_DISCOVERY_PAGES
                ));
            }
            if let Some(error) = discovery_error {
                crate::services::event_service::EventService::publish(
                    "balance.token_discovery.failed",
                    json!({
                        "wallet_id": request.request.wallet_id,
                        "chain_id": request.request.chain.chain_id,
                        "provider": request.request.provider,
                        "message": error
                    }),
                );
            }

            for item in &discovered_items {
                let contract = match item.get("contractAddress").and_then(Value::as_str) {
                    Some(value) if !value.trim().is_empty() => value.to_owned(),
                    _ => continue,
                };
                if tokens.iter().any(|asset| asset.address.eq_ignore_ascii_case(&contract))
                    || request.request.assets.iter().any(|asset| asset.eq_ignore_ascii_case(&contract)) {
                    continue;
                }
                let raw_balance = item.get("tokenBalance").and_then(Value::as_str).unwrap_or("0x0");
                let balance = match U256::from_str_radix(raw_balance.trim_start_matches("0x"), 16) {
                    Ok(value) if !value.is_zero() => value,
                    _ => continue,
                };

                // Metadata is best-effort. A metadata failure must not discard a confirmed balance.
                let metadata = request.rpc.execute(vec![
                    RpcRequest {
                        provider: request.request.provider.clone(),
                        chain_type: request.request.chain.normalized_type(),
                        chain_id: request.request.chain.chain_id.clone(),
                        endpoint: request.request.rpc_endpoint.clone(),
                        method: "eth_call".into(),
                        params: json!([{"to": contract, "data": "0x313ce567"}, "latest"]),
                    },
                    RpcRequest {
                        provider: request.request.provider.clone(),
                        chain_type: request.request.chain.normalized_type(),
                        chain_id: request.request.chain.chain_id.clone(),
                        endpoint: request.request.rpc_endpoint.clone(),
                        method: "eth_call".into(),
                        params: json!([{"to": contract, "data": "0x95d89b41"}, "latest"]),
                    },
                    RpcRequest {
                        provider: request.request.provider.clone(),
                        chain_type: request.request.chain.normalized_type(),
                        chain_id: request.request.chain.chain_id.clone(),
                        endpoint: request.request.rpc_endpoint.clone(),
                        method: "eth_call".into(),
                        params: json!([{"to": contract, "data": "0x06fdde03"}, "latest"]),
                    },
                ]).await;

                let decimals = metadata.get(0).and_then(|r| r.as_ref().ok())
                    .and_then(|r| decode_uint(&r.result).ok())
                    .and_then(|v| if v <= U256::from(u32::MAX) { Some(v.as_u32()) } else { None });
                let symbol = metadata.get(1).and_then(|r| r.as_ref().ok())
                    .and_then(|r| decode_string(&r.result));
                let name = metadata.get(2).and_then(|r| r.as_ref().ok())
                    .and_then(|r| decode_string(&r.result));
                tokens.push(BalanceAsset {
                    address: contract,
                    symbol,
                    name,
                    decimals,
                    balance: balance.to_string(),
                    is_native: false,
                });
            }
        }

        let native = BalanceAsset {
            address: "native".into(),
            symbol: request.request.chain.symbol.clone(),
            name: Some(request.request.chain.name.clone()),
            decimals: Some(18),
            balance: native_value.to_string(),
            is_native: true,
        };

        // Providers return raw chain-specific balances. The Balance Service owns
        // adapter-registry lookup and normalization.
        let normalized = ChainBalance {
            wallet_id: request.request.wallet_id,
            chain_id: request.request.chain.chain_id.clone(),
            chain_type: request.request.chain.chain_type.clone(),
            address: request.request.address,
            native: Some(native),
            tokens,
        };

        output.push(BalanceProviderResponse::success(normalized));
    }

    Ok(output)
}

#[derive(Clone)]
pub struct BalanceProviderRequest {
    pub request: BalanceRequest,
    pub rpc: std::sync::Arc<RpcOrchestrator>,
}

#[derive(Debug, Clone)]
pub struct BalanceProviderResponse {
    pub result: Result<ChainBalance, BalanceProviderError>,
    pub wallet_id: String,
    pub chain_id: String,
}

impl BalanceProviderResponse {
    pub fn success(balance: ChainBalance) -> Self {
        Self {
            wallet_id: balance.wallet_id.clone(),
            chain_id: balance.chain_id.clone(),
            result: Ok(balance),
        }
    }

    pub fn error(request: &BalanceRequest, error: BalanceProviderError) -> Self {
        Self {
            wallet_id: request.wallet_id.clone(),
            chain_id: request.chain.chain_id.clone(),
            result: Err(error),
        }
    }

    pub fn placeholder(request: BalanceProviderRequest) -> Self {
        Self::error(
            &request.request,
            BalanceProviderError::Message(format!(
                "provider implementation pending for {}",
                request.request.provider
            )),
        )
    }
}

#[derive(Debug, Clone)]
pub enum BalanceProviderError { Message(String) }

impl std::fmt::Display for BalanceProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::Message(v) => f.write_str(v) }
    }
}
impl std::error::Error for BalanceProviderError {}

/// A provider may override this to emit results incrementally while its own
/// batching layer completes. The default keeps existing providers compatible.
#[async_trait]
pub trait BalanceProvider: Send + Sync {
    async fn fetch_batch(&self, requests: Vec<BalanceProviderRequest>) -> Result<Vec<BalanceProviderResponse>, BalanceProviderError>;

    async fn fetch_batch_stream(
        &self,
        requests: Vec<BalanceProviderRequest>,
        sender: UnboundedSender<BalanceProviderResponse>,
    ) -> Result<(), BalanceProviderError> {
        match self.fetch_batch(requests.clone()).await {
            Ok(responses) => {
                for response in responses {
                    let _ = sender.send(response);
                }
            }
            Err(error) => {
                // A provider-level failure is converted into one error result
                // per requested chain, preserving partial-batch semantics.
                for request in requests {
                    let _ = sender.send(BalanceProviderResponse::error(
                        &request.request,
                        BalanceProviderError::Message(error.to_string()),
                    ));
                }
            }
        }
        Ok(())
    }
}



/// Load balance execution metadata from the same blockchain catalog used by the UI.
/// Chain names and IDs are matched against the catalog so refreshes still work for
/// wallet records created before balance_method was introduced.
pub(super) fn balance_method_config(chain: &crate::types::chain::ChainRequest) -> Option<Value> {
    static BLOCKCHAIN_CATALOG: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    // Prefer the deployed catalog file so balance behavior follows JSON config.
    // The embedded copy is a safe fallback when the engine runs from another
    // working directory or the static file is not mounted separately.
    let catalog = BLOCKCHAIN_CATALOG.get_or_init(|| {
        let catalog_text = std::fs::read_to_string("static/dev_ui/blockchains.json")
            .unwrap_or_else(|_| include_str!("../../../../static/dev_ui/blockchains.json").to_owned());
        serde_json::from_str(&catalog_text).unwrap_or(Value::Null)
    });
    let networks = catalog.get("networks")?.as_array()?;
    let wanted_name = chain.name.trim();
    let wanted_id = chain.chain_id.trim();
    let wanted_type = chain.normalized_type();
    let same_family = |network: &&Value| {
        network.get("chain_type").and_then(Value::as_str)
            .map(|kind| crate::types::chain::ChainFamily::from(kind).as_str() == wanted_type.as_str())
            .unwrap_or(false)
    };
    let by_name = networks.iter().find(|network| {
        same_family(network)
            && network.get("name").and_then(Value::as_str)
                .map(|name| name.trim().eq_ignore_ascii_case(wanted_name)).unwrap_or(false)
    });
    let found = by_name.or_else(|| networks.iter().find(|network| {
        same_family(network) && network.get("chain_id").and_then(Value::as_str) == Some(wanted_id)
    }));
    found.and_then(|network| network.get("balance_method")).cloned()
}

fn replace_address_template(value: &Value, address: &str) -> Value {
    match value {
        Value::String(text) => Value::String(text.replace("{address}", address)),
        Value::Array(items) => Value::Array(items.iter().map(|item| replace_address_template(item, address)).collect()),
        Value::Object(items) => Value::Object(items.iter().map(|(key, item)| (key.clone(), replace_address_template(item, address))).collect()),
        _ => value.clone(),
    }
}

fn replace_address_in_path(path: &str, address: &str) -> String {
    path.replace("{address}", address)
}

/// Executes a catalog-defined non-EVM balance method. Provider authentication and
/// transport remain in RpcOrchestrator; this code only interprets catalog metadata.
pub(super) async fn fetch_configured_native_balance(
    request: &BalanceProviderRequest,
    config: &Value,
) -> Result<ChainBalance, BalanceProviderError> {
    let r = &request.request;
    let executor = config.get("executor").and_then(Value::as_str).unwrap_or("");
    let response = match executor {
        "json_rpc" => {
            let method = config.get("method").and_then(Value::as_str).ok_or_else(|| BalanceProviderError::Message(format!(
                "balance_method for {} is missing method", r.chain.name
            )))?;
            let params = replace_address_template(
                config.get("params").unwrap_or(&Value::Null),
                &r.address,
            );
            request.rpc.execute_one(RpcRequest {
                provider: r.provider.clone(),
                chain_type: r.chain.normalized_type(),
                chain_id: r.chain.chain_id.clone(),
                endpoint: r.rpc_endpoint.clone(),
                method: method.to_owned(),
                params,
            }).await.map_err(BalanceProviderError::Message)?.result
        }
        "http_post" | "http_get" => {
            let path_template = config.get("path").and_then(Value::as_str).ok_or_else(|| BalanceProviderError::Message(format!(
                "balance_method for {} is missing HTTP path", r.chain.name
            )))?;
            // HTTP path templates (e.g. /v2/accounts/{address}) must be
            // interpolated just like JSON-RPC params and HTTP request bodies.
            let path = replace_address_in_path(path_template, &r.address);
            let endpoint = format!("{}{}", r.rpc_endpoint.trim_end_matches('/'), if path.starts_with('/') { path } else { format!("/{path}") });
            let headers = if let Some(entries) = config.get("headers").and_then(Value::as_object) {
                let mut resolved = serde_json::Map::new();
                for (name, spec) in entries {
                    let env_name = spec.get("env").and_then(Value::as_str).ok_or_else(|| BalanceProviderError::Message(format!(
                        "{} balance_method header '{}' must name an environment variable", r.chain.name, name
                    )))?;
                    let value = std::env::var(env_name).map_err(|_| BalanceProviderError::Message(format!(
                        "{} balance request requires environment variable {}", r.chain.name, env_name
                    )))?;
                    resolved.insert(name.clone(), Value::String(value));
                }
                Value::Object(resolved)
            } else {
                Value::Null
            };
            let (method, params) = if executor == "http_post" {
                let body = replace_address_template(config.get("body").unwrap_or(&Value::Null), &r.address);
                if headers.is_null() {
                    ("__http_post", body)
                } else {
                    ("__http_post", serde_json::json!({"__http_body": body, "__headers": headers}))
                }
            } else if headers.is_null() {
                ("__http_get", Value::Null)
            } else {
                ("__http_get", serde_json::json!({"__headers": headers}))
            };
            request.rpc.execute_one(RpcRequest {
                provider: r.provider.clone(),
                chain_type: "http".into(),
                chain_id: r.chain.chain_id.clone(),
                endpoint,
                method: method.into(),
                params,
            }).await.map_err(BalanceProviderError::Message)?.result
        }
        other => return Err(BalanceProviderError::Message(format!(
            "unsupported configured balance executor '{}' for chain {}", other, r.chain.name
        ))),
    };

    if let Some(error) = response.get("error") {
        return Err(BalanceProviderError::Message(format!(
            "{} balance method returned an RPC error: {}", r.chain.name, error
        )));
    }
    if let Some(error) = response.get("Error").and_then(Value::as_str) {
        return Err(BalanceProviderError::Message(format!(
            "{} balance method returned an HTTP API error: {}", r.chain.name, error
        )));
    }

    if config.get("response_mode").and_then(Value::as_str) == Some("sui_graphql_balance") {
        if let Some(errors) = response.get("errors").and_then(Value::as_array) {
            if !errors.is_empty() {
                return Err(BalanceProviderError::Message(format!(
                    "{} Sui GraphQL returned errors: {}", r.chain.name, serde_json::to_string(&errors).unwrap_or_else(|_| format!("{:?}", errors))
                )));
            }
        }
        let raw_balance = response.pointer("/data/address/balance/totalBalance")
            .and_then(|value| match value {
                Value::String(text) => Some(text.clone()),
                Value::Number(number) => Some(number.to_string()),
                _ => None,
            })
            .ok_or_else(|| BalanceProviderError::Message(format!(
                "{} Sui GraphQL response omitted data.address.balance.totalBalance: {}", r.chain.name, response
            )))?;
        if raw_balance.is_empty() || !raw_balance.bytes().all(|b| b.is_ascii_digit()) {
            return Err(BalanceProviderError::Message(format!(
                "{} Sui GraphQL returned an invalid totalBalance", r.chain.name
            )));
        }
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(),
            chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(),
            address: r.address.clone(),
            native: Some(BalanceAsset {
                address: "native".into(),
                symbol: config.get("symbol").and_then(Value::as_str).map(str::to_owned).or_else(|| r.chain.symbol.clone()),
                name: config.get("asset_name").and_then(Value::as_str).map(str::to_owned).or_else(|| Some(r.chain.name.clone())),
                decimals: config.get("decimals").and_then(Value::as_u64).map(|value| value as u32),
                balance: raw_balance.to_owned(),
                is_native: true,
            }),
            tokens: Vec::new(),
        });
    }

    if config.get("response_mode").and_then(Value::as_str) == Some("cardano_amounts") {
        let amounts = response.get("amount").and_then(Value::as_array).ok_or_else(|| BalanceProviderError::Message(format!(
            "{} Cardano response omitted amount array: {}", r.chain.name, response
        )))?;
        let mut native = None;
        let mut tokens = Vec::new();
        for asset in amounts {
            let unit = asset.get("unit").and_then(Value::as_str).unwrap_or_default().to_owned();
            let quantity = asset.get("quantity").and_then(Value::as_str).map(str::to_owned)
                .or_else(|| asset.get("quantity").and_then(Value::as_u64).map(|value| value.to_string()));
            if unit.is_empty() || quantity.is_none() { continue; }
            let quantity = quantity.unwrap_or_default();
            if unit == "lovelace" {
                native = Some(BalanceAsset {
                    address: "native".into(), symbol: Some("ADA".into()), name: Some("Cardano".into()),
                    decimals: Some(6), balance: quantity, is_native: true,
                });
            } else {
                tokens.push(BalanceAsset {
                    address: unit.clone(), symbol: Some(unit), name: None,
                    decimals: None, balance: quantity, is_native: false,
                });
            }
        }
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(), chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(), address: r.address.clone(),
            native, tokens,
        });
    }

    if config.get("response_mode").and_then(Value::as_str) == Some("utxo_address") {
        let stat = |group: &str, field: &str| -> Result<u64, BalanceProviderError> {
            response.get(group).and_then(|value| value.get(field)).and_then(Value::as_u64)
                .ok_or_else(|| BalanceProviderError::Message(format!(
                    "{} UTXO response omitted valid {}.{} value: {}", r.chain.name, group, field, response
                )))
        };
        let confirmed = stat("chain_stats", "funded_txo_sum")?.saturating_sub(stat("chain_stats", "spent_txo_sum")?);
        let pending = stat("mempool_stats", "funded_txo_sum")?.saturating_sub(stat("mempool_stats", "spent_txo_sum")?);
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(),
            chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(),
            address: r.address.clone(),
            native: Some(BalanceAsset {
                address: "native".into(),
                symbol: config.get("symbol").and_then(Value::as_str).map(str::to_owned).or_else(|| r.chain.symbol.clone()),
                name: config.get("asset_name").and_then(Value::as_str).map(str::to_owned).or_else(|| Some(r.chain.name.clone())),
                decimals: config.get("decimals").and_then(Value::as_u64).map(|value| value as u32),
                balance: confirmed.saturating_add(pending).to_string(),
                is_native: true,
            }),
            tokens: Vec::new(),
        });
    }

    if config.get("response_mode").and_then(Value::as_str) == Some("bep2_balances") {
        let balances = response.get("balances").and_then(Value::as_array).ok_or_else(|| BalanceProviderError::Message(format!(
            "{} BEP2 response omitted balances array: {}", r.chain.name, response
        )))?;
        let mut native = None;
        let mut tokens = Vec::new();
        for coin in balances {
            let symbol = coin.get("symbol").and_then(Value::as_str).unwrap_or_default().to_owned();
            if symbol.is_empty() { continue; }
            let amount = |field: &str| -> Result<u128, BalanceProviderError> {
                coin.get(field).and_then(Value::as_str).and_then(|text| text.parse::<u128>().ok())
                    .or_else(|| coin.get(field).and_then(Value::as_u64).map(u128::from))
                    .ok_or_else(|| BalanceProviderError::Message(format!(
                        "{} BEP2 response omitted valid {} amount for {}: {}", r.chain.name, field, symbol, coin
                    )))
            };
            let total = amount("free")?.saturating_add(amount("locked")?).saturating_add(amount("frozen")?).to_string();
            if symbol.eq_ignore_ascii_case("BNB") {
                native = Some(BalanceAsset {
                    address: "native".into(), symbol: Some("BNB".into()),
                    name: Some("BNB Beacon Chain".into()), decimals: Some(8),
                    balance: total, is_native: true,
                });
            } else {
                tokens.push(BalanceAsset {
                    address: symbol.clone(), symbol: Some(symbol), name: None,
                    decimals: None, balance: total, is_native: false,
                });
            }
        }
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(), chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(), address: r.address.clone(),
            native, tokens,
        });
    }

    if config.get("response_mode").and_then(Value::as_str) == Some("sum_fields") {
        let fields = config.get("balance_fields").and_then(Value::as_array).ok_or_else(|| BalanceProviderError::Message(format!(
            "{} balance_method sum_fields requires balance_fields", r.chain.name
        )))?;
        let mut total = 0u128;
        for field in fields {
            let field = field.as_str().ok_or_else(|| BalanceProviderError::Message(format!(
                "{} balance_method contains a non-string balance field", r.chain.name
            )))?;
            let value = response.get(field).ok_or_else(|| BalanceProviderError::Message(format!(
                "{} balance response omitted configured field '{}': {}", r.chain.name, field, response
            )))?;
            let amount = value.as_u64().map(u128::from)
                .or_else(|| value.as_str().and_then(|text| text.parse::<u128>().ok()))
                .ok_or_else(|| BalanceProviderError::Message(format!(
                    "{} balance field '{}' was not a valid non-negative integer: {}", r.chain.name, field, value
                )))?;
            total = total.checked_add(amount).ok_or_else(|| BalanceProviderError::Message(format!(
                "{} balance field sum overflowed u128", r.chain.name
            )))?;
        }
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(),
            chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(),
            address: r.address.clone(),
            native: Some(BalanceAsset {
                address: "native".into(),
                symbol: config.get("symbol").and_then(Value::as_str).map(str::to_owned).or_else(|| r.chain.symbol.clone()),
                name: config.get("asset_name").and_then(Value::as_str).map(str::to_owned).or_else(|| Some(r.chain.name.clone())),
                decimals: config.get("decimals").and_then(Value::as_u64).map(|value| value as u32),
                balance: total.to_string(),
                is_native: true,
            }),
            tokens: Vec::new(),
        });
    }

    if config.get("response_mode").and_then(Value::as_str) == Some("u256_limbs") {
        let parts = response.as_array().ok_or_else(|| BalanceProviderError::Message(format!(
            "{} balance response must be an array of low/high U256 limbs: {}", r.chain.name, response
        )))?;
        let parse_limb = |index: usize| -> Result<U256, BalanceProviderError> {
            let text = parts.get(index).and_then(Value::as_str).unwrap_or(if index == 1 { "0x0" } else { "" });
            U256::from_str_radix(text.trim_start_matches("0x"), 16).map_err(|error| BalanceProviderError::Message(format!(
                "{} balance response contained invalid U256 limb {}: {}", r.chain.name, index, error
            )))
        };
        let low = parse_limb(0)?;
        let high = parse_limb(1)?;
        let total = high * (U256::one() << 128) + low;
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(),
            chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(),
            address: r.address.clone(),
            native: Some(BalanceAsset {
                address: "native".into(),
                symbol: config.get("symbol").and_then(Value::as_str).map(str::to_owned).or_else(|| r.chain.symbol.clone()),
                name: config.get("asset_name").and_then(Value::as_str).map(str::to_owned).or_else(|| Some(r.chain.name.clone())),
                decimals: config.get("decimals").and_then(Value::as_u64).map(|value| value as u32),
                balance: total.to_string(),
                is_native: true,
            }),
            tokens: Vec::new(),
        });
    }

    if config.get("response_mode").and_then(Value::as_str) == Some("cosmos_balances") {
        let balances = response.get("balances").and_then(Value::as_array).ok_or_else(|| BalanceProviderError::Message(format!(
            "{} balance response omitted balances array: {}", r.chain.name, response
        )))?;
        let native_denom = config.get("native_denom").and_then(Value::as_str).unwrap_or_default();
        let mut native = if native_denom.is_empty() { None } else {
            Some(BalanceAsset {
                address: "native".into(),
                symbol: config.get("symbol").and_then(Value::as_str).map(str::to_owned).or_else(|| r.chain.symbol.clone()),
                name: config.get("asset_name").and_then(Value::as_str).map(str::to_owned).or_else(|| Some(r.chain.name.clone())),
                decimals: config.get("decimals").and_then(Value::as_u64).map(|value| value as u32),
                balance: "0".into(),
                is_native: true,
            })
        };
        let mut tokens = Vec::new();
        for coin in balances {
            let denom = coin.get("denom").and_then(Value::as_str).unwrap_or_default().to_owned();
            let amount = coin.get("amount").and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| coin.get("amount").and_then(Value::as_u64).map(|value| value.to_string()));
            if denom.is_empty() || amount.is_none() { continue; }
            let amount = amount.unwrap_or_default();
            if !native_denom.is_empty() && denom == native_denom {
                native = Some(BalanceAsset {
                    address: "native".into(),
                    symbol: config.get("symbol").and_then(Value::as_str).map(str::to_owned).or_else(|| r.chain.symbol.clone()),
                    name: config.get("asset_name").and_then(Value::as_str).map(str::to_owned).or_else(|| Some(r.chain.name.clone())),
                    decimals: config.get("decimals").and_then(Value::as_u64).map(|value| value as u32),
                    balance: amount,
                    is_native: true,
                });
            } else if native_denom.is_empty() && native.is_none() {
                native = Some(BalanceAsset {
                    address: "native".into(),
                    symbol: config.get("symbol").and_then(Value::as_str).map(str::to_owned).or_else(|| r.chain.symbol.clone()).or(Some(denom)),
                    name: config.get("asset_name").and_then(Value::as_str).map(str::to_owned).or_else(|| Some(r.chain.name.clone())),
                    decimals: config.get("decimals").and_then(Value::as_u64).map(|value| value as u32),
                    balance: amount,
                    is_native: true,
                });
            } else {
                tokens.push(BalanceAsset {
                    address: denom.clone(),
                    symbol: Some(denom),
                    name: None,
                    decimals: None,
                    balance: amount,
                    is_native: false,
                });
            }
        }
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(),
            chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(),
            address: r.address.clone(),
            native,
            tokens,
        });
    }


    // TRON /wallet/getaccount returns native TRX (SUN) and, when available,
    // a `trc20` array containing token contract -> raw balance maps. Preserve
    // both in the same ChainBalance so callers don't need a second token query.
    if config.get("response_mode").and_then(Value::as_str) == Some("tron_account") {
        let raw_native = response.get("balance")
            .and_then(|value| value.as_u64().map(|n| n.to_string())
                .or_else(|| value.as_str().and_then(|s| s.parse::<u128>().ok()).map(|n| n.to_string())))
            .unwrap_or_else(|| "0".to_owned());
        let mut tokens = Vec::new();
        if let Some(entries) = response.get("trc20").and_then(Value::as_array) {
            for entry in entries {
                let Some(object) = entry.as_object() else { continue; };
                for (contract, raw) in object {
                    let amount = raw.as_str().and_then(|s| s.parse::<u128>().ok())
                        .or_else(|| raw.as_u64().map(u128::from));
                    let Some(amount) = amount else { continue; };
                    if amount == 0 || contract.trim().is_empty() { continue; }
                    tokens.push(BalanceAsset {
                        address: contract.clone(),
                        symbol: None,
                        name: None,
                        // TRON's account response does not include TRC20 decimals;
                        // keep this unknown rather than incorrectly assuming 6/18.
                        decimals: None,
                        balance: amount.to_string(),
                        is_native: false,
                    });
                }
            }
        } else if let Some(entries) = response.get("trc20").and_then(Value::as_object) {
            // Accept object-shaped responses from compatible TRON indexers too.
            for (contract, raw) in entries {
                let amount = raw.as_str().and_then(|s| s.parse::<u128>().ok())
                    .or_else(|| raw.as_u64().map(u128::from));
                let Some(amount) = amount else { continue; };
                if amount == 0 || contract.trim().is_empty() { continue; }
                tokens.push(BalanceAsset {
                    address: contract.clone(), symbol: None, name: None,
                    decimals: None, balance: amount.to_string(), is_native: false,
                });
            }
        }
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(),
            chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(),
            address: r.address.clone(),
            native: Some(BalanceAsset {
                address: "native".into(),
                symbol: Some("TRX".into()),
                name: Some("TRON".into()),
                decimals: Some(6),
                balance: raw_native,
                is_native: true,
            }),
            tokens,
        });
    }


    // XRP account_info only includes native drops. account_lines is a separate
    // standard XRPL method and exposes issued-currency trust-line balances.
    if config.get("response_mode").and_then(Value::as_str) == Some("xrp_account") {
        let raw_native = response.pointer("/result/account_data/Balance")
            .or_else(|| response.pointer("/account_data/Balance"))
            .and_then(|v| v.as_str().map(str::to_owned)
                .or_else(|| v.as_u64().map(|n| n.to_string())))
            .ok_or_else(|| BalanceProviderError::Message(format!(
                "XRP account_info response omitted native Balance: {}", response
            )))?;
        let lines_result = request.rpc.execute_one(RpcRequest {
            provider: r.provider.clone(),
            chain_type: r.chain.normalized_type(),
            chain_id: r.chain.chain_id.clone(),
            endpoint: r.rpc_endpoint.clone(),
            method: "account_lines".into(),
            params: json!([{"account": r.address, "ledger_index": "validated"}]),
        }).await;
        let mut tokens = Vec::new();
        if let Ok(lines_response) = lines_result {
            let lines = lines_response.result.pointer("/result/lines")
                .or_else(|| lines_response.result.get("lines"))
                .and_then(Value::as_array);
            if let Some(lines) = lines {
                for line in lines {
                    let currency = line.get("currency").and_then(Value::as_str).unwrap_or_default();
                    let balance = line.get("balance").and_then(Value::as_str).unwrap_or_default();
                    let issuer = line.get("account").and_then(Value::as_str).unwrap_or_default();
                    if currency.is_empty() || balance.is_empty() || issuer.is_empty()
                        || balance == "0" || balance == "-0" { continue; }
                    tokens.push(BalanceAsset {
                        // Issued currencies are identified by both currency and issuer.
                        address: format!("{}:{}", issuer, currency),
                        symbol: Some(currency.to_owned()),
                        name: None,
                        decimals: None,
                        balance: balance.to_owned(),
                        is_native: false,
                    });
                }
            }
        } else { report_token_discovery_failure(request, "XRP account_lines request failed; native XRP balance was retained"); }
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(), chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(), address: r.address.clone(),
            native: Some(BalanceAsset {
                address: "native".into(), symbol: Some("XRP".into()),
                name: Some("XRP Ledger".into()), decimals: Some(6),
                balance: raw_native, is_native: true,
            }),
            tokens,
        });
    }

    // TzKT provides a token index alongside its native XTZ balance endpoint.
    // The second request is opt-in in the chain catalog; an indexer failure
    // does not erase a valid native balance.
    if config.get("response_mode").and_then(Value::as_str) == Some("tezos_account") {
        let raw_native = if response.is_number() {
            response.as_u64().map(|n| n.to_string())
        } else {
            response.as_str().and_then(|s| s.parse::<u128>().ok()).map(|n| n.to_string())
        }.ok_or_else(|| BalanceProviderError::Message(format!(
            "Tezos balance endpoint returned an invalid native amount: {}", response
        )))?;
        let mut tokens = Vec::new();
        if let Some(discovery) = config.get("token_discovery") {
            if let Some(path_template) = discovery.get("path").and_then(Value::as_str) {
                let path = replace_address_in_path(path_template, &r.address);
                let endpoint = format!("{}{}", r.rpc_endpoint.trim_end_matches('/'),
                    if path.starts_with('/') { path } else { format!("/{path}") });
                if let Ok(result) = request.rpc.execute_one(RpcRequest {
                    provider: r.provider.clone(), chain_type: "http".into(),
                    chain_id: r.chain.chain_id.clone(), endpoint,
                    method: "__http_get".into(), params: Value::Null,
                }).await {
                    if let Some(items) = result.result.as_array() {
                        for item in items {
                            let token = item.get("token").unwrap_or(item);
                            let token_address = token.get("contract").and_then(Value::as_str)
                                .or_else(|| token.pointer("/contract/address").and_then(Value::as_str))
                                .or_else(|| token.get("address").and_then(Value::as_str))
                                .or_else(|| item.pointer("/token/contract/address").and_then(Value::as_str))
                                .unwrap_or_default();
                            let raw = item.get("balance").and_then(|v| v.as_str().map(str::to_owned)
                                .or_else(|| v.as_u64().map(|n| n.to_string()))).unwrap_or_default();
                            if token_address.is_empty() || raw.is_empty() || raw == "0" { continue; }
                            let metadata = token.get("metadata").or_else(|| item.get("metadata"));
                            tokens.push(BalanceAsset {
                                address: token_address.to_owned(),
                                symbol: metadata.and_then(|m| m.get("symbol")).and_then(Value::as_str).map(str::to_owned),
                                name: metadata.and_then(|m| m.get("name")).and_then(Value::as_str).map(str::to_owned),
                                decimals: metadata.and_then(|m| m.get("decimals")).and_then(Value::as_u64).map(|n| n as u32),
                                balance: raw, is_native: false,
                            });
                        }
                    }
                } else { report_token_discovery_failure(request, "Tezos token discovery request failed; native XTZ balance was retained"); }
            }
        }
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(), chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(), address: r.address.clone(),
            native: Some(BalanceAsset {
                address: "native".into(), symbol: Some("XTZ".into()),
                name: Some("Tezos".into()), decimals: Some(6),
                balance: raw_native, is_native: true,
            }),
            tokens,
        });
    }


    // Aptos' account resource endpoint lists legacy CoinStore balances by
    // coin type. Keep the configured native coin result and add every other
    // non-zero CoinStore balance returned by the same fullnode.
    if config.get("response_mode").and_then(Value::as_str) == Some("aptos_account") {
        let native_value = response.get("balance").unwrap_or(&response);
        let raw_native = if native_value.is_number() {
            native_value.as_u64().map(|n| n.to_string())
        } else {
            native_value.as_str().and_then(|s| s.parse::<u128>().ok()).map(|n| n.to_string())
        }.ok_or_else(|| BalanceProviderError::Message(format!(
            "Aptos native balance endpoint returned an invalid amount: {}", response
        )))?;
        let mut tokens = Vec::new();
        if let Some(discovery) = config.get("token_discovery") {
            if let Some(path_template) = discovery.get("path").and_then(Value::as_str) {
                let path = replace_address_in_path(path_template, &r.address);
                let endpoint = format!("{}{}", r.rpc_endpoint.trim_end_matches('/'),
                    if path.starts_with('/') { path } else { format!("/{path}") });
                if let Ok(result) = request.rpc.execute_one(RpcRequest {
                    provider: r.provider.clone(), chain_type: "http".into(),
                    chain_id: r.chain.chain_id.clone(), endpoint,
                    method: "__http_get".into(), params: Value::Null,
                }).await {
                    if let Some(resources) = result.result.as_array() {
                        for resource in resources {
                            let type_name = resource.get("type").and_then(Value::as_str).unwrap_or_default();
                            if !type_name.contains("::coin::CoinStore<") { continue; }
                            let coin_type = type_name.split_once('<').map(|(_, rest)| rest.trim_end_matches('>').to_owned())
                                .unwrap_or_default();
                            if coin_type.is_empty() || coin_type.contains("aptos_coin::AptosCoin") { continue; }
                            let raw = resource.pointer("/data/coin/value")
                                .and_then(|v| v.as_str().map(str::to_owned)
                                    .or_else(|| v.as_u64().map(|n| n.to_string())))
                                .unwrap_or_default();
                            if raw.is_empty() || raw == "0" { continue; }
                            tokens.push(BalanceAsset {
                                address: coin_type.clone(),
                                symbol: coin_type.rsplit("::").next().map(str::to_owned),
                                name: None, decimals: None, balance: raw, is_native: false,
                            });
                        }
                    }
                } else { report_token_discovery_failure(request, "Aptos account resources request failed; native APT balance was retained"); }
            }
        }
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(), chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(), address: r.address.clone(),
            native: Some(BalanceAsset {
                address: "native".into(), symbol: Some("APT".into()),
                name: Some("Aptos".into()), decimals: Some(8),
                balance: raw_native, is_native: true,
            }),
            tokens,
        });
    }

    // Sui GraphQL's address balances connection enumerates owned coin types;
    // this is broader than querying only the native SUI coin type.
    if config.get("response_mode").and_then(Value::as_str) == Some("sui_graphql_assets") {
        if let Some(errors) = response.get("errors").and_then(Value::as_array) {
            if !errors.is_empty() {
                return Err(BalanceProviderError::Message(format!(
                    "Sui GraphQL returned errors: {}", serde_json::to_string(errors).unwrap_or_default()
                )));
            }
        }
        let nodes = response.pointer("/data/address/balances/nodes")
            .and_then(Value::as_array).ok_or_else(|| BalanceProviderError::Message(format!(
                "Sui GraphQL response omitted data.address.balances.nodes: {}", response
            )))?;
        let mut native_amount = None;
        let mut tokens = Vec::new();
        for node in nodes {
            let coin_type = node.pointer("/coinType/repr").and_then(Value::as_str)
                .or_else(|| node.get("coinType").and_then(Value::as_str)).unwrap_or_default();
            let amount = node.get("totalBalance").and_then(|v| v.as_str().map(str::to_owned)
                .or_else(|| v.as_u64().map(|n| n.to_string()))).unwrap_or_default();
            if coin_type.is_empty() || amount.is_empty() || amount == "0" { continue; }
            if coin_type == "0x2::sui::SUI" {
                native_amount = Some(amount);
            } else {
                tokens.push(BalanceAsset {
                    address: coin_type.to_owned(),
                    symbol: coin_type.rsplit("::").next().map(str::to_owned),
                    name: None, decimals: None, balance: amount, is_native: false,
                });
            }
        }
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(), chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(), address: r.address.clone(),
            native: Some(BalanceAsset {
                address: "native".into(), symbol: Some("SUI".into()),
                name: Some("Sui".into()), decimals: Some(9),
                balance: native_amount.unwrap_or_else(|| "0".into()), is_native: true,
            }),
            tokens,
        });
    }


    // NEAR's RPC cannot enumerate arbitrary NEP-141 contracts. For contracts
    // supplied in BalanceRequest.assets, query ft_balance_of and include each
    // confirmed balance alongside the native NEAR amount.
    if config.get("response_mode").and_then(Value::as_str) == Some("near_account") {
        let raw_native_value = response.get("amount").or_else(|| response.pointer("/result/amount")).unwrap_or(&response);
        let raw_native = raw_native_value.as_str().and_then(|s| s.parse::<u128>().ok())
            .or_else(|| raw_native_value.as_u64().map(u128::from))
            .map(|n| n.to_string())
            .ok_or_else(|| BalanceProviderError::Message(format!(
                "NEAR account response omitted a valid amount: {}", response
            )))?;
        let mut tokens = Vec::new();
        for contract in &r.assets {
            let contract = contract.trim();
            if contract.is_empty() { continue; }
            let args = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                serde_json::to_vec(&json!({"account_id": r.address})).unwrap_or_default(),
            );
            let token_result = request.rpc.execute_one(RpcRequest {
                provider: r.provider.clone(), chain_type: r.chain.normalized_type(),
                chain_id: r.chain.chain_id.clone(), endpoint: r.rpc_endpoint.clone(),
                method: "query".into(),
                params: json!({
                    "request_type": "call_function", "finality": "final",
                    "account_id": contract, "method_name": "ft_balance_of",
                    "args_base64": args
                }),
            }).await;
            let Ok(token_result) = token_result else {
                report_token_discovery_failure(request, format!("NEAR ft_balance_of request failed for contract {}", contract));
                continue;
            };
            let result = token_result.result.get("result").and_then(Value::as_array);
            let bytes = match result {
                Some(values) => values.iter().filter_map(Value::as_u64).map(|v| v as u8).collect::<Vec<_>>(),
                None => Vec::new(),
            };
            let amount = String::from_utf8(bytes).ok().and_then(|s| s.parse::<u128>().ok());
            let Some(amount) = amount else { continue; };
            if amount == 0 { continue; }
            tokens.push(BalanceAsset {
                address: contract.to_owned(), symbol: None, name: None,
                decimals: None, balance: amount.to_string(), is_native: false,
            });
        }
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(), chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(), address: r.address.clone(),
            native: Some(BalanceAsset {
                address: "native".into(), symbol: Some("NEAR".into()),
                name: Some("NEAR".into()), decimals: Some(24),
                balance: raw_native, is_native: true,
            }),
            tokens,
        });
    }

    // Algorand account responses already enumerate owned ASA holdings.
    if config.get("response_mode").and_then(Value::as_str) == Some("algorand_account") {
        let raw_native = response.get("amount").and_then(Value::as_u64)
            .map(|n| n.to_string()).ok_or_else(|| BalanceProviderError::Message(format!(
                "Algorand account response omitted amount: {}", response
            )))?;
        let mut tokens = Vec::new();
        if let Some(assets) = response.get("assets").and_then(Value::as_array) {
            for asset in assets {
                let id = asset.get("asset-id").and_then(Value::as_u64)
                    .or_else(|| asset.get("asset_id").and_then(Value::as_u64));
                let amount = asset.get("amount").and_then(Value::as_u64);
                let (Some(id), Some(amount)) = (id, amount) else { continue; };
                if amount == 0 { continue; }
                tokens.push(BalanceAsset {
                    address: id.to_string(), symbol: None, name: None,
                    decimals: None, balance: amount.to_string(), is_native: false,
                });
            }
        }
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(), chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(), address: r.address.clone(),
            native: Some(BalanceAsset {
                address: "native".into(), symbol: Some("ALGO".into()),
                name: Some("Algorand".into()), decimals: Some(6),
                balance: raw_native, is_native: true,
            }),
            tokens,
        });
    }


    // Hedera mirror-node account responses expose native HBAR separately from
    // token holdings. Query the account token endpoint and retain all non-zero
    // HTS token balances, with decimals when the mirror node provides them.
    if config.get("response_mode").and_then(Value::as_str) == Some("hedera_account") {
        let raw_native = response.pointer("/balance/balance")
            .or_else(|| response.get("balance"))
            .and_then(|v| v.as_str().and_then(|s| s.parse::<u128>().ok())
                .or_else(|| v.as_u64().map(u128::from)))
            .map(|n| n.to_string()).ok_or_else(|| BalanceProviderError::Message(format!(
                "Hedera account response omitted a valid HBAR balance: {}", response
            )))?;
        let mut tokens = Vec::new();
        if let Some(discovery) = config.get("token_discovery") {
            if let Some(path_template) = discovery.get("path").and_then(Value::as_str) {
                let path = replace_address_in_path(path_template, &r.address);
                let endpoint = format!("{}{}", r.rpc_endpoint.trim_end_matches('/'),
                    if path.starts_with('/') { path } else { format!("/{path}") });
                if let Ok(result) = request.rpc.execute_one(RpcRequest {
                    provider: r.provider.clone(), chain_type: "http".into(),
                    chain_id: r.chain.chain_id.clone(), endpoint,
                    method: "__http_get".into(), params: Value::Null,
                }).await {
                    if let Some(items) = result.result.get("tokens").and_then(Value::as_array) {
                        for item in items {
                            let id = item.get("token_id").and_then(Value::as_str).unwrap_or_default();
                            let amount = item.get("balance").and_then(|v| v.as_str().and_then(|s| s.parse::<u128>().ok())
                                .or_else(|| v.as_u64().map(u128::from)));
                            let amount = amount.unwrap_or(0);
                            if id.is_empty() || amount == 0 { continue; }
                            tokens.push(BalanceAsset {
                                address: id.to_owned(),
                                symbol: item.get("symbol").and_then(Value::as_str).map(str::to_owned),
                                name: item.get("name").and_then(Value::as_str).map(str::to_owned),
                                decimals: item.get("decimals").and_then(Value::as_u64).map(|n| n as u32),
                                balance: amount.to_string(), is_native: false,
                            });
                        }
                    }
                }
            }
        }
        return Ok(ChainBalance {
            wallet_id: r.wallet_id.clone(), chain_id: r.chain.chain_id.clone(),
            chain_type: r.chain.chain_type.clone(), address: r.address.clone(),
            native: Some(BalanceAsset {
                address: "native".into(), symbol: Some("HBAR".into()),
                name: Some("Hedera".into()), decimals: Some(8),
                balance: raw_native, is_native: true,
            }),
            tokens,
        });
    }

    let result_path = config.get("result_path").and_then(Value::as_str).unwrap_or("");
    let empty_object_is_zero = config.get("empty_object_is_zero").and_then(Value::as_bool) == Some(true)
        && response.as_object().map(|object| object.is_empty()).unwrap_or(false);
    // Some account APIs omit the native balance property when it is zero,
    // even when the account object contains token/permission/staking data.
    // This behavior is opt-in per chain so malformed responses on other
    // networks continue to fail loudly.
    let missing_result_is_zero = config.get("missing_result_is_zero").and_then(Value::as_bool) == Some(true);
    let raw_balance = if empty_object_is_zero {
        "0".to_owned()
    } else {
        let mut balance_value = &response;
        let mut missing_configured_path = false;
        if !result_path.is_empty() {
            for segment in result_path.split('.') {
                match balance_value.get(segment) {
                    Some(value) => balance_value = value,
                    None if missing_result_is_zero => {
                        missing_configured_path = true;
                        break;
                    }
                    None => return Err(BalanceProviderError::Message(format!(
                        "{} balance response omitted configured field '{}': {}",
                        r.chain.name, result_path, response
                    ))),
                }
            }
        }
        if missing_configured_path {
            "0".to_owned()
        } else {
            if balance_value.is_number() {
                balance_value.as_u64().map(|value| value.to_string())
            } else if let Some(text) = balance_value.as_str() {
                let encoding = config.get("value_encoding").and_then(Value::as_str).unwrap_or("integer");
                match encoding {
                    "hex_quantity" => u128::from_str_radix(text.trim_start_matches("0x"), 16).ok().map(|value| value.to_string()),
                    _ => text.parse::<u128>().ok().map(|value| value.to_string()),
                }
            } else {
                None
            }.ok_or_else(|| BalanceProviderError::Message(format!(
                "{} balance response did not contain a valid configured integer at '{}': {}",
                r.chain.name, result_path, response
            )))?
        }
    };

    let decimals = config.get("decimals").and_then(Value::as_u64).map(|value| value as u32);
    let symbol = config.get("symbol").and_then(Value::as_str)
        .map(str::to_owned).or_else(|| r.chain.symbol.clone());
    let name = config.get("asset_name").and_then(Value::as_str)
        .map(str::to_owned).unwrap_or_else(|| r.chain.name.clone());

    let mut tokens = Vec::new();
    // Solana's getBalance returns only lamports. Enumerate both the original
    // SPL Token program and Token-2022 accounts so the same balance request
    // also returns token mints and their raw balances.
    if r.chain.normalized_type().eq_ignore_ascii_case("solana") {
        for program_id in [
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
            "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
        ] {
            let token_accounts = request.rpc.execute_one(RpcRequest {
                provider: r.provider.clone(),
                chain_type: r.chain.normalized_type(),
                chain_id: r.chain.chain_id.clone(),
                endpoint: r.rpc_endpoint.clone(),
                method: "getTokenAccountsByOwner".into(),
                params: json!([
                    r.address,
                    {"programId": program_id},
                    {"encoding": "jsonParsed"}
                ]),
            }).await;

            let Ok(token_accounts) = token_accounts else {
                // Some RPC providers disable one token program's query.
                // Preserve native balance and any token accounts discovered
                // through the other program rather than failing the whole call.
                continue;
            };
            let Some(accounts) = token_accounts.result.get("value").and_then(Value::as_array) else {
                continue;
            };
            for account in accounts {
                let info = match account.pointer("/account/data/parsed/info") {
                    Some(value) => value,
                    None => continue,
                };
                let mint = match info.get("mint").and_then(Value::as_str) {
                    Some(value) if !value.is_empty() => value.to_owned(),
                    _ => continue,
                };
                if tokens.iter().any(|asset: &BalanceAsset| asset.address == mint) {
                    continue;
                }
                let amount = match info.pointer("/tokenAmount/amount").and_then(Value::as_str) {
                    Some(value) if value.bytes().all(|byte| byte.is_ascii_digit()) => value.to_owned(),
                    _ => continue,
                };
                let token_decimals = info.pointer("/tokenAmount/decimals")
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok());
                tokens.push(BalanceAsset {
                    address: mint,
                    symbol: None,
                    name: None,
                    decimals: token_decimals,
                    balance: amount,
                    is_native: false,
                });
            }
        }

        // Standard Solana getTokenAccountsByOwner only returns mint addresses,
        // raw balances, and decimals; it does not include token display metadata.
        // Enrich mints through the Digital Asset Standard (DAS) getAsset method
        // when the configured endpoint/provider supports it (Alchemy/Helius).
        // Metadata is best-effort: unsupported DAS methods never erase balances.
        let endpoint_lower = r.rpc_endpoint.to_ascii_lowercase();
        let provider_lower = r.provider.to_ascii_lowercase();
        let supports_das = provider_lower.contains("alchemy")
            || provider_lower.contains("helius")
            || endpoint_lower.contains("alchemy")
            || endpoint_lower.contains("helius");
        if supports_das && !tokens.is_empty() {
            let metadata_requests = tokens.iter().map(|token| RpcRequest {
                provider: r.provider.clone(),
                chain_type: r.chain.normalized_type(),
                chain_id: r.chain.chain_id.clone(),
                endpoint: r.rpc_endpoint.clone(),
                method: "getAsset".into(),
                params: json!([{"id": token.address}]),
            }).collect::<Vec<_>>();

            let metadata_results = request.rpc.execute(metadata_requests).await;
            for (token, result) in tokens.iter_mut().zip(metadata_results) {
                let Ok(asset_response) = result else { continue; };
                let asset = &asset_response.result;
                let asset = asset.get("result").unwrap_or(asset);
                let metadata = asset.pointer("/content/metadata");
                if token.name.is_none() {
                    token.name = metadata.and_then(|m| m.get("name"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned);
                }
                if token.symbol.is_none() {
                    token.symbol = metadata.and_then(|m| m.get("symbol"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned)
                        .or_else(|| asset.pointer("/token_info/symbol")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(str::to_owned));
                }
                if token.decimals.is_none() {
                    token.decimals = asset.pointer("/token_info/decimals")
                        .and_then(Value::as_u64)
                        .and_then(|value| u32::try_from(value).ok());
                }
            }
        }
    }

    Ok(ChainBalance {
        wallet_id: r.wallet_id.clone(),
        chain_id: r.chain.chain_id.clone(),
        chain_type: r.chain.chain_type.clone(),
        address: r.address.clone(),
        native: Some(BalanceAsset {
            address: "native".into(),
            symbol,
            name: Some(name),
            decimals,
            balance: raw_balance,
            is_native: true,
        }),
        tokens,
    })
}

#[cfg(test)]
mod evm_uint_tests {
    use super::{balance_method_config, decode_uint};
    use serde_json::json;

    #[test]
    fn confirmed_zero_is_a_valid_uint_balance() {
        let value = decode_uint(&json!("0x0")).expect("zero is a valid uint256");
        assert!(value.is_zero());
        assert_eq!(value.to_string(), "0");
    }

    #[test]
    fn uint256_values_larger_than_u128_are_preserved() {
        let value = decode_uint(&json!("0x100000000000000000000000000000000"))
            .expect("valid uint256 should decode without u128 overflow");
        assert_eq!(value.to_string(), "340282366920938463463374607431768211456");
    }



    #[test]
    fn every_supported_catalog_network_has_a_balance_method() {
        let catalog: serde_json::Value = serde_json::from_str(
            include_str!("../../../../static/dev_ui/blockchains.json")
        ).expect("blockchain catalog must be valid JSON");
        let networks = catalog.get("networks").and_then(serde_json::Value::as_array)
            .expect("catalog must contain networks");
        for network in networks.iter().filter(|network| network.get("adapter_supported").and_then(serde_json::Value::as_bool) == Some(true)) {
            assert!(network.get("balance_method").and_then(serde_json::Value::as_object).is_some(),
                "supported network {} is missing balance_method", network.get("name").and_then(serde_json::Value::as_str).unwrap_or("<unnamed>"));
        }
    }

    #[test]
    fn solana_balance_method_is_read_from_catalog() {
        let mut chain = crate::types::chain::ChainRequest::new("Solana Mainnet", "solana");
        chain.chain_id = "solana-mainnet".into();
        let config = balance_method_config(&chain).expect("Solana config should exist in catalog");
        assert_eq!(config.get("executor").and_then(serde_json::Value::as_str), Some("json_rpc"));
        assert_eq!(config.get("method").and_then(serde_json::Value::as_str), Some("getBalance"));
        assert_eq!(config.get("result_path").and_then(serde_json::Value::as_str), Some("value"));
    }

    #[test]
    fn tron_balance_method_is_read_from_catalog() {
        let mut chain = crate::types::chain::ChainRequest::new("Tron Mainnet", "tron");
        chain.chain_id = "195".into();
        let config = balance_method_config(&chain).expect("TRON config should exist in catalog");
        assert_eq!(config.get("executor").and_then(serde_json::Value::as_str), Some("http_post"));
        assert_eq!(config.get("path").and_then(serde_json::Value::as_str), Some("/wallet/getaccount"));
        assert_eq!(config.get("result_path").and_then(serde_json::Value::as_str), Some("balance"));
    }


    #[test]
    fn alchemy_utxo_balance_method_is_catalog_driven() {
        let mut chain = crate::types::chain::ChainRequest::new("Bitcoin Mainnet", "btc");
        chain.chain_id = "bitcoin".into();
        let config = balance_method_config(&chain).expect("Bitcoin config should exist in catalog");
        assert_eq!(config.get("executor").and_then(serde_json::Value::as_str), Some("http_get"));
        assert_eq!(config.get("path").and_then(serde_json::Value::as_str), Some("/api/v2/address/{address}"));
        assert_eq!(config.get("response_mode").and_then(serde_json::Value::as_str), Some("sum_fields"));
    }

    #[test]
    fn cosmos_balance_method_is_catalog_driven() {
        let mut chain = crate::types::chain::ChainRequest::new("Cosmos Hub", "cosmos");
        chain.chain_id = "cosmoshub-4".into();
        let config = balance_method_config(&chain).expect("Cosmos config should exist in catalog");
        assert_eq!(config.get("executor").and_then(serde_json::Value::as_str), Some("http_get"));
        assert_eq!(config.get("response_mode").and_then(serde_json::Value::as_str), Some("cosmos_balances"));
    }


    #[test]
    fn starknet_balance_method_is_catalog_driven() {
        let mut chain = crate::types::chain::ChainRequest::new("Starknet Mainnet", "starknet");
        chain.chain_id = "0x534e5f4d41494e".into();
        let config = balance_method_config(&chain).expect("Starknet config should exist in catalog");
        assert_eq!(config.get("executor").and_then(serde_json::Value::as_str), Some("json_rpc"));
        assert_eq!(config.get("method").and_then(serde_json::Value::as_str), Some("starknet_call"));
        assert_eq!(config.get("response_mode").and_then(serde_json::Value::as_str), Some("u256_limbs"));
    }


    #[test]
    fn cardano_method_includes_environment_configured_auth() {
        let mut chain = crate::types::chain::ChainRequest::new("Cardano", "cardano");
        chain.chain_id = "1815".into();
        let config = balance_method_config(&chain).expect("Cardano config should exist in catalog");
        assert_eq!(config.get("executor").and_then(serde_json::Value::as_str), Some("http_get"));
        assert_eq!(config.get("path").and_then(serde_json::Value::as_str), Some("/addresses/{address}"));
        assert_eq!(config.get("headers").and_then(|headers| headers.get("project_id")).and_then(|header| header.get("env")).and_then(serde_json::Value::as_str), Some("BLOCKFROST_PROJECT_ID"));
    }

    #[test]
    fn injective_catalog_uses_evm_balance_executor() {
        let mut chain = crate::types::chain::ChainRequest::new("Injective Mainnet", "evm");
        chain.chain_id = "1776".into();
        let config = balance_method_config(&chain).expect("Injective config should exist in catalog");
        assert_eq!(config.get("executor").and_then(serde_json::Value::as_str), Some("evm"));
    }

    #[test]
    fn http_path_templates_replace_the_address() {
        assert_eq!(
            super::replace_address_in_path("/v2/accounts/{address}/balance", "REAL_CHAIN_ADDRESS"),
            "/v2/accounts/REAL_CHAIN_ADDRESS/balance"
        );
    }

    #[test]
    fn malformed_uint_is_an_error_not_zero() {
        assert!(decode_uint(&json!("not-hex")).is_err());
    }
}
