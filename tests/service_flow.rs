use nodex_wallet_engine::adapters::adapter_trait::WalletAdapter;
use nodex_wallet_engine::adapters::evm::EvmAdapter;
use nodex_wallet_engine::adapters::solana::SolanaAdapter;
use nodex_wallet_engine::adapters::utxo::UtxoAdapter;
use nodex_wallet_engine::services::address_service::AddressService;
use nodex_wallet_engine::crypto::key_derivation::KeyDerivationService;
use nodex_wallet_engine::services::wallet_create_service::SecretPhraseService;
use nodex_wallet_engine::wallet_engine::WalletProvider;
use nodex_wallet_engine::types::chain::ChainRequest;

#[test]
fn wallet_provider_routes_evm_request_through_key_derivation_and_address_service() {
    let payload = SecretPhraseService::create(12).expect("secret phrase creation should succeed");
    let provider = WalletProvider::new(&payload.mnemonic).expect("wallet provider should initialize");

    let public_key = KeyDerivationService::derive_public_key(&payload.seed, "evm").expect("evm public key derivation should succeed");
    let expected_address = AddressService::public_key_to_address("evm", &public_key)
        .expect("address service should derive evm address");

    let actual_address = provider.get_address_by_chain(&ChainRequest { name: "Ethereum".into(), chain_id: "1".into(), symbol: Some("ETH".into()), network: None, chain_type: "evm".into(), derivation_path: None, rpc_provider: None, rpc_url: None, address_type: None, p2pkh_version: None, p2sh_version: None, bech32_hrp: None }).expect("wallet provider should derive evm address");

    assert_eq!(actual_address, expected_address);
    assert!(actual_address.starts_with("0x"));
}

#[test]
fn wallet_provider_routes_solana_request_through_key_derivation_and_address_service() {
    let payload = SecretPhraseService::create(12).expect("secret phrase creation should succeed");
    let provider = WalletProvider::new(&payload.mnemonic).expect("wallet provider should initialize");

    let public_key = KeyDerivationService::derive_public_key(&payload.seed, "solana").expect("solana public key derivation should succeed");
    let expected_address = AddressService::public_key_to_address("solana", &public_key)
        .expect("address service should derive solana address");

    let actual_address = provider.get_address_by_chain(&ChainRequest { name: "Solana".into(), chain_id: "solana".into(), symbol: Some("SOL".into()), network: None, chain_type: "solana".into(), derivation_path: None, rpc_provider: None, rpc_url: None, address_type: None, p2pkh_version: None, p2sh_version: None, bech32_hrp: None }).expect("wallet provider should derive solana address");

    assert_eq!(actual_address, expected_address);
    assert!(!actual_address.is_empty());
}

#[test]
fn wallet_provider_routes_bitcoin_request_through_key_derivation_and_address_service() {
    let payload = SecretPhraseService::create(12).expect("secret phrase creation should succeed");
    let provider = WalletProvider::new(&payload.mnemonic).expect("wallet provider should initialize");

    let public_key = KeyDerivationService::derive_public_key(&payload.seed, "bitcoin").expect("bitcoin public key derivation should succeed");
    let expected_address = AddressService::public_key_to_address("bitcoin", &public_key)
        .expect("address service should derive bitcoin address");

    let actual_address = provider.get_address_by_chain(&ChainRequest { name: "Bitcoin".into(), chain_id: "bitcoin".into(), symbol: Some("BTC".into()), network: None, chain_type: "bitcoin".into(), derivation_path: None, rpc_provider: None, rpc_url: None, address_type: None, p2pkh_version: None, p2sh_version: None, bech32_hrp: None }).expect("wallet provider should derive bitcoin address");

    assert_eq!(actual_address, expected_address);
    assert!(!actual_address.is_empty());
}

#[test]
fn address_service_public_key_to_address_uses_adapter_boundaries() {
    let evm_key = KeyDerivationService::derive_public_key(&[0u8; 64], "evm").expect("evm key");
    let solana_key = KeyDerivationService::derive_public_key(&[0u8; 64], "solana").expect("solana key");
    let bitcoin_key = KeyDerivationService::derive_public_key(&[0u8; 64], "bitcoin").expect("bitcoin key");

    let evm_address = AddressService::public_key_to_address("evm", &evm_key).expect("evm address");
    let solana_address = AddressService::public_key_to_address("solana", &solana_key).expect("solana address");
    let bitcoin_address = AddressService::public_key_to_address("bitcoin", &bitcoin_key).expect("bitcoin address");

    assert!(evm_address.starts_with("0x"));
    assert!(!solana_address.is_empty());
    assert!(!bitcoin_address.is_empty());

    assert_eq!(evm_address, EvmAdapter::pub_key_to_address(&evm_key).expect("evm adapter address match"));
    assert_eq!(solana_address, SolanaAdapter::pub_key_to_address(&solana_key).expect("solana adapter address match"));
    assert_eq!(bitcoin_address, UtxoAdapter::pub_key_to_address(&bitcoin_key).expect("utxo adapter address match"));
}
