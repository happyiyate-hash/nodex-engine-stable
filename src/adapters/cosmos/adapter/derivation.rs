use k256::elliptic_curve::sec1::ToEncodedPoint;

use crate::common::errors::EngineError;
use crate::crypto::bip32::{derive_child_key, derive_master_key};
use crate::crypto::chain_addresses::cosmos_secp256k1_address;
use crate::crypto::derivation::DerivationPath;
use crate::types::chain::ChainRequest;

/// Resolve the address prefix from request metadata first. The chain-type
/// fallback is only for older callers that do not yet send bech32_hrp.
pub fn prefix_for_chain(chain_type: &str, requested_hrp: Option<&str>) -> Result<String, EngineError> {
    if let Some(hrp) = requested_hrp {
        let hrp = hrp.trim();
        if hrp.is_empty() {
            return Err(EngineError::Validation("bech32_hrp cannot be empty for Cosmos-family address derivation".into()));
        }
        if !hrp.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()) {
            return Err(EngineError::Validation(format!("invalid Cosmos Bech32 prefix: {}", hrp)));
        }
        return Ok(hrp.to_owned());
    }

    let normalized = chain_type.trim().to_ascii_lowercase();
    let hrp = match normalized.as_str() {
        "agoric" => "agoric",
        "akash" => "akash",
        "axelar" => "axelar",
        "band" | "bandchain" => "band",
        "celestia" | "tia" => "celestia",
        "comdex" => "comdex",
        "coreum" => "core",
        "cosmos" | "cosmoshub" | "cosmos-hub" => "cosmos",
        "crescent" => "cre",
        "cryptoorg" | "crypto.org" | "cro" => "cro",
        "fetch" | "fetchai" => "fetch",
        "iris" | "irisnet" => "iaa",
        "juno" => "juno",
        "kava" => "kava",
        "kujira" => "kujira",
        "mars" | "mars-hub" => "mars",
        "injective" | "nativeinjective" | "inj" => "inj",
        "canto" | "nativecanto" => "canto",
        "neutron" => "neutron",
        "noble" => "noble",
        "osmosis" | "osmo" => "osmo",
        "persistence" => "persistence",
        "quasar" => "quasar",
        "secret" | "scrt" => "secret",
        "sei" => "sei",
        "sommelier" => "somm",
        "stargaze" => "stars",
        "stride" => "stride",
        "terra" | "terraclassic" | "terra-classic" => "terra",
        "umee" => "umee",
        _ => return Err(EngineError::Validation(format!(
            "bech32_hrp is required for Cosmos-family chain {}", chain_type
        ))),
    };
    Ok(hrp.to_owned())
}

pub fn derive_address(seed: &[u8], path: &str, hrp: &str) -> Result<String, EngineError> {
    let master = derive_master_key(seed)?;
    let parsed = DerivationPath::parse(path.to_owned())?;
    let child = derive_child_key(&master, &parsed)?;
    let secret = k256::SecretKey::from_bytes((&child.key).into())
        .map_err(|_| EngineError::Crypto("invalid Cosmos secp256k1 private key".into()))?;
    let public = secret.public_key().to_encoded_point(true);
    cosmos_secp256k1_address(public.as_bytes(), hrp)
}

pub fn derive_addresses(
    seed: &[u8],
    chains: &[ChainRequest],
) -> Result<std::collections::HashMap<String, String>, EngineError> {
    let mut out = std::collections::HashMap::with_capacity(chains.len());
    for chain in chains {
        let path = chain.derivation_path.as_deref().filter(|p| !p.trim().is_empty())
            .ok_or_else(|| EngineError::Validation(format!(
                "derivation_path is required for Cosmos-family chain {}", chain.name
            )))?;
        let hrp = prefix_for_chain(&chain.chain_type, chain.bech32_hrp.as_deref())?;
        let address = derive_address(seed, path, &hrp)?;
        if out.insert(chain.name.clone(), address).is_some() {
            return Err(EngineError::Validation(format!("duplicate blockchain target: {}", chain.name)));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_hrp_takes_precedence_over_generic_chain_type() {
        assert_eq!(prefix_for_chain("cosmos", Some("akash")).unwrap(), "akash");
        assert_eq!(prefix_for_chain("cosmos", Some("celestia")).unwrap(), "celestia");
    }

    #[test]
    fn missing_unknown_hrp_is_not_silently_cosmos() {
        assert!(prefix_for_chain("unknown-chain", None).is_err());
    }
}
