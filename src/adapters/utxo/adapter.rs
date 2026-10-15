use k256::elliptic_curve::sec1::ToEncodedPoint;
use crate::adapters::adapter_trait::WalletAdapter;
use crate::common::errors::EngineError;
use crate::crypto::address::{p2pkh_address, p2sh_p2wpkh_address, p2wpkh_address};
use crate::crypto::bip32::{derive_child_key, derive_master_key};
use crate::crypto::derivation::DerivationPath;
use crate::crypto::signing::sign_secp256k1_payload;
use crate::types::chain::ChainRequest;
use std::collections::HashMap;

pub struct UtxoAdapter;

impl UtxoAdapter {
    fn metadata(chain: &ChainRequest) -> Result<(u8, Option<u8>, String, String), EngineError> {
        let version = chain.p2pkh_version.ok_or_else(|| EngineError::Validation(format!(
            "p2pkh_version is required for UTXO blockchain {}", chain.name
        )))?;
        let hrp = chain.bech32_hrp.clone().unwrap_or_default();

        let path = match chain.derivation_path.as_deref() {
            Some(p) if !p.trim().is_empty() => p.to_string(),
            _ => return Err(EngineError::Validation(format!(
                "derivation_path is required for UTXO blockchain {}", chain.name
            ))),
        };

        Ok((version, chain.p2sh_version, hrp, path))
    }

    fn derive_with_path(
        seed: &[u8],
        path: &str,
        p2pkh_version: u8,
        p2sh_version: Option<u8>,
        address_type: &str,
        hrp: &str,
    ) -> Result<String, EngineError> {
        let derivation_path = DerivationPath::parse(path.to_string())?;
        let master_key = derive_master_key(seed)?;
        let child_key = derive_child_key(&master_key, &derivation_path)?;
        let secret = k256::SecretKey::from_bytes((&child_key.key).into())
            .map_err(|_| EngineError::Crypto("invalid derived UTXO private key".into()))?;
        let public = secret.public_key().to_encoded_point(true);
        let bytes = public.as_bytes();

        match address_type {
            "p2pkh" => p2pkh_address(bytes, p2pkh_version).map_err(EngineError::Validation),
            "p2wpkh" => {
                if hrp.is_empty() {
                    return Err(EngineError::Validation("bech32_hrp is required for p2wpkh".into()));
                }
                p2wpkh_address(bytes, hrp).map_err(EngineError::Validation)
            }
            "p2sh-p2wpkh" => {
                let sh_version = p2sh_version.ok_or_else(|| EngineError::Validation(
                    "p2sh_version is required for p2sh-p2wpkh".into()
                ))?;
                p2sh_p2wpkh_address(bytes, sh_version).map_err(EngineError::Validation)
            }
            other => Err(EngineError::Validation(format!("unsupported UTXO address_type: {}", other))),
        }
    }

    fn address_type(chain: &ChainRequest) -> String {
        chain.address_type.as_deref().unwrap_or("p2pkh").trim().to_ascii_lowercase()
    }

    pub fn derive_addresses(seed: &[u8], chains: &[ChainRequest]) -> Result<HashMap<String, String>, EngineError> {
        let mut result = HashMap::with_capacity(chains.len());
        for chain in chains {
            let (version, p2sh_version, hrp, path) = Self::metadata(chain)?;
            let address = Self::derive_with_path(seed, &path, version, p2sh_version, &Self::address_type(chain), &hrp)?;
            result.insert(chain.name.clone(), address);
        }
        Ok(result)
    }

    pub fn normalize_balance(raw: &str, decimals: u32) -> Result<String, EngineError> {
        let value = raw.trim().parse::<u128>()
            .map_err(|_| EngineError::Validation("invalid UTXO integer balance".into()))?;
        if decimals == 0 { return Ok(value.to_string()); }
        let base = 10u128.pow(decimals);
        let whole = value / base;
        let fraction = value % base;
        if fraction == 0 { return Ok(whole.to_string()); }
        let mut fraction_text = format!("{:0width$}", fraction, width = decimals as usize);
        while fraction_text.ends_with('0') { fraction_text.pop(); }
        Ok(format!("{}.{}", whole, fraction_text))
    }
}

impl WalletAdapter for UtxoAdapter {
    fn derive_addresses(seed: &[u8], chains: &[ChainRequest]) -> Result<HashMap<String, String>, EngineError> {
        Self::derive_addresses(seed, chains)
    }

    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
        if public_key.len() < 33 {
            return Err(EngineError::Validation("compressed public key expected".into()));
        }
        p2pkh_address(&public_key[..33], 0).map_err(EngineError::Validation)
    }

    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
        Self::derive_with_path(seed, path, 0, None, "p2pkh", "")
    }

    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        sign_secp256k1_payload(private_key, tx_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain(name: &str, symbol: &str) -> ChainRequest {
        let mut c = ChainRequest::new(name, "btc");
        c.symbol = Some(symbol.into());
        c.p2pkh_version = Some(if symbol == "LTC" { 48 } else { 0 });
        c.p2sh_version = Some(if symbol == "LTC" { 50 } else { 5 });
        c.derivation_path = Some(if symbol == "LTC" { "m/44'/2'/0'/0/0" } else { "m/44'/0'/0'/0/0" }.into());
        c
    }

    #[test]
    fn derives_bitcoin_and_litecoin_from_one_family_adapter() {
        let seed = [0x42u8; 64];
        let result = UtxoAdapter::derive_addresses(&seed, &[chain("Bitcoin", "BTC"), chain("Litecoin", "LTC")]).unwrap();
        assert!(result.get("Bitcoin").is_some());
        assert!(result.get("Litecoin").is_some());
        assert_ne!(result["Bitcoin"], result["Litecoin"]);
    }

    #[test]
    fn normalizes_base_units() {
        assert_eq!(UtxoAdapter::normalize_balance("123456789", 8).unwrap(), "1.23456789");
    }
}
