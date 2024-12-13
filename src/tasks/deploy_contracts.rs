use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs, panic};

use anyhow::{anyhow, ensure};
use aptos::common::types::{CliCommand, CliError, TransactionSummary};
use aptos::move_tool::MoveTool;
use aptos::Tool;
use aptos_sdk::crypto::ValidCryptoMaterialStringExt;
use aptos_sdk::move_types::account_address::AccountAddress;
use aptos_sdk::types::LocalAccount;
use clap::Parser;
use config::{Config, File, FileFormat};
use dialoguer::theme::ColorfulTheme;
use dialoguer::Confirm;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::deploy_config::{AptosNetwork, DeployConfig, DeployModuleType};
use crate::utils::{generate_account_and_faucet, DEFAULT_FAUCET_AMOUNT};

const DEPLOYER_PROFILE: &str = "jayce_deployer";

#[derive(Deserialize, Debug, Clone)]
pub struct MoveTomlFile {
    pub addresses: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
struct DeployReport {
    account: AccountAddress,
    network: AptosNetwork,
    info: Vec<TxReport>,
}

#[derive(Serialize, Deserialize)]
struct TxReport {
    module_path: PathBuf,
    address_name: String,
    deployed_at: AccountAddress,
    tx_info: Vec<TransactionSummary>,
}

pub async fn deploy_contracts(mut config: DeployConfig) -> anyhow::Result<()> {
    let report_info: Arc<Mutex<Vec<TxReport>>> = Arc::new(Mutex::new(vec![]));
    let sender_addr = match &config.private_key {
        None => {
            if !config.yes
                && !Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("No private key provided, do you want to generate one?")
                    .default(false)
                    .show_default(true)
                    .wait_for_newline(true)
                    .interact()?
            {
                return Ok(());
            }
            let account = generate_account_and_faucet(
                &config.network,
                config.faucet_url.clone(),
                config.rest_url.clone(),
            )
            .await?;
            let private_key = account.private_key().to_encoded_string()?;
            let address = account.address();
            println!(
                "Generated account with address: {}, balance: {} Octas",
                address, DEFAULT_FAUCET_AMOUNT
            );
            println!("Your private key is: {}", private_key);
            config.private_key = Some(private_key);
            address
        }
        Some(private_key) => LocalAccount::from_private_key(private_key, 0)?.address(),
    };

    create_profile(&config).await?;

    let config = Arc::new(config);
    let report_info_clone = Arc::clone(&report_info);
    let config_clone = Arc::clone(&config);
    let result = tokio::spawn(async move {
        let mut report_info = report_info_clone.lock().await;
        run_core(&config_clone, &mut report_info, sender_addr).await
    })
    .await;

    fs::write(
        &config.output_json,
        serde_json::to_string_pretty(&DeployReport {
            account: sender_addr,
            network: config.network.clone(),
            info: std::mem::take(&mut *report_info.lock().await),
        })?,
    )?;
    remove_profile()?;
    match result {
        Ok(result) => result,
        Err(err) => Err(err.into()),
    }
}

async fn run_core(
    config: &DeployConfig,
    report_info: &mut Vec<TxReport>,
    sender_addr: AccountAddress,
) -> anyhow::Result<()> {
    let mut deployed_addresses = config.deployed_addresses.clone();
    for (package_dir, address_name) in config.modules_path.iter().zip(&config.addresses_name) {
        if deployed_addresses.contains_key(address_name) {
            println!(
                "Address name {} already deployed, skipping...",
                address_name
            );
            continue;
        }
        println!(
            "Deploying package {} with address name {}...",
            package_dir.to_str().unwrap(),
            address_name
        );
        let named_addresses =
            get_named_addresses(package_dir, address_name, config.module_type.clone())?;
        let named_addresses = named_addresses
            .keys()
            .map(|named_address| {
                let mut hex_address = deployed_addresses.get(named_address);
                if hex_address.is_none() {
                    if named_address == address_name {
                        hex_address = Some(&sender_addr);
                    } else {
                        panic!(
                            "{}",
                            format!(
                                "'{}' should be deployed before '{}'",
                                named_address, address_name
                            )
                        );
                    }
                }
                format!("{}={}", named_address, hex_address.unwrap())
            })
            .reduce(|acc, cur| format!("{},{}", acc, cur))
            .map(|named_addresses| format!("--named-addresses {}", named_addresses))
            .unwrap_or("".to_string());

        let args = format!(
            "aptos move {} \
                    --package-dir {} \
                    --included-artifacts {} \
                    --profile {} \
                    {} \
                    {} \
                    ",
            match config.module_type {
                DeployModuleType::Object => "create-object-and-publish-package",
                DeployModuleType::Account => "publish",
            },
            package_dir.to_str().unwrap(),
            if config.publish_code { "all" } else { "none" },
            DEPLOYER_PROFILE,
            match config.module_type {
                DeployModuleType::Account => "".to_string(),
                DeployModuleType::Object => format!("--address-name {}", address_name),
            },
            named_addresses
        );
        let mut args: Vec<&str> = args.split_whitespace().collect();

        if config.yes {
            args.push("--assume-yes");
        }

        let (tx_info, deployed_at) = match run_deploy_command(&args).await {
            Ok(x) => x,
            Err(err) => {
                match err {
                    CliError::PackageSizeExceeded(err1, err0) => {
                        println!(
                            "The package is larger than {} bytes ({} bytes)!",
                            err1, err0
                        );
                        match config.network {
                            AptosNetwork::Mainnet | AptosNetwork::Testnet => {
                                if !config.yes && !Confirm::with_theme(&ColorfulTheme::default())
                                    .with_prompt("Do you want to publish packages using chunked publish?")
                                    .default(false)
                                    .show_default(true)
                                    .wait_for_newline(true)
                                    .interact()? {
                                    return Err(err.into());
                                } else {
                                    args.push("--chunked-publish");
                                    run_deploy_command(&args).await?
                                }
                            }
                            _ => {
                                return Err(anyhow!(
                                    "{} is not supported for chunked publish",
                                    config.network
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(err.into());
                    }
                }
            }
        };

        let deployed_at = match config.module_type {
            DeployModuleType::Account => sender_addr,
            DeployModuleType::Object => deployed_at.unwrap(),
        };
        deployed_addresses.insert(address_name.clone(), deployed_at);
        report_info.push(TxReport {
            module_path: package_dir.clone(),
            address_name: address_name.clone(),
            deployed_at,
            tx_info,
        });
    }
    Ok(())
}

async fn create_profile(config: &DeployConfig) -> anyhow::Result<()> {
    let private_key = config
        .private_key
        .clone()
        .expect("Private key not found, this should not happen");
    let rest_url = match config.rest_url.clone() {
        None => config.network.rest_url().expect("Failed to get rest url"),
        Some(rest_url) => rest_url,
    };
    let faucet_url = match config.faucet_url.clone() {
        None => config
            .network
            .faucet_url()
            .expect("Failed to get faucet url"),
        Some(faucet_url) => faucet_url,
    };

    let command = format!(
        "aptos init \
        --network {} \
        --profile {} \
        --private-key {} \
        --rest-url {} \
        --faucet-url {}",
        config.network, DEPLOYER_PROFILE, private_key, rest_url, faucet_url
    );
    let command: Vec<&str> = command.split_whitespace().collect();
    let tool = Tool::try_parse_from(&command).expect("Failed to parse arguments");
    if let Tool::Init(cmd_executor) = tool {
        Ok(cmd_executor.execute().await?)
    } else {
        Err(anyhow!(format!(
            "Wrong arguments to deploy contracts: {:?}",
            command
        )))
    }
}

fn remove_profile() -> anyhow::Result<()> {
    let mut config_yaml: serde_yaml::Value = Config::builder()
        .add_source(File::new(".aptos/config.yaml", FileFormat::Yaml))
        .build()?
        .try_deserialize()?;
    let profiles = config_yaml["profiles"].as_mapping_mut().unwrap();
    if profiles.len() == 1 {
        if profiles.contains_key(DEPLOYER_PROFILE) {
            fs::remove_dir_all(".aptos")?;
        }
    } else if profiles.remove(DEPLOYER_PROFILE).is_some() {
        fs::write(".aptos/config.yaml", serde_yaml::to_string(&config_yaml)?)?;
    }
    Ok(())
}

async fn run_deploy_command(
    args: &Vec<&str>,
) -> anyhow::Result<(Vec<TransactionSummary>, Option<AccountAddress>), CliError> {
    let tool = Tool::try_parse_from(args).expect("Failed to parse arguments");

    if let Tool::Move(MoveTool::CreateObjectAndPublishPackage(cmd_executor)) = tool {
        let (tx_info, object_addr) = cmd_executor.execute().await?;
        Ok((tx_info, Some(object_addr)))
    } else if let Tool::Move(MoveTool::Publish(cmd_executor)) = tool {
        let tx_info = cmd_executor.execute().await?;
        Ok((tx_info, None))
    } else {
        Err(CliError::UnexpectedError(format!(
            "Wrong arguments to deploy contracts: {:?}",
            args
        )))
    }
}

fn get_named_addresses(
    package_dir: &Path,
    address_name: &String,
    module_type: DeployModuleType,
) -> anyhow::Result<HashMap<String, String>> {
    let move_toml: MoveTomlFile = Config::builder()
        .add_source(File::new(
            package_dir.join("Move.toml").to_str().unwrap(),
            FileFormat::Toml,
        ))
        .build()?
        .try_deserialize()?;
    let mut named_addresses = move_toml.addresses;
    ensure!(
        named_addresses.contains_key(address_name),
        format!(
            "Address name {} not found in {}/Move.toml",
            address_name,
            package_dir.to_str().unwrap()
        )
    );
    if module_type == DeployModuleType::Object {
        named_addresses.remove(address_name);
    }
    Ok(named_addresses)
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;
    use std::env::var;
    use std::path::PathBuf;
    use std::str::FromStr;

    use aptos_sdk::types::account_address::AccountAddress;

    use crate::deploy_config::{AptosNetwork, DeployConfig};
    use crate::tasks::deploy_contracts::deploy_contracts;

    #[tokio::test]
    async fn test_deploy_contracts() {
        let mut deployed_addresses = BTreeMap::new();
        deployed_addresses.insert(
            "lib_addr".to_string(),
            AccountAddress::from_str(
                "2d01428a36c36c2799e2c489f02b09f08339dc6321cef017458f7e21ea8a0fcc",
            )
            .unwrap(),
        );
        deployed_addresses.insert(
            "cpu_2_addr".to_string(),
            AccountAddress::from_str(
                "7b38c1276ba8662d085df8f4f4226d314d0296bc6dc955c43ea2b9ec05829980",
            )
            .unwrap(),
        );
        let config = DeployConfig {
            module_type: crate::deploy_config::DeployModuleType::Object,
            private_key: Some(var("APTOS_PRIVATE_KEY").unwrap()),
            network: AptosNetwork::Testnet,
            modules_path: vec![
                PathBuf::from("examples/contracts/navori/cpu"),
                PathBuf::from("examples/contracts/navori/verifier"),
            ],
            addresses_name: vec!["cpu_addr".to_string(), "verifier_addr".to_string()],
            yes: true,
            output_json: PathBuf::from("test.json"),
            deployed_addresses,
            rest_url: None,
            faucet_url: None,
            publish_code: true,
        };
        deploy_contracts(config).await.unwrap();
    }
}
