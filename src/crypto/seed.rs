use crate::common::errors::EngineError;
use pbkdf2::pbkdf2_hmac_array;
use sha2::Sha512;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone)]
pub struct Mnemonic(String);

impl Mnemonic {
    pub fn new(value: &str) -> Result<Self, EngineError> {
        if value.trim().is_empty() {
            return Err(EngineError::Validation("mnemonic cannot be empty".into()));
        }
        // Canonicalize whitespace at the input boundary. BIP-39 then requires
        // NFKD normalization when converting this sentence to a seed.
        Ok(Self(value.split_whitespace().collect::<Vec<_>>().join(" ")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn derive_master_seed(mnemonic: &Mnemonic, passphrase: Option<&str>) -> Result<Vec<u8>, EngineError> {
    // BIP-39: both the mnemonic sentence and the passphrase salt must be
    // UTF-8 NFKD-normalized before PBKDF2-HMAC-SHA512.
    let normalized_mnemonic: String = mnemonic.as_str().nfkd().collect();
    let normalized_passphrase: String = passphrase.unwrap_or_default().nfkd().collect();
    let salt = format!("mnemonic{normalized_passphrase}");
    let seed: [u8; 64] = pbkdf2_hmac_array::<Sha512, 64>(
        normalized_mnemonic.as_bytes(),
        salt.as_bytes(),
        2048,
    );
    Ok(seed.to_vec())
}

#[cfg(test)]
mod tests {
    use super::{derive_master_seed, Mnemonic};

    #[test]
    fn matches_bip39_official_seed_vector() {
        // BIP-39 vector: "abandon ... about", passphrase "TREZOR".
        // Expected seed prefix from the published reference vector.
        let mnemonic = Mnemonic::new(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        )
        .expect("valid mnemonic");
        let seed = derive_master_seed(&mnemonic, Some("TREZOR"))
            .expect("seed derivation should work");
        assert_eq!(seed.len(), 64);
        assert_eq!(&hex::encode(seed)[..16], "c55257c360c07c72");
    }

    #[test]
    fn canonically_equivalent_passphrases_produce_the_same_seed() {
        let mnemonic = Mnemonic::new(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        )
        .expect("valid mnemonic");
        let composed = derive_master_seed(&mnemonic, Some("caf\u{00e9}"))
            .expect("seed derivation should work");
        let decomposed = derive_master_seed(&mnemonic, Some("cafe\u{0301}"))
            .expect("seed derivation should work");
        assert_eq!(composed, decomposed);
    }

    #[test]
    fn derives_consistent_seed_from_mnemonic() {
        let mnemonic = Mnemonic::new(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        )
        .expect("valid mnemonic");
        let seed_a = derive_master_seed(&mnemonic, Some("password"))
            .expect("seed generation should work");
        let seed_b = derive_master_seed(&mnemonic, Some("password"))
            .expect("seed generation should work");
        assert_eq!(seed_a, seed_b);
    }
}
