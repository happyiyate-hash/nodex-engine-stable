use ripemd::Ripemd160;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use sha2::{Digest, Sha256};

pub fn hash160(data: &[u8]) -> [u8; 20] {
    let sha = Sha256::digest(data);
    let ripemd = Ripemd160::digest(sha);
    ripemd.into()
}

pub fn base58check_address(version: u8, payload: &[u8]) -> String {
    let mut data = Vec::with_capacity(1 + payload.len() + 4);
    data.push(version);
    data.extend_from_slice(payload);
    let first = Sha256::digest(&data);
    let second = Sha256::digest(first);
    data.extend_from_slice(&second[..4]);
    bs58::encode(data).into_string()
}

pub fn p2pkh_address(public_key: &[u8], version: u8) -> Result<String, String> {
    if public_key.len() != 33 {
        return Err("compressed public key must be 33 bytes".into());
    }
    let h = hash160(public_key);
    Ok(base58check_address(version, &h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_base58check_address() {
        let address = p2pkh_address(&[0x02; 33], 0).expect("address");
        assert!(!address.is_empty());
    }
}

pub fn p2pkh_from_secret(secret: &[u8], version: u8) -> Result<String, String> {
    let key = k256::SecretKey::from_bytes(secret.into())
        .map_err(|_| "invalid secp256k1 private key".to_string())?;
    let public = key.public_key().to_encoded_point(true);
    p2pkh_address(public.as_bytes(), version)
}


/// Encode a witness program using BIP173 Bech32.
pub fn segwit_address(hrp: &str, witness_version: u8, program: &[u8]) -> Result<String, String> {
    if witness_version > 16 { return Err("invalid witness version".into()); }
    if program.len() < 2 || program.len() > 40 { return Err("invalid witness program length".into()); }
    if witness_version == 0 && program.len() != 20 && program.len() != 32 {
        return Err("witness v0 program must be 20 or 32 bytes".into());
    }
    let data = std::iter::once(witness_version)
        .chain(convert_bits(program, 8, 5, true)?.into_iter())
        .collect::<Vec<_>>();
    let mut values = Vec::with_capacity(hrp.len() + 1 + data.len() + 6);
    for b in hrp.as_bytes() {
        values.push((*b as u8) >> 5);
    }
    values.push(0);
    for b in hrp.as_bytes() {
        values.push((*b as u8) & 31);
    }
    values.extend_from_slice(&data);
    let polymod = bech32_polymod(&values.iter().map(|v| *v as u8).chain(std::iter::repeat(0).take(6)).collect::<Vec<_>>()) ^ 1;
    let mut out = String::from(hrp);
    out.push('1');
    const CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    for v in data {
        out.push(CHARSET[v as usize] as char);
    }
    for i in 0..6 {
        out.push(CHARSET[((polymod >> (5 * (5 - i))) & 31) as usize] as char);
    }
    Ok(out)
}

fn bech32_polymod(values: &[u8]) -> u32 {
    const GEN: [u32; 5] = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
    let mut chk = 1u32;
    for value in values {
        let top = chk >> 25;
        chk = (chk & 0x1ffffff) << 5 ^ (*value as u32);
        for (i, g) in GEN.iter().enumerate() {
            if ((top >> i) & 1) != 0 { chk ^= g; }
        }
    }
    chk
}

fn convert_bits(data: &[u8], from: u32, to: u32, pad: bool) -> Result<Vec<u8>, String> {
    let mut acc = 0u32;
    let mut bits = 0u32;
    let maxv = (1u32 << to) - 1;
    let max_acc = (1u32 << (from + to - 1)) - 1;
    let mut ret = Vec::new();
    for value in data {
        let v = *value as u32;
        if (v >> from) != 0 { return Err("invalid convert-bits value".into()); }
        acc = ((acc << from) | v) & max_acc;
        bits += from;
        while bits >= to {
            bits -= to;
            ret.push(((acc >> bits) & maxv) as u8);
        }
    }
    if pad {
        if bits > 0 { ret.push(((acc << (to - bits)) & maxv) as u8); }
    } else if bits >= from || ((acc << (to - bits)) & maxv) != 0 {
        return Err("invalid convert-bits padding".into());
    }
    Ok(ret)
}

pub fn p2wpkh_address(public_key: &[u8], hrp: &str) -> Result<String, String> {
    if public_key.len() != 33 { return Err("compressed public key must be 33 bytes".into()); }
    let h = hash160(public_key);
    segwit_address(hrp, 0, &h)
}

pub fn p2sh_p2wpkh_address(public_key: &[u8], version: u8) -> Result<String, String> {
    if public_key.len() != 33 { return Err("compressed public key must be 33 bytes".into()); }
    let h = hash160(public_key);
    let mut redeem = Vec::with_capacity(22);
    redeem.push(0x00);
    redeem.push(0x14);
    redeem.extend_from_slice(&h);
    let script_hash = hash160(&redeem);
    Ok(base58check_address(version, &script_hash))
}
