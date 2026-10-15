use crate::adapters::adapter_trait::WalletAdapter;
use crate::common::errors::EngineError;
use crate::crypto::bip32::{derive_child_key_ed25519, derive_master_key_ed25519};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};
use crate::crypto::derivation::DerivationPath;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};

const SOLANA_TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const SOLANA_NATIVE_MINT: &str = "So11111111111111111111111111111111111111112";

/// Minimal chain configuration for Solana adapter.
#[derive(Clone, Debug)]
pub struct ChainConfig {
    pub rpc_url: String,
    pub name: Option<String>,
}

/// Asset row used by the adapter.
#[derive(Clone, Debug)]
pub struct AssetRow {
    pub address: String,
    pub symbol: String,
    pub decimals: Option<u32>,
    pub is_native: bool,
    pub balance: String,
}

pub struct SolanaAdapter {
    client: Option<Client>,
    rpc_url: String,
}

impl SolanaAdapter {
    pub fn new(chain: ChainConfig) -> Self {
        let rpc_url = chain.rpc_url.clone();
        let client = if rpc_url.is_empty() { None } else { Some(Client::new()) };
        Self { client, rpc_url }
    }

    /// Normalize raw Solana lamports/SPL amounts into display balances.
    /// Network access remains in the provider/RPC layer.
    pub fn normalize_balances(assets: Vec<AssetRow>) -> Vec<AssetRow> {
        assets.into_iter().map(|mut asset| {
            let decimals = asset.decimals.unwrap_or(if asset.is_native { 9 } else { 0 });
            asset.balance = normalize_raw_amount(&asset.balance, decimals)
                .unwrap_or_else(|| "0".to_string());
            asset
        }).collect()
    }



    pub async fn check_account_security(&self, owner_address: &str, mint_address: Option<&str>) -> Option<SolanaSecurityResult> {
        if self.client.is_none() || self.rpc_url.is_empty() {
            return None;
        }

        let client = self.client.as_ref().unwrap();
        let accounts_payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getParsedTokenAccountsByOwner",
            "params": [
                owner_address,
                { "programId": SOLANA_TOKEN_PROGRAM },
                { "encoding": "jsonParsed" }
            ]
        });

        let token_accounts = match client.post(&self.rpc_url).json(&accounts_payload).send().await {
            Ok(resp) => match resp.json::<Value>().await {
                Ok(parsed) => Some(parsed),
                Err(_) => None,
            },
            Err(_) => None,
        };

        let token_accounts_value = token_accounts
            .as_ref()
            .and_then(|json| json.get("result"))
            .and_then(|result| result.get("value"))
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();

        let empty_accounts_count = token_accounts_value
            .iter()
            .filter(|account| {
                account
                    .pointer("/account/data/parsed/info/tokenAmount/uiAmount")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
                    == 0.0
            })
            .count();

        let mut authorities = Authorities { freeze: false, mint: false };
        let mut delegate = DelegateInfo { active: false, amount: 0.0 };

        if let Some(mint_address) = mint_address {
            if mint_address != SOLANA_NATIVE_MINT {
                let mint_payload = json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "getParsedAccountInfo",
                    "params": [mint_address, { "encoding": "jsonParsed", "commitment": "confirmed" }]
                });

                if let Ok(parsed) = match client.post(&self.rpc_url).json(&mint_payload).send().await {
                    Ok(resp) => resp.json::<Value>().await,
                    Err(err) => Err(err),
                } {
                    if let Some(info) = parsed
                        .pointer("/result/value/data/parsed/info")
                    {
                        authorities.freeze = info.get("freezeAuthority").is_some();
                        authorities.mint = info.get("mintAuthority").is_some();
                    }
                }

                if let Some(user_token_account) = token_accounts_value.iter().find(|account| {
                    account
                        .pointer("/account/data/parsed/info/mint")
                        .and_then(|v| v.as_str())
                        .map(|mint| mint == mint_address)
                        .unwrap_or(false)
                }) {
                    if let Some(info) = user_token_account.pointer("/account/data/parsed/info") {
                        if let Some(delegate_val) = info.get("delegate") {
                            delegate.active = !delegate_val.is_null();
                        }
                        delegate.amount = info
                            .pointer("/delegatedAmount/uiAmount")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                    }
                }
            }
        }

        Some(SolanaSecurityResult {
            wallet_type: "Standard (Individual Node)".to_string(),
            delegations: if delegate.active { 1 } else { 0 },
            delegate_amount: delegate.amount,
            freeze_authority: authorities.freeze,
            mint_authority: authorities.mint,
            reclaimable_sol: empty_accounts_count as f64 * 0.002,
            empty_accounts_count,
            security_score: if authorities.freeze { "Moderate".to_string() } else { "Excellent".to_string() },
        })
    }
}

fn normalize_raw_amount(raw: &str, decimals: u32) -> Option<String> {
    if raw.trim().is_empty() {
        return None;
    }
    let raw = raw.trim();
    let negative = raw.starts_with('-');
    if negative {
        return None;
    }
    if !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let digits = raw.trim_start_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };

    if decimals == 0 {
        return Some(digits.to_string());
    }

    let decimals = decimals as usize;
    if digits.len() <= decimals {
        let fraction = format!("{:0>width$}", digits, width = decimals);
        let fraction = fraction.trim_end_matches('0');
        if fraction.is_empty() {
            Some("0".to_string())
        } else {
            Some(format!("0.{}", fraction))
        }
    } else {
        let split = digits.len() - decimals;
        let whole = &digits[..split];
        let fraction = digits[split..].trim_end_matches('0');
        if fraction.is_empty() {
            Some(whole.to_string())
        } else {
            Some(format!("{}.{}", whole, fraction))
        }
    }
}

#[derive(Deserialize)]
struct SolanaBalanceResponse {
    result: SolanaBalanceResult,
}

#[derive(Deserialize)]
struct SolanaBalanceResult {
    value: u64,
}

#[derive(Clone, Debug)]
pub struct SolanaSecurityResult {
    pub wallet_type: String,
    pub delegations: usize,
    pub delegate_amount: f64,
    pub freeze_authority: bool,
    pub mint_authority: bool,
    pub reclaimable_sol: f64,
    pub empty_accounts_count: usize,
    pub security_score: String,
}

#[derive(Default)]
struct Authorities {
    freeze: bool,
    mint: bool,
}

#[derive(Default)]
struct DelegateInfo {
    active: bool,
    amount: f64,
}

impl WalletAdapter for SolanaAdapter {
    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
        if public_key.len() < 32 {
            return Err(EngineError::Validation("ed25519 public key expected".into()));
        }
        Ok(bs58::encode(&public_key[..32]).into_string())
    }

    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
        let derivation_path = DerivationPath::parse(path.to_string())?;
        let master_key = derive_master_key_ed25519(seed)?;
        let child_key = derive_child_key_ed25519(&master_key, &derivation_path)?;
        let secret = SecretKey::from_bytes(&child_key.key)
            .map_err(|_| EngineError::Crypto("invalid derived Solana secret key".into()))?;
        let public = PublicKey::from(&secret);
        Self::pub_key_to_address(public.as_bytes())
    }

    fn address_from_private_key(private_key: &[u8]) -> Result<String, EngineError> {
        if private_key.len() != 32 { return Err(EngineError::Validation("Solana private key must be exactly 32 bytes".into())); }
        let secret = SecretKey::from_bytes(private_key).map_err(|_| EngineError::Crypto("invalid Solana private key".into()))?;
        let public = PublicKey::from(&secret);
        Self::pub_key_to_address(public.as_bytes())
    }

    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        if private_key.len() != 32 {
            return Err(EngineError::Validation("Solana private key must be exactly 32 bytes".into()));
        }
        let secret = SecretKey::from_bytes(private_key)
            .map_err(|_| EngineError::Crypto("invalid Solana private key".into()))?;
        let public = PublicKey::from(&secret);
        let keypair = Keypair { secret, public };
        Ok(keypair.sign(tx_data).to_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::adapter_trait::WalletAdapter;

    #[tokio::test]
    async fn solana_adapter_returns_zero_balance_without_rpc() {
        let chain = ChainConfig { rpc_url: "".to_string(), name: None };
        let adapter = SolanaAdapter::new(chain);

        let assets = vec![AssetRow {
            address: "".to_string(),
            symbol: "SOL".to_string(),
            decimals: Some(9),
            is_native: true,
            balance: "".to_string(),
        }];

        let balances = SolanaAdapter::normalize_balances(assets);
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].balance, "0");
    }

    #[test]
    fn solana_adapter_signs_message_with_ed25519() {
        let private_key = [0x11u8; 32];
        let signature = SolanaAdapter::sign_transaction(&private_key, b"nodex-test")
            .expect("signature should be generated");
        assert_eq!(signature.len(), 64);
    }

    #[test]
    fn solana_adapter_encodes_public_key_to_base58() {
        let public_key = [0x11u8; 32];
        let address = SolanaAdapter::pub_key_to_address(&public_key).expect("address should encode");
        assert!(!address.is_empty());
    }
}
