use crate::common::errors::EngineError;
use crate::crypto::bip32::{derive_child_key,derive_child_key_ed25519,derive_master_key,derive_master_key_ed25519};
use ed25519_dalek::SecretKey as Ed25519SecretKey;
use k256::{PublicKey as K256PublicKey,SecretKey as K256SecretKey};
use k256::elliptic_curve::sec1::ToEncodedPoint;

#[derive(Debug,Clone,PartialEq,Eq)] pub struct DerivationSegment{pub index:u32,pub is_hardened:bool}
#[derive(Debug,Clone,PartialEq,Eq)] pub struct DerivationPath{pub segments:Vec<DerivationSegment>}
impl DerivationPath{pub fn parse(path:String)->Result<Self,EngineError>{let parts:Vec<&str>=path.split('/').collect();if parts.first()!=Some(&"m")||parts.len()<2{return Err(EngineError::Validation(format!("invalid derivation path: {}",path)));}let mut segments=Vec::new();for raw in parts.iter().skip(1){let hardened=raw.ends_with('\'');let n=raw.trim_end_matches('\'').parse::<u32>().map_err(|_|EngineError::Validation(format!("invalid derivation path segment: {}",raw)))?;segments.push(DerivationSegment{index:n,is_hardened:hardened});}Ok(Self{segments})}}

pub struct KeyDerivationService;
impl KeyDerivationService {
 pub fn derive_public_key(seed:&[u8],chain_type:&str)->Result<Vec<u8>,EngineError>{let(path,ed)=Self::path_and_curve(chain_type)?;let p=DerivationPath::parse(path.to_string())?;let child=if ed{let m=derive_master_key_ed25519(seed)?;derive_child_key_ed25519(&m,&p)?}else{let m=derive_master_key(seed)?;derive_child_key(&m,&p)?};if ed{let s=Ed25519SecretKey::from_bytes(&child.key).map_err(|_|EngineError::Crypto("invalid ed25519 derived key".into()))?;Ok(ed25519_dalek::PublicKey::from(&s).to_bytes().to_vec())}else{let s=K256SecretKey::from_bytes((&child.key).into()).map_err(|_|EngineError::Crypto("invalid secp256k1 derived key".into()))?;let public=K256PublicKey::from_secret_scalar(&s.to_nonzero_scalar());Ok(public.to_encoded_point(chain_type!="evm").as_bytes().to_vec())}}
 pub fn derive_private_key(seed:&[u8],chain_type:&str)->Result<Vec<u8>,EngineError>{let(path,ed)=Self::path_and_curve(chain_type)?;let p=DerivationPath::parse(path.to_string())?;let c=if ed{let m=derive_master_key_ed25519(seed)?;derive_child_key_ed25519(&m,&p)?}else{let m=derive_master_key(seed)?;derive_child_key(&m,&p)?};Ok(c.key.to_vec())}
 fn path_and_curve(chain_type:&str)->Result<(&'static str,bool),EngineError>{match chain_type{"evm"=>Ok(("m/44'/60'/0'/0/0",false)),"btc"|"bitcoin"|"bitcoin-cash"|"litecoin"|"doge"=>Ok(("m/44'/0'/0'/0/0",false)),"solana"=>Ok(("m/44'/501'/0'/0'",true)),"cosmos"|"osmosis"|"secret"|"injective"|"celestia"=>Ok(("m/44'/118'/0'/0/0",false)),"polkadot"|"kusama"|"substrate"=>Ok(("m/44'/354'/0'/0'/0'",true)),"xrp"=>Ok(("m/44'/144'/0'/0/0",false)),"algorand"=>Ok(("m/44'/283'/0'/0/0",false)),"hedera"=>Ok(("m/44'/3030'/0'/0/0",false)),"bep2"=>Ok(("m/44'/714'/0'/0/0",false)),"near"=>Ok(("m/44'/397'/0'/0/0",false)),"aptos"=>Ok(("m/44'/637'/0'/0/0",false)),"sui"=>Ok(("m/44'/784'/0'/0/0",false)),"tezos"=>Ok(("m/44'/1729'/0'/0/0",false)),"tron"=>Ok(("m/44'/195'/0'/0/0",false)),"starknet"=>Ok(("m/44'/9004'/0'/0/0",false)),_=>Err(EngineError::Unsupported(format!("unsupported chain type for key derivation: {}",chain_type)))}}
}
