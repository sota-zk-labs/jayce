use crate::deploy_config::AptosNetwork;
use anyhow::{anyhow, ensure};
use aptos_sdk::rest_client::FaucetClient;
use aptos_sdk::types::LocalAccount;
use rand::rngs::OsRng;
use std::process::Command;
use std::str::FromStr;
use url::Url;

pub const DEFAULT_FAUCET_AMOUNT: u64 = 100_000_000;
pub fn check_aptos_installed() -> anyhow::Result<()> {
    ensure!(
        Command::new("aptos").output().is_ok(),
        "Aptos CLI not found. Please install it from https://aptos.dev/en/build/cli"
    );
    Ok(())
}

pub async fn generate_account_and_faucet(
    network: &AptosNetwork,
    mut faucet_url: Option<String>,
    mut rest_url: Option<String>,
) -> anyhow::Result<LocalAccount> {
    let account = LocalAccount::generate(&mut OsRng);
    if faucet_url.is_none() {
        faucet_url = network.faucet_url();
    }
    if faucet_url.is_none() {
        return Err(anyhow!(format!(
            "Faucet URL not found for network: {}",
            network
        )));
    }
    if rest_url.is_none() {
        rest_url = network.rest_url();
    }
    if rest_url.is_none() {
        return Err(anyhow!(format!(
            "REST URL not found for network: {}",
            network
        )));
    }
    let faucet_client = FaucetClient::new(
        Url::from_str(&faucet_url.unwrap())?,
        Url::from_str(&rest_url.unwrap())?,
    );

    faucet_client
        .fund(account.address(), DEFAULT_FAUCET_AMOUNT)
        .await?;
    Ok(account)
}
