use crate::common::errors::EngineError;
use blake2::{Blake2b512, Digest as BlakeDigest};
use ed25519_dalek::{PublicKey, SecretKey};

const CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
fn polymod(values: &[u8]) -> u32 {
    let mut chk=1u32; const G:[u32;5]=[0x3b6a57b2,0x26508e6d,0x1ea119fa,0x3d4233dd,0x2a1462b3];
    for v in values { let top=chk>>25; chk=(chk&0x1ffffff)<<5 ^ *v as u32; for (i,g) in G.iter().enumerate(){if ((top>>i)&1)!=0{chk^=g;}} } chk
}
fn expand(hrp:&str)->Vec<u8>{let mut v=Vec::new();v.extend(hrp.bytes().map(|b|b>>5));v.push(0);v.extend(hrp.bytes().map(|b|b&31));v}
fn bits(data:&[u8])->Result<Vec<u8>,EngineError>{let mut acc=0u32;let mut n=0u8;let mut out=Vec::new();for b in data{acc=(acc<<8)|*b as u32;n+=8;while n>=5{n-=5;out.push(((acc>>n)&31)as u8);}}if n>0{out.push(((acc<<(5-n))&31)as u8);}Ok(out)}
pub fn bech32_encode(hrp:&str,bytes:&[u8])->Result<String,EngineError>{let d=bits(bytes)?;let mut p=expand(hrp);p.extend(&d);p.extend([0;6]);let pm=polymod(&p)^1;let mut s=format!("{}1",hrp);for x in d{s.push(CHARSET[x as usize]as char);}for i in 0..6{s.push(CHARSET[((pm>>(5*(5-i)))&31)as usize]as char);}Ok(s)}
pub fn cosmos_secp256k1_address(public_key:&[u8],hrp:&str)->Result<String,EngineError>{use sha2::Digest;let h=sha2::Sha256::digest(public_key);let r=ripemd::Ripemd160::digest(h);bech32_encode(hrp,&r)}
pub fn aptos_address_from_public_key(public_key:&[u8])->Result<String,EngineError>{if public_key.len()!=32{return Err(EngineError::Validation("Aptos public key must be 32 bytes".into()));}use sha3::Digest;let mut h=sha3::Sha3_256::new();h.update(public_key);h.update([0]);Ok(format!("0x{}",hex::encode(h.finalize())))}
pub fn aptos_address_from_private_key(k:&[u8])->Result<String,EngineError>{let s=SecretKey::from_bytes(k).map_err(|_|EngineError::Crypto("invalid Ed25519 private key".into()))?;aptos_address_from_public_key(PublicKey::from(&s).as_bytes())}
pub fn sui_address_from_public_key(public_key:&[u8])->Result<String,EngineError>{if public_key.len()!=32{return Err(EngineError::Validation("Sui public key must be 32 bytes".into()));}let mut h=Blake2b512::new();h.update([0]);h.update(public_key);let d=h.finalize();Ok(format!("0x{}",hex::encode(&d[..32])))}
pub fn sui_address_from_private_key(k:&[u8])->Result<String,EngineError>{let s=SecretKey::from_bytes(k).map_err(|_|EngineError::Crypto("invalid Ed25519 private key".into()))?;sui_address_from_public_key(PublicKey::from(&s).as_bytes())}
