use crate::adapters::adapter_trait::WalletAdapter;
use crate::common::errors::EngineError;
use blake2::{Blake2b512, Digest};
use crate::crypto::bip32::{derive_master_key_ed25519, derive_child_key_ed25519};
use crate::crypto::derivation::DerivationPath;
use crate::crypto::signing::sign_ed25519_payload;
use crate::types::chain::ChainRequest;

/// Cardano adapter for the Cardano family.
///
/// Cardano is a standalone blockchain family. ADA and Cardano native assets
/// share the same ledger/UTxO model and therefore do not require separate
/// adapters.
pub struct CardanoAdapter;

impl CardanoAdapter {
    fn network_id(network: Option<&str>) -> u8 {
        match network.unwrap_or("mainnet").trim().to_ascii_lowercase().as_str() {
            "mainnet" | "main" => 1,
            // Shelley address network tag 0 is used for test networks.
            _ => 0,
        }
    }

    /// Converts a payment derivation path into the corresponding payment and
    /// staking paths for a Shelley address.
    ///
    /// Expected CIP-1852 structure:
    /// m/1852'/1815'/account'/role/index
    fn payment_and_stake_paths(path: &str) -> Result<(String, String), EngineError> {
        let mut parts: Vec<&str> = path.trim().split('/').collect();
        if parts.len() != 6 || parts[0] != "m" {
            return Err(EngineError::Validation(
                "Cardano derivation_path must be m/1852'/1815'/account'/role/index".into(),
            ));
        }

        if parts[1] != "1852'" || parts[2] != "1815'" || parts[3].is_empty()
            || parts[4].is_empty() || parts[5].is_empty()
        {
            return Err(EngineError::Validation(
                "Cardano derivation_path must use CIP-1852: m/1852'/1815'/account'/role/index".into(),
            ));
        }

        // The caller identifies the wallet account/index through the supplied
        // path; Cardano payment addresses use role 0 and staking uses role 2.
        parts[4] = "0'";
        let payment = parts.join("/");
        parts[4] = "2'";
        let stake = parts.join("/");
        Ok((payment, stake))
    }

    fn derive_public_key(seed: &[u8], path: &str) -> Result<[u8; 32], EngineError> {
        let master = derive_master_key_ed25519(seed)?;
        let parsed = DerivationPath::parse(path.to_string())?;
        let child = derive_child_key_ed25519(&master, &parsed)?;
        let secret = ed25519_dalek::SecretKey::from_bytes(&child.key)
            .map_err(|_| EngineError::Crypto("invalid Cardano Ed25519 key".into()))?;
        Ok(ed25519_dalek::PublicKey::from(&secret).to_bytes())
    }

    fn key_hash(public_key: &[u8; 32]) -> [u8; 28] {
        let digest = Blake2b512::digest(public_key);
        let mut out = [0u8; 28];
        out.copy_from_slice(&digest[..28]);
        out
    }

    fn base_address(payment_key: &[u8; 32], stake_key: &[u8; 32], network_id: u8) -> String {
        // Shelley base address: header type 0000 = key-hash payment +
        // key-hash stake credential.
        let payment_hash = Self::key_hash(payment_key);
        let stake_hash = Self::key_hash(stake_key);
        let mut bytes = Vec::with_capacity(57);
        bytes.push(network_id & 0x0f);
        bytes.extend_from_slice(&payment_hash);
        bytes.extend_from_slice(&stake_hash);
        bech32_cardano(&bytes, if network_id == 1 { "addr" } else { "addr_test" })
    }
}

impl WalletAdapter for CardanoAdapter {
    fn derive_addresses(
        seed: &[u8],
        chains: &[ChainRequest],
    ) -> Result<std::collections::HashMap<String, String>, EngineError> {
        let mut out = std::collections::HashMap::with_capacity(chains.len());

        for chain in chains {
            let path = chain.derivation_path.as_deref().ok_or_else(|| {
                EngineError::Validation(format!(
                    "derivation_path is required for Cardano blockchain {}",
                    chain.name
                ))
            })?;

            let (payment_path, stake_path) = Self::payment_and_stake_paths(path)?;
            let payment_key = Self::derive_public_key(seed, &payment_path)?;
            let stake_key = Self::derive_public_key(seed, &stake_path)?;
            let address = Self::base_address(
                &payment_key,
                &stake_key,
                Self::network_id(chain.network.as_deref()),
            );
            out.insert(chain.name.clone(), address);
        }

        Ok(out)
    }

    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
        if public_key.len() != 32 {
            return Err(EngineError::Validation(
                "Cardano Ed25519 public key must be 32 bytes".into(),
            ));
        }

        // This trait method has no ChainRequest/network argument, so retain
        // mainnet as the only unqualified representation. Normal wallet
        // derivation uses derive_addresses(), which receives chain.network.
        let mut key = [0u8; 32];
        key.copy_from_slice(public_key);
        let hash = Self::key_hash(&key);

        // Without a stake key, the generic public-key conversion can only
        // produce an enterprise address. Full wallet derivation uses base
        // addresses through derive_addresses().
        let mut bytes = Vec::with_capacity(29);
        bytes.push(0x61); // enterprise key-hash address, mainnet
        bytes.extend_from_slice(&hash);
        Ok(bech32_cardano(&bytes, "addr"))
    }

    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
        let (payment_path, stake_path) = Self::payment_and_stake_paths(path)?;
        let payment_key = Self::derive_public_key(seed, &payment_path)?;
        let stake_key = Self::derive_public_key(seed, &stake_path)?;
        Ok(Self::base_address(&payment_key, &stake_key, 1))
    }

    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        if private_key.len() != 32 {
            return Err(EngineError::Validation(
                "Cardano signing key seed must be 32 bytes".into(),
            ));
        }

        // Cardano transaction witnesses sign the transaction-body hash,
        // not the raw transaction bytes.
        let digest = Blake2b512::digest(tx_data);
        sign_ed25519_payload(private_key, &digest[..32])
    }
}

fn bech32_cardano(bytes: &[u8], hrp: &str) -> String {
    const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

    fn convert(data: &[u8]) -> Vec<u8> {
        let mut acc = 0u32;
        let mut bits = 0u8;
        let mut out = Vec::new();

        for &b in data {
            acc = (acc << 8) | b as u32;
            bits += 8;
            while bits >= 5 {
                bits -= 5;
                out.push(((acc >> bits) & 31) as u8);
            }
        }

        if bits > 0 {
            out.push(((acc << (5 - bits)) & 31) as u8);
        }
        out
    }

    fn polymod(values: &[u8]) -> u32 {
        const GEN: [u32; 5] = [
            0x3b6a57b2,
            0x26508e6d,
            0x1ea119fa,
            0x3d4233dd,
            0x2a1462b3,
        ];
        let mut chk = 1u32;
        for &v in values {
            let top = chk >> 25;
            chk = ((chk & 0x1ffffff) << 5) ^ v as u32;
            for i in 0..5 {
                if ((top >> i) & 1) != 0 {
                    chk ^= GEN[i];
                }
            }
        }
        chk
    }

    let data = convert(bytes);
    let mut values = Vec::new();

    for &b in hrp.as_bytes() {
        values.push(b >> 5);
    }
    values.push(0);
    for &b in hrp.as_bytes() {
        values.push(b & 31);
    }
    values.extend_from_slice(&data);
    values.extend_from_slice(&[0; 6]);

    let p = polymod(&values) ^ 1;
    let mut checksum = [0u8; 6];
    for i in 0..6 {
        checksum[i] = ((p >> (5 * (5 - i))) & 31) as u8;
    }

    let mut out = String::with_capacity(hrp.len() + 1 + data.len() + 6);
    out.push_str(hrp);
    out.push('1');
    for d in data.into_iter().chain(checksum) {
        out.push(CHARSET[d as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cardano_cip1852_derivation_works() {
        let chains = vec![ChainRequest {
            name: "Cardano".into(),
            chain_id: "mainnet".into(),
            symbol: Some("ADA".into()),
            network: Some("mainnet".into()),
            chain_type: "cardano".into(),
            derivation_path: Some("m/1852'/1815'/0'/0'/0'".into()),
            address_type: None,
            p2pkh_version: None,
            p2sh_version: None,
            bech32_hrp: None,
            rpc_provider: None,
            rpc_url: None,
        }];

        let result = CardanoAdapter::derive_addresses(&[0u8; 32], &chains);
        assert!(result.is_ok());
        assert!(result.unwrap().get("Cardano").unwrap().starts_with("addr1"));
    }

    #[test]
    fn cardano_requires_derivation_path() {
        let chain = ChainRequest::new("Cardano", "cardano");
        let result = CardanoAdapter::derive_addresses(&[0u8; 32], &[chain]);
        assert!(result.is_err());
    }

    #[test]
    fn cardano_testnet_uses_addr_test() {
        let mut chain = ChainRequest::new("Cardano", "cardano");
        chain.chain_id = "preprod".into();
        chain.network = Some("preprod".into());
        chain.derivation_path = Some("m/1852'/1815'/0'/0'/0'".into());
        let result = CardanoAdapter::derive_addresses(&[0u8; 32], &[chain]).unwrap();
        assert!(result.get("Cardano").unwrap().starts_with("addr_test1"));
    }
}
