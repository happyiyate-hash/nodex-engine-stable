use crate::common::errors::EngineError;
use crate::crypto::bip32::{derive_child_key, derive_child_key_ed25519, derive_master_key, derive_master_key_ed25519};
use crate::crypto::derivation::DerivationPath;
use crate::types::chain::ChainFamily;
use ed25519_dalek::SecretKey as Ed25519SecretKey;
use k256::{PublicKey as K256PublicKey, SecretKey as K256SecretKey};
use k256::elliptic_curve::sec1::ToEncodedPoint;

pub struct KeyDerivationService;

impl KeyDerivationService {
    pub fn derive_public_key(seed: &[u8], chain_type: &str) -> Result<Vec<u8>, EngineError> {
        let path = crate::adapters::registry::default_derivation_path(chain_type)
            .ok_or_else(|| EngineError::Unsupported(format!("unsupported chain_type: {}", chain_type)))?;
        let family = ChainFamily::from(chain_type);
        let parsed = DerivationPath::parse(path.to_string())?;
        let child = match family {
            ChainFamily::Solana | ChainFamily::Substrate => {
                let master = derive_master_key_ed25519(seed)?;
                derive_child_key_ed25519(&master, &parsed)?
            }
            _ => {
                let master = derive_master_key(seed)?;
                derive_child_key(&master, &parsed)?
            }
        };
        match family {
            ChainFamily::Solana | ChainFamily::Substrate | ChainFamily::Near | ChainFamily::Aptos | ChainFamily::Sui => {
                let secret = Ed25519SecretKey::from_bytes(&child.key).map_err(|_| EngineError::Crypto("invalid ed25519 key".into()))?;
                Ok(ed25519_dalek::PublicKey::from(&secret).to_bytes().to_vec())
            }
            ChainFamily::Evm => {
                let secret = K256SecretKey::from_bytes((&child.key).into()).map_err(|_| EngineError::Crypto("invalid secp256k1 key".into()))?;
                Ok(K256PublicKey::from_secret_scalar(&secret.to_nonzero_scalar()).to_encoded_point(false).as_bytes().to_vec())
            }
            _ => {
                let secret = K256SecretKey::from_bytes((&child.key).into()).map_err(|_| EngineError::Crypto("invalid secp256k1 key".into()))?;
                Ok(K256PublicKey::from_secret_scalar(&secret.to_nonzero_scalar()).to_encoded_point(true).as_bytes().to_vec())
            }
        }
    }

    pub fn derive_private_key_at_path(seed: &[u8], chain_type: &str, path: &str) -> Result<Vec<u8>, EngineError> {
        let family = ChainFamily::from(chain_type);
        let parsed = DerivationPath::parse(path.to_string())?;
        let child = match family {
            ChainFamily::Solana | ChainFamily::Substrate | ChainFamily::Near | ChainFamily::Aptos | ChainFamily::Sui => {
                let master = derive_master_key_ed25519(seed)?;
                derive_child_key_ed25519(&master, &parsed)?
            }
            _ => {
                let master = derive_master_key(seed)?;
                derive_child_key(&master, &parsed)?
            }
        };
        Ok(child.key.to_vec())
    }

    pub fn derive_private_key(seed: &[u8], chain_type: &str) -> Result<Vec<u8>, EngineError> {
        let path = crate::adapters::registry::default_derivation_path(chain_type)
            .ok_or_else(|| EngineError::Unsupported(format!("unsupported chain_type: {}", chain_type)))?;
        let family = ChainFamily::from(chain_type);
        let parsed = DerivationPath::parse(path.to_string())?;
        let child = match family {
            ChainFamily::Solana | ChainFamily::Substrate | ChainFamily::Near | ChainFamily::Aptos | ChainFamily::Sui => {
                let master = derive_master_key_ed25519(seed)?;
                derive_child_key_ed25519(&master, &parsed)?
            }
            _ => {
                let master = derive_master_key(seed)?;
                derive_child_key(&master, &parsed)?
            }
        };
        Ok(child.key.to_vec())
    }
}
