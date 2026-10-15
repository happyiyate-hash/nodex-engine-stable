use crate::adapters::adapter_trait::WalletAdapter;
use crate::common::errors::EngineError;
use crate::crypto::bip32::{derive_child_key, derive_master_key};
use crate::crypto::derivation::DerivationPath;
use rand::RngCore;
use starknet_crypto::{get_public_key, pedersen_hash, sign, Felt};

/// Starknet account adapter.
///
/// Starknet accounts are smart contracts. The default wallet profile uses the
/// OpenZeppelin AccountUpgradeable class and computes its counterfactual address
/// from the Stark public key, class hash, constructor calldata and salt.
///
/// RPC, funding and DeployAccount submission remain outside the adapter.
pub struct StarknetAdapter;

#[derive(Clone, Debug)]
pub struct AssetRow {
    pub address: String,
    pub symbol: String,
    pub decimals: Option<u32>,
    pub is_native: bool,
    pub balance: String,
}

/// Current OpenZeppelin AccountUpgradeable Sierra class hash documented for the
/// 2.x preset set. This value is a deployment parameter, not a cryptographic
/// property of Starknet itself.
const DEFAULT_OZ_ACCOUNT_CLASS_HASH: &str =
    "0x079a9a12fdfa0481e8d8d46599b90226cd7247b2667358bb00636dd864002314";

const STARKNET_CONTRACT_ADDRESS_PREFIX: &str = "STARKNET_CONTRACT_ADDRESS";
const L2_ADDRESS_UPPER_BOUND: &str =
    "0x07ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff00";

impl StarknetAdapter {
    /// The wallet layer supplies the derivation path. We do not silently
    /// replace it. A common Starknet HD path is m/44'/9004'/0'/0/0.
    fn validate_path(path: &str) -> Result<(), EngineError> {
        let parsed = DerivationPath::parse(path.to_string())?;
        if parsed.segments.is_empty() {
            return Err(EngineError::Validation(
                "Starknet derivation_path cannot be empty".into(),
            ));
        }
        Ok(())
    }

    pub fn public_key_from_private_key(private_key: &[u8]) -> Result<String, EngineError> {
        let bytes: [u8; 32] = private_key.try_into()
            .map_err(|_| EngineError::Validation("Starknet private key must be 32 bytes".into()))?;
        let key = Felt::from_bytes_be(&bytes);
        Ok(format!("0x{:x}", get_public_key(&key)))
    }

    /// Starknet's contract address formula:
    /// pedersen(prefix, deployer_address, salt, class_hash,
    ///          pedersen(constructor_calldata))
    /// followed by normalization to the Starknet L2 address range.
    pub fn calculate_contract_address(
        salt: &str,
        class_hash: &str,
        constructor_calldata: &[String],
        deployer_address: &str,
    ) -> Result<String, EngineError> {
        let salt = parse_felt(salt, "salt")?;
        let class_hash = parse_felt(class_hash, "class_hash")?;
        let deployer = parse_felt(deployer_address, "deployer_address")?;

        let calldata = constructor_calldata.iter()
            .map(|v| parse_felt(v, "constructor_calldata"))
            .collect::<Result<Vec<_>, _>>()?;

        let calldata_hash = compute_hash_on_elements(&calldata);
        let prefix = Felt::parse_cairo_short_string(STARKNET_CONTRACT_ADDRESS_PREFIX)
            .map_err(|_| EngineError::Validation("invalid Starknet address prefix".into()))?;

        let raw = compute_hash_on_elements(&[
            prefix,
            deployer,
            salt,
            class_hash,
            calldata_hash,
        ]);

        let upper_bound = parse_felt(L2_ADDRESS_UPPER_BOUND, "address upper bound")?;
        let normalized = if raw >= upper_bound { raw - upper_bound } else { raw };
        Ok(format!("0x{:x}", normalized))
    }

    /// Default OpenZeppelin account profile:
    /// constructor(public_key), salt = public_key, deployer = 0.
    pub fn address_from_private_key(private_key: &[u8]) -> Result<String, EngineError> {
        let public_key = Self::public_key_from_private_key(private_key)?;
        Self::address_from_public_key_with_profile(
            &public_key,
            DEFAULT_OZ_ACCOUNT_CLASS_HASH,
        )
    }

    pub fn address_from_public_key_with_profile(
        public_key: &str,
        class_hash: &str,
    ) -> Result<String, EngineError> {
        Self::calculate_contract_address(
            public_key,
            class_hash,
            &[public_key.to_string()],
            "0x0",
        )
    }

    /// Normalize raw provider amounts using the token's supplied decimals.
    /// Fetching remains in the provider layer.
    pub fn normalize_balances(assets: Vec<AssetRow>) -> Result<Vec<AssetRow>, EngineError> {
        assets.into_iter()
            .map(|mut asset| {
                let decimals = asset.decimals.ok_or_else(|| EngineError::Validation(
                    format!("missing decimals for Starknet asset {}", asset.address)
                ))?;
                asset.balance = normalize_raw_amount(&asset.balance, decimals)?;
                Ok(asset)
            })
            .collect()
    }

    /// Sign a 32-byte Starknet transaction/message hash.
    pub fn sign_message_hash(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        let key_bytes: [u8; 32] = private_key.try_into()
            .map_err(|_| EngineError::Validation("Starknet private key must be 32 bytes".into()))?;
        let hash_bytes: [u8; 32] = tx_data.try_into()
            .map_err(|_| EngineError::Validation("Starknet signing input must be a 32-byte message hash".into()))?;
        let private = Felt::from_bytes_be(&key_bytes);
        let message = Felt::from_bytes_be(&hash_bytes);

        for _ in 0..32 {
            let mut k_bytes = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut k_bytes);
            let k = Felt::from_bytes_be(&k_bytes);
            if let Ok(signature) = sign(&private, &message, &k) {
                let mut out = Vec::with_capacity(64);
                out.extend_from_slice(&signature.r.to_bytes_be());
                out.extend_from_slice(&signature.s.to_bytes_be());
                return Ok(out);
            }
        }
        Err(EngineError::Crypto("failed to generate a valid Starknet ECDSA nonce".into()))
    }
}

fn parse_felt(value: &str, field: &str) -> Result<Felt, EngineError> {
    Felt::from_hex(value).map_err(|_| EngineError::Validation(
        format!("invalid Starknet {} felt", field)
    ))
}

fn compute_hash_on_elements(elements: &[Felt]) -> Felt {
    let mut hash = Felt::ZERO;
    for element in elements {
        hash = pedersen_hash(&hash, element);
    }
    hash = pedersen_hash(&hash, &Felt::from(elements.len() as u64));
    hash
}

fn normalize_raw_amount(raw: &str, decimals: u32) -> Result<String, EngineError> {
    if raw.trim().is_empty() || raw.starts_with('-') || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err(EngineError::Validation("invalid Starknet base amount".into()));
    }
    let value = raw.trim().parse::<u128>()
        .map_err(|_| EngineError::Validation("Starknet base amount exceeds u128".into()))?;
    if decimals == 0 { return Ok(value.to_string()); }
    let scale = 10u128.checked_pow(decimals)
        .ok_or_else(|| EngineError::Validation("Starknet decimals exceed supported range".into()))?;
    let whole = value / scale;
    let fraction = value % scale;
    if fraction == 0 { return Ok(whole.to_string()); }
    let mut f = format!("{:0width$}", fraction, width = decimals as usize);
    while f.ends_with('0') { f.pop(); }
    Ok(format!("{}.{}", whole, f))
}

impl WalletAdapter for StarknetAdapter {
    fn pub_key_to_address(public_key: &[u8]) -> Result<String, EngineError> {
        if public_key.len() != 32 {
            return Err(EngineError::Validation(
                "Starknet public key must be 32 bytes".into(),
            ));
        }
        let key = Felt::from_bytes_be(
            public_key.try_into().map_err(|_| EngineError::Validation("invalid public key".into()))?
        );
        Self::address_from_public_key_with_profile(
            &format!("0x{:x}", key),
            DEFAULT_OZ_ACCOUNT_CLASS_HASH,
        )
    }

    fn derive_address_from_path(seed: &[u8], path: &str) -> Result<String, EngineError> {
        Self::validate_path(path)?;
        let master = derive_master_key(seed)?;
        let parsed = DerivationPath::parse(path.to_string())?;
        let child = derive_child_key(&master, &parsed)?;
        Self::address_from_private_key(&child.key)
    }

    fn sign_transaction(private_key: &[u8], tx_data: &[u8]) -> Result<Vec<u8>, EngineError> {
        Self::sign_message_hash(private_key, tx_data)
    }

    fn address_from_private_key(private_key: &[u8]) -> Result<String, EngineError> {
        Self::address_from_private_key(private_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_a_derivation_path() {
        assert!(StarknetAdapter::validate_path("m/44'/9004'/0'/0/0").is_ok());
        assert!(StarknetAdapter::validate_path("").is_err());
    }

    #[test]
    fn normalizes_strk_amount() {
        let assets = vec![AssetRow {
            address: "STRK".into(),
            symbol: "STRK".into(),
            decimals: Some(18),
            is_native: true,
            balance: "1230000000000000000".into(),
        }];
        let normalized = StarknetAdapter::normalize_balances(assets).unwrap();
        assert_eq!(normalized[0].balance, "1.23");
    }

    #[test]
    fn calculates_counterfactual_account_address() {
        let address = StarknetAdapter::address_from_public_key_with_profile(
            "0x1",
            DEFAULT_OZ_ACCOUNT_CLASS_HASH,
        ).unwrap();
        assert!(address.starts_with("0x"));
    }
}
