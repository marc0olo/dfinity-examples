mod bitcoin_api;
mod bitcoin_wallet;
mod ecdsa_api;
mod schnorr_api;
mod types;

use ic_cdk::api::management_canister::bitcoin::{
    BitcoinNetwork, GetUtxosResponse, MillisatoshiPerByte,
};
use ic_cdk_macros::{init, update};
use std::cell::{Cell, RefCell};
use std::env;

thread_local! {
    // The bitcoin network to connect to.
    //
    // When developing locally this should be `Regtest`.
    // When deploying to the IC this should be `Testnet`.
    // `Mainnet` is currently unsupported.
    static NETWORK: Cell<BitcoinNetwork> = Cell::new(BitcoinNetwork::Testnet);

    // The derivation path to use for the threshold key.
    static DERIVATION_PATH: Vec<Vec<u8>> = vec![];

    // The ECDSA key name.
    static KEY_NAME: RefCell<String> = RefCell::new(String::from(""));

    pub static SCHNORR_CANISTER: RefCell<String> = RefCell::new(String::from(""));
}

#[init]
pub fn init(network: BitcoinNetwork, schnorr_canister: String) {
    NETWORK.with(|n| n.set(network));

    KEY_NAME.with(|key_name| {
        key_name.replace(String::from(match network {
            // For local development, we use a special test key with dfx.
            BitcoinNetwork::Regtest => "dfx_test_key",
            // On the IC we're using a test ECDSA key.
            BitcoinNetwork::Mainnet | BitcoinNetwork::Testnet => "test_key_1",
        }))
    });

    SCHNORR_CANISTER.with(|schnorr_canister_id| {
        let canister_id = env::var("CANISTER_ID_SCHNORR_CANISTER").unwrap_or(schnorr_canister);
        ic_cdk::println!("CANISTER_ID_SCHNORR_CANISTER: {}", &canister_id);
        schnorr_canister_id.replace(canister_id);
    });
}

#[update]
pub fn for_test_only_set_schnorr_canister_id(new_schnorr_canister_id: String) {
    SCHNORR_CANISTER.with(|schnorr_canister_id| {
        ic_cdk::println!(
            "Changing schnorr canister id from {} to {}",
            schnorr_canister_id.borrow(),
            new_schnorr_canister_id
        );
        schnorr_canister_id.replace(new_schnorr_canister_id);
    });
}

/// Returns the balance of the given bitcoin address.
#[update]
pub async fn get_balance(address: String) -> u64 {
    let network = NETWORK.with(|n| n.get());
    bitcoin_api::get_balance(network, address).await
}

/// Returns the UTXOs of the given bitcoin address.
#[update]
pub async fn get_utxos(address: String) -> GetUtxosResponse {
    let network = NETWORK.with(|n| n.get());
    bitcoin_api::get_utxos(network, address).await
}

/// Returns the 100 fee percentiles measured in millisatoshi/byte.
/// Percentiles are computed from the last 10,000 transactions (if available).
#[update]
pub async fn get_current_fee_percentiles() -> Vec<MillisatoshiPerByte> {
    let network = NETWORK.with(|n| n.get());
    bitcoin_api::get_current_fee_percentiles(network).await
}

/// Returns the P2PKH address of this canister at a specific derivation path.
#[update]
pub async fn get_p2pkh_address() -> String {
    let derivation_path = DERIVATION_PATH.with(|d| d.clone());
    let key_name = KEY_NAME.with(|kn| kn.borrow().to_string());
    let network = NETWORK.with(|n| n.get());
    bitcoin_wallet::p2pkh::get_address(network, key_name, derivation_path).await
}

/// Sends the given amount of bitcoin from this canister's p2pkh address to the given address.
/// Returns the transaction ID.
#[update]
pub async fn send_from_p2pkh(request: types::SendRequest) -> String {
    let derivation_path = DERIVATION_PATH.with(|d| d.clone());
    let network = NETWORK.with(|n| n.get());
    let key_name = KEY_NAME.with(|kn| kn.borrow().to_string());
    let tx_id = bitcoin_wallet::p2pkh::send(
        network,
        derivation_path,
        key_name,
        request.destination_address,
        request.amount_in_satoshi,
    )
    .await;

    tx_id.to_string()
}

/// Returns the P2TR address of this canister at a specific derivation path.
#[update]
pub async fn get_p2tr_script_spend_address() -> String {
    let mut derivation_path = DERIVATION_PATH.with(|d| d.clone());
    derivation_path.push(b"script_spend".to_vec());
    let key_name = KEY_NAME.with(|kn| kn.borrow().to_string());
    let network = NETWORK.with(|n| n.get());

    bitcoin_wallet::p2tr_script_spend::get_address(network, key_name, derivation_path)
        .await
        .to_string()
}

/// Sends the given amount of bitcoin from this canister's p2tr address to the given address.
/// Returns the transaction ID.
#[update]
pub async fn send_from_p2tr_script_spend(request: types::SendRequest) -> String {
    let mut derivation_path = DERIVATION_PATH.with(|d| d.clone());
    derivation_path.push(b"script_spend".to_vec());
    let network = NETWORK.with(|n| n.get());
    let key_name = KEY_NAME.with(|kn| kn.borrow().to_string());
    let tx_id = bitcoin_wallet::p2tr_script_spend::send(
        network,
        derivation_path,
        key_name,
        request.destination_address,
        request.amount_in_satoshi,
    )
    .await;

    tx_id.to_string()
}

/// Returns the P2TR address of this canister at a specific derivation path.
#[update]
pub async fn get_p2tr_raw_key_spend_address() -> String {
    let mut derivation_path = DERIVATION_PATH.with(|d| d.clone());
    derivation_path.push(b"key_spend".to_vec());
    let key_name = KEY_NAME.with(|kn| kn.borrow().to_string());
    let network = NETWORK.with(|n| n.get());

    bitcoin_wallet::p2tr_raw_key_spend::get_address(network, key_name, derivation_path)
        .await
        .to_string()
}

/// Sends the given amount of bitcoin from this canister's p2tr address to the
/// given address. Returns the transaction ID.
///
/// IMPORTANT: This function uses an untweaked key as the spending key.
///
/// WARNING: This function is not suited for multi-party scenarios where
/// multiple keys are used for spending.
#[update]
pub async fn send_from_p2tr_raw_key_spend(request: types::SendRequest) -> String {
    let mut derivation_path = DERIVATION_PATH.with(|d| d.clone());
    derivation_path.push(b"key_spend".to_vec());
    let network = NETWORK.with(|n| n.get());
    let key_name = KEY_NAME.with(|kn| kn.borrow().to_string());
    let tx_id = bitcoin_wallet::p2tr_raw_key_spend::send(
        network,
        derivation_path,
        key_name,
        request.destination_address,
        request.amount_in_satoshi,
    )
    .await;

    tx_id.to_string()
}
