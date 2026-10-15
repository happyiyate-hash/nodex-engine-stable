//! Browser entry point for the NodeX wallet runtime.
//!
//! Security boundary: never export seed bytes or private keys to JavaScript.
//! Keep secret-bearing operations inside this module as the runtime grows.

use bip39::{Language, Mnemonic};
use wasm_bindgen::prelude::*;

const WORD_COUNT: usize = 12;

#[wasm_bindgen]
pub fn runtime_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Generate a BIP-39 English recovery phrase using the browser's secure
/// randomness source provided by getrandom's wasm-bindgen-compatible backend.
///
/// The phrase is intentionally returned only to the calling UI so it can be
/// shown once for the user to back up. Callers must not log or persist it in
/// plaintext.
#[wasm_bindgen]
pub fn generate_recovery_phrase() -> Result<String, JsValue> {
    Mnemonic::generate_in(Language::English, WORD_COUNT)
        .map(|mnemonic| mnemonic.to_string())
        .map_err(|error| JsValue::from_str(&format!("Mnemonic generation failed: {error}")))
}

/// Validate a BIP-39 phrase and its checksum without sending it to a server.
#[wasm_bindgen]
pub fn validate_recovery_phrase(phrase: &str) -> bool {
    let normalized = phrase.split_whitespace().collect::<Vec<_>>().join(" ");
    Mnemonic::parse_in(Language::English, &normalized).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_phrase_is_valid_bip39() {
        let phrase = generate_recovery_phrase().expect("phrase generation should succeed");
        assert_eq!(phrase.split_whitespace().count(), WORD_COUNT);
        assert!(validate_recovery_phrase(&phrase));
    }

    #[test]
    fn invalid_phrase_is_rejected() {
        assert!(!validate_recovery_phrase("not a valid recovery phrase"));
    }

    #[test]
    fn valid_known_phrase_passes_checksum() {
        assert!(validate_recovery_phrase(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        ));
    }
}
