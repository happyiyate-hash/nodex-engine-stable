use crate::common::errors::EngineError;
use crate::crypto::mnemonic::{generate_mnemonic, validate_mnemonic};

pub fn generate_bip39_mnemonic(word_count: usize) -> Result<String, EngineError> {
    generate_mnemonic(word_count)
}

pub fn validate_bip39_mnemonic(mnemonic: &str) -> Result<(), EngineError> {
    validate_mnemonic(mnemonic)
}

#[cfg(test)]
mod tests {
    use super::{generate_bip39_mnemonic, validate_bip39_mnemonic};

    #[test]
    fn bip39_generate_and_validate_round_trip() {
        let mnemonic = generate_bip39_mnemonic(24).expect("24-word mnemonic should be generated");
        println!("mnemonic: {mnemonic}");
        let result = validate_bip39_mnemonic(&mnemonic);
        println!("validation result: {result:?}");
        assert!(result.is_ok());
    }
}