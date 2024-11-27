use crate::deploy_config::{AptosNetwork, DeployConfig, DeployModuleType};
use anyhow::anyhow;
use aptos::common::types::{CliCommand, TransactionSummary};
use aptos::move_tool::MoveTool;
use aptos::Tool;
use aptos_sdk::move_types::account_address::AccountAddress;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

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

// Todo: handle already deployed contracts in named_addresses
pub async fn deploy_contracts(config: &DeployConfig) -> anyhow::Result<()> {
    let mut report_info = vec![];
    match config.module_type {
        DeployModuleType::Account => {
            todo!()
        }
        DeployModuleType::Object => {
            let mut object_addresses = HashMap::<String, String>::new();
            for (package_dir, address_name) in
                config.modules_path.iter().zip(&config.addresses_name)
            {
                println!(
                    "Deploying package {} with address name {}...",
                    package_dir.to_str().unwrap(),
                    address_name
                );
                let named_addresses = object_addresses
                    .iter()
                    .map(|(named_address, hex_address)| {
                        format!("{}={}", named_address, hex_address)
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

                object_addresses.insert(address_name.clone(), object_address.to_hex_literal());
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
            account: AccountAddress::from_str(&config.private_key)?,
            network: AptosNetwork::Mainnet,
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

#[cfg(test)]
mod test {
    use crate::deploy_config::AptosNetwork;
    use crate::tasks::deploy_contracts::deploy_contracts;
    use std::env::var;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_deploy_contracts() {
        let config = crate::deploy_config::DeployConfig {
            module_type: crate::deploy_config::DeployModuleType::Object,
            private_key: var("APTOS_PRIVATE_KEY").unwrap(),
            network: AptosNetwork::Testnet,
            modules_path: vec![
                PathBuf::from("/home/ubuntu/code/zkp/navori-2/libs"),
                PathBuf::from("/home/ubuntu/code/zkp/navori-2/cpu-2"),
                PathBuf::from("/home/ubuntu/code/zkp/navori-2/cpu"),
                PathBuf::from("/home/ubuntu/code/zkp/navori-2/verifier"),
            ],
            addresses_name: vec![
                "lib_addr".to_string(),
                "cpu_2_addr".to_string(),
                "cpu_addr".to_string(),
                "verifier_addr".to_string(),
            ],
            yes: true,
            output_json: PathBuf::from("test.json"),
        };
        deploy_contracts(&config).await.unwrap();
    }
}
