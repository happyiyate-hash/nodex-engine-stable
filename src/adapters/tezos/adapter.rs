use crate::adapters::adapter_trait::WalletAdapter;
use crate::crypto::bip32::{derive_child_key, derive_child_key_ed25519, derive_master_key, derive_master_key_ed25519};
use crate::crypto::derivation::DerivationPath;
use crate::crypto::signing::sign_ed25519_payload;
use blake2::{Blake2b512, Digest as BlakeDigest};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use sha2::Digest;
use p256::ecdsa::signature::Signer;
use crate::common::errors::EngineError;

/// Minimal chain configuration for the Tezos adapter.
#[derive(Clone, Debug)]
pub struct ChainConfig {
    pub rpc_url: String,
    pub name: Option<String>,
}

/// Asset representation used by the adapter.
#[derive(Clone, Debug)]
pub struct AssetRow {
    pub address: String,
    pub symbol: String,
    pub decimals: Option<u32>,
    pub is_native: bool,
    pub balance: String,
}

pub struct TezosAdapter;

impl TezosAdapter {
    pub const DEFAULT_DERIVATION_PATH: &'static str = "m/44'/1729'/0'/0'";
    pub fn normalize_raw_amount(raw: &str, decimals: u32) -> Result<String, EngineError> {
        let value = raw.trim().parse::<u128>().map_err(|_| EngineError::Validation("invalid Tezos asset amount".into()))?;
        if decimals == 0 { return Ok(value.to_string()); }
        let scale = 10u128.checked_pow(decimals).ok_or_else(|| EngineError::Validation("Tezos decimals exceed supported range".into()))?;
        let whole=value/scale; let fraction=value%scale;
        if fraction==0 { return Ok(whole.to_string()); }
        let mut f=format!("{:0width$}",fraction,width=decimals as usize); while f.ends_with('0'){f.pop();}
        Ok(format!("{}.{}",whole,f))
    }
    pub fn normalize_balances(assets: Vec<AssetRow>) -> Result<Vec<AssetRow>, EngineError> {
        assets.into_iter().map(|mut a| {
            let d=a.decimals.or(if a.is_native {Some(6)} else {None}).ok_or_else(|| EngineError::Validation(format!("missing decimals for Tezos asset {}",a.address)))?;
            a.balance=Self::normalize_raw_amount(&a.balance,d)?; Ok(a)
        }).collect()
    }
    fn validate_path(path:&str)->Result<DerivationPath,EngineError>{
        let p=DerivationPath::parse(path.to_string())?;
        if p.segments.len()<4 || p.segments[0].index!=44 || !p.segments[0].is_hardened || p.segments[1].index!=1729 || !p.segments[1].is_hardened || !p.segments[2].is_hardened || !p.segments[3].is_hardened {
            return Err(EngineError::Validation("Tezos derivation_path must start with m/44'/1729'/account'/change'".into()));
        } Ok(p)
    }
    fn hash160(data:&[u8])->[u8;20]{let d=Blake2b512::digest(data);let mut o=[0u8;20];o.copy_from_slice(&d[..20]);o}
    pub fn address_from_public_key_curve(public_key:&[u8],curve:TezosCurve)->Result<String,EngineError>{
        let (prefix,key)=match curve {
            TezosCurve::Ed25519=>{if public_key.len()!=32{return Err(EngineError::Validation("tz1 public key must be 32 bytes".into()));}(&[6,161,159][..],public_key)}
            TezosCurve::Secp256k1=>{let k=k256::PublicKey::from_sec1_bytes(public_key).map_err(|_|EngineError::Validation("invalid tz2 public key".into()))?;let encoded=k.to_encoded_point(true);return Ok(tezos_base58check(&[6,161,161],&Self::hash160(encoded.as_bytes())))}
            TezosCurve::P256=>{let k=p256::PublicKey::from_sec1_bytes(public_key).map_err(|_|EngineError::Validation("invalid tz3 public key".into()))?;let encoded=k.to_encoded_point(true);return Ok(tezos_base58check(&[6,161,164],&Self::hash160(encoded.as_bytes())))}
        }; Ok(tezos_base58check(prefix,&Self::hash160(key)))
    }
    pub fn derive_address_for_curve(seed:&[u8],path:&str,curve:TezosCurve)->Result<String,EngineError>{
        let p=Self::validate_path(path)?;
        match curve {
            TezosCurve::Ed25519=>{let m=derive_master_key_ed25519(seed)?;let c=derive_child_key_ed25519(&m,&p)?;let s=ed25519_dalek::SecretKey::from_bytes(&c.key).map_err(|_|EngineError::Crypto("invalid Tezos Ed25519 key".into()))?;Self::address_from_public_key_curve(&ed25519_dalek::PublicKey::from(&s).to_bytes(),curve)}
            TezosCurve::Secp256k1=>{let m=derive_master_key(seed)?;let c=derive_child_key(&m,&p)?;let s=k256::SecretKey::from_bytes((&c.key).into()).map_err(|_|EngineError::Crypto("invalid Tezos secp256k1 key".into()))?;Self::address_from_public_key_curve(s.public_key().to_encoded_point(true).as_bytes(),curve)}
            TezosCurve::P256=>{let m=derive_master_key(seed)?;let c=derive_child_key(&m,&p)?;let s=p256::SecretKey::from_slice(&c.key).map_err(|_|EngineError::Crypto("invalid Tezos P-256 key".into()))?;Self::address_from_public_key_curve(s.public_key().to_encoded_point(true).as_bytes(),curve)}
        }
    }
    pub fn sign_operation_for_curve(private_key:&[u8],tx_data:&[u8],curve:TezosCurve)->Result<Vec<u8>,EngineError>{
        let mut payload=Vec::with_capacity(tx_data.len()+1);payload.push(0x03);payload.extend_from_slice(tx_data);
        match curve {
            TezosCurve::Ed25519=>sign_ed25519_payload(private_key,&payload),
            TezosCurve::Secp256k1=>crate::crypto::signing::sign_secp256k1_payload(private_key,&payload),
            TezosCurve::P256=>{let k=p256::ecdsa::SigningKey::from_bytes(private_key.into()).map_err(|_|EngineError::Crypto("invalid Tezos P-256 key".into()))?;let s:p256::ecdsa::Signature=k.sign(&payload);Ok(s.to_bytes().to_vec())}
        }
    }
}

fn tezos_base58check(prefix: &[u8], payload: &[u8]) -> String {
    let mut data = Vec::with_capacity(prefix.len() + payload.len() + 4);
    data.extend_from_slice(prefix);
    data.extend_from_slice(payload);
    let first = sha2::Sha256::digest(&data);
    let second = sha2::Sha256::digest(&first);
    data.extend_from_slice(&second[..4]);
    bs58::encode(data).into_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TezosCurve {
    Ed25519,
    Secp256k1,
    P256,
}

impl WalletAdapter for TezosAdapter {
    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
        TezosAdapter::address_from_public_key_curve(public_key, TezosCurve::Ed25519)
    }

    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
        TezosAdapter::derive_address_for_curve(seed, path, TezosCurve::Ed25519)
    }

    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        TezosAdapter::sign_operation_for_curve(private_key, tx_data, TezosCurve::Ed25519)
    }

    fn address_from_private_key(private_key: &[u8]) -> Result<String, EngineError> {
        if private_key.len() != 32 {
            return Err(EngineError::Validation("Tezos Ed25519 private key must be 32 bytes".into()));
        }
        let secret = ed25519_dalek::SecretKey::from_bytes(private_key)
            .map_err(|_| EngineError::Crypto("invalid Tezos Ed25519 private key".into()))?;
        let public = ed25519_dalek::PublicKey::from(&secret).to_bytes();
        TezosAdapter::address_from_public_key_curve(&public, TezosCurve::Ed25519)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_default_tezos_path() {
        assert!(TezosAdapter::validate_path(TezosAdapter::DEFAULT_DERIVATION_PATH).is_ok());
        assert!(TezosAdapter::validate_path("m/44'/60'/0'/0/0").is_err());
    }

    #[test]
    fn supports_classic_tezos_curves() {
        let seed = [0x42u8; 64];
        for curve in [TezosCurve::Ed25519, TezosCurve::Secp256k1, TezosCurve::P256] {
            let address = TezosAdapter::derive_address_for_curve(
                &seed, TezosAdapter::DEFAULT_DERIVATION_PATH, curve
            ).unwrap();
            assert!(address.starts_with("tz1") || address.starts_with("tz2") || address.starts_with("tz3"));
        }
    }

    #[test]
    fn normalizes_mutez() {
        let assets = vec![AssetRow {
            address: "XTZ".into(),
            symbol: "XTZ".into(),
            decimals: Some(6),
            is_native: true,
            balance: "1234567".into(),
        }];
        let normalized = TezosAdapter::normalize_balances(assets).unwrap();
        assert_eq!(normalized[0].balance, "1.234567");
    }
}
