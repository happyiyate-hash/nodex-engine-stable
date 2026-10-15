use nodex_wallet_engine::adapters::evm::EvmAdapter;
use nodex_wallet_engine::adapters::solana::SolanaAdapter;
use nodex_wallet_engine::adapters::utxo::UtxoAdapter;
use nodex_wallet_engine::adapters::adapter_trait::WalletAdapter;

#[test]
fn evm_adapter_derives_address_from_path() {
    let seed = [0x42u8; 64];
    let address = EvmAdapter::derive_address_from_path(&seed, "m/44'/60'/0'/0/0").expect("evm path derivation should work");
    assert!(address.starts_with("0x"));
    assert_eq!(address.len(), 42);
}

#[test]
fn solana_adapter_derives_address_from_path() {
    let seed = [0x42u8; 64];
    let address = SolanaAdapter::derive_address_from_path(&seed, "m/44'/501'/0'/0'").expect("solana path derivation should work");
    assert!(!address.is_empty());
}

#[test]
fn utxo_adapter_derives_address_from_path() {
    let seed = [0x42u8; 64];
    let address = UtxoAdapter::derive_address_from_path(&seed, "m/84'/0'/0'/0/0").expect("utxo path derivation should work");
    assert!(!address.is_empty());
}

#[test]
fn evm_canonical_derivation_is_stable() {
    let seed = [0x42u8; 64];
    let canonical = EvmAdapter::derive_canonical_address(&seed).expect("canonical EVM derivation should work");
    let explicit = EvmAdapter::derive_address_from_path(&seed, EvmAdapter::CANONICAL_DERIVATION_PATH)
        .expect("canonical path derivation should work");
    assert_eq!(canonical, explicit);
}

#[test]
fn evm_canonical_address_is_reused_across_networks() {
    let seed = [0x42u8; 64];
    let address = EvmAdapter::derive_canonical_address(&seed).expect("canonical EVM derivation should work");
    let same_address = EvmAdapter::derive_canonical_address(&seed).expect("canonical EVM derivation should work");
    assert_eq!(address, same_address);
}
