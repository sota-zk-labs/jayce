use crate::deploy_config::{AptosNetwork, DeployConfig, DeployModuleType};
use anyhow::{anyhow, ensure};
use aptos::common::types::{CliCommand, TransactionSummary};
use aptos::move_tool::MoveTool;
use aptos::Tool;
use aptos_sdk::move_types::account_address::AccountAddress;
use clap::Parser;
use config::{Config, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use aptos_sdk::types::LocalAccount;

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
    object_address: Option<AccountAddress>,
    tx_info: TransactionSummary,
}

pub async fn deploy_contracts(config: &DeployConfig) -> anyhow::Result<()> {
    let mut report_info = vec![];
    match config.module_type {
        DeployModuleType::Account => {
            todo!()
        }
        DeployModuleType::Object => {
            let mut object_addresses = config.deployed_addresses.clone();
            for (package_dir, address_name) in
                config.modules_path.iter().zip(&config.addresses_name)
            {
                if object_addresses.contains_key(address_name) {
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
                let named_addresses = get_named_addresses(package_dir, address_name)?;
                let named_addresses = named_addresses
                    .iter()
                    .map(|(named_address, _)| {
                        let hex_address = object_addresses.get(named_address);
                        if hex_address.is_none() {
                            panic!(
                                "{}",
                                format!(
                                    "{} should be deployed before {}",
                                    named_address, address_name
                                )
                            );
                        }
                        format!("{}={}", named_address, hex_address.unwrap())
                    })
                    .reduce(|acc, cur| format!("{},{}", acc, cur))
                    .map(|named_addresses| format!("--named-addresses {}", named_addresses))
                    .unwrap_or("".to_string());

                let args = format!(
                    "aptos move create-object-and-publish-package \
                --package-dir {} \
                --private-key {} \
                --skip-fetch-latest-git-deps \
                --included-artifacts none \
                --address-name {} \
                --url {} \
                {} \
                ",
                    package_dir.to_str().unwrap(),
                    &config.private_key,
                    address_name,
                    config.network.rest_url(),
                    named_addresses
                );
                let mut args: Vec<&str> = args.split_whitespace().collect();

                if config.yes {
                    args.push("--assume-yes");
                }

                let (tx_info, object_address) = deploy_to_object(&args).await?;

                object_addresses.insert(address_name.clone(), object_address);
                report_info.push(TxReport {
                    module_path: package_dir.clone(),
                    address_name: address_name.clone(),
                    object_address: Some(object_address),
                    tx_info,
                });
            }
        }
    };
    fs::write(
        &config.output_json,
        serde_json::to_string_pretty(&DeployReport {
            account: LocalAccount::from_private_key(&config.private_key, 0)?.address(),
            network: config.network.clone(),
            info: report_info,
        })?,
    )?;
    Ok(())
}

async fn deploy_to_object(
    args: &Vec<&str>,
) -> anyhow::Result<(TransactionSummary, AccountAddress)> {
    let tool = Tool::try_parse_from(args).expect("Failed to parse arguments");

    // Match on the parsed `Tool` to extract `CreateObjectAndPublishPackage`
    if let Tool::Move(MoveTool::CreateObjectAndPublishPackage(cmd)) = tool {
        Ok(cmd.execute().await?)
    } else {
        Err(anyhow!(format!(
            "Wrong arguments to deploy contracts: {:?}",
            args
        )))
    }
}

fn get_named_addresses(
    package_dir: &PathBuf,
    address_name: &String,
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
    named_addresses.remove(address_name);
    Ok(named_addresses)
}

#[cfg(test)]
mod test {
    use crate::deploy_config::AptosNetwork;
    use crate::tasks::deploy_contracts::deploy_contracts;
    use aptos_sdk::types::account_address::AccountAddress;
    use std::collections::BTreeMap;
    use std::env::var;
    use std::path::PathBuf;
    use std::str::FromStr;
    use aptos_sdk::types::LocalAccount;

    #[tokio::test]
    async fn test_deploy_contracts() {
        let mut deployed_addresses = BTreeMap::new();
        deployed_addresses.insert(
            "lib_addr".to_string(),
            AccountAddress::from_str(
                "2d77ba9653c5260988950fd4cbd47dac49934cee8152d6a4a32b866d86a600b1",
            )
            .unwrap(),
        );
        deployed_addresses.insert(
            "cpu_2_addr".to_string(),
            AccountAddress::from_str(
                "1b9750db89454d4697480a49908ac7d703f6d6db2b2b79ea9b2d8201485dbbfa",
            )
            .unwrap(),
        );
        let config = crate::deploy_config::DeployConfig {
            module_type: crate::deploy_config::DeployModuleType::Object,
            private_key: var("APTOS_PRIVATE_KEY").unwrap(),
            network: AptosNetwork::Testnet,
            modules_path: vec![
                // PathBuf::from("examples/contracts/navori/libs"),
                // PathBuf::from("examples/contracts/navori/cpu-2"),
                PathBuf::from("examples/contracts/navori/cpu"),
                PathBuf::from("examples/contracts/navori/verifier"),
            ],
            addresses_name: vec![
                // "lib_addr".to_string(),
                // "cpu_2_addr".to_string(),
                "cpu_addr".to_string(),
                "verifier_addr".to_string(),
            ],
            yes: true,
            output_json: PathBuf::from("test.json"),
            deployed_addresses,
        };
        deploy_contracts(&config).await.unwrap();
    }
}
