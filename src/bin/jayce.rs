use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

use anyhow::{ensure, Result};
use aptos_sdk::move_types::account_address::AccountAddress;
use clap::{CommandFactory, Parser, Subcommand};
use jayce::deploy_config::{AptosNetwork, DeployConfig, DeployModuleType, PartialDeployConfig};
use jayce::tasks::deploy_contracts::deploy_contracts;

// Todo: add descriptions to the commands
#[derive(Parser, Debug)]
#[command(name = "jayce")]
#[command(about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[clap(short, long)]
    version: bool,
}

#[derive(Subcommand, Clone, Debug, PartialEq)]
enum Commands {
    /// Deploy contracts
    Deploy {
        #[arg(long)]
        private_key: Option<String>,
        #[arg(long, default_value_t = DeployModuleType::Object)]
        module_type: DeployModuleType,
        #[arg(long, num_args = 1.., value_delimiter = ',')]
        modules_path: Option<Vec<PathBuf>>,
        #[arg(long, num_args = 1.., value_delimiter = ',')]
        addresses_name: Option<Vec<String>>,
        #[arg(long, default_value_t = AptosNetwork::Devnet)]
        network: AptosNetwork,
        #[arg(long, default_value = "deploy-report.json")]
        output_json: PathBuf,
        #[arg(long, value_parser = aptos::common::utils::parse_map::<String, AccountAddress>)]
        deployed_addresses: BTreeMap<String, AccountAddress>,
        #[arg(short, long, default_value_t = false)]
        yes: bool,
        /// Sets a custom config file
        #[arg(long)]
        config_path: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let args_str: Vec<String> = env::args().collect();
    if args.version {
        println!(env!("APP_VERSION"));
        return Ok(());
    }
    match args.command {
        None => {
            Cli::command().print_help()?;
            Ok(())
        }
        Some(command) => match command {
            Commands::Deploy {
                private_key,
                addresses_name,
                network,
                output_json,
                deployed_addresses,
                yes,
                config_path,
                module_type,
                modules_path,
            } => {
                let mut partial_deploy_config = if let Some(config_path) = config_path {
                    PartialDeployConfig::from_path(config_path.to_str().unwrap())?
                } else {
                    PartialDeployConfig {
                        private_key: None,
                        module_type: None,
                        modules_path: None,
                        addresses_name: None,
                        network: None,
                        yes: None,
                        output_json: None,
                        deployed_addresses: None,
                    }
                };
                if private_key.is_some() {
                    partial_deploy_config.private_key = private_key;
                }
                if partial_deploy_config.module_type.is_none()
                    || args_str.contains(&"--module-type".to_string())
                {
                    partial_deploy_config.module_type = Some(module_type);
                }
                if modules_path.is_some() {
                    partial_deploy_config.modules_path = modules_path;
                }
                if addresses_name.is_some() {
                    partial_deploy_config.addresses_name = addresses_name;
                }
                if partial_deploy_config.network.is_none()
                    || args_str.contains(&"--network".to_string())
                {
                    partial_deploy_config.network = Some(network);
                }
                if partial_deploy_config.yes.is_none()
                    || args_str.contains(&"--yes".to_string())
                    || args_str.contains(&"-y".to_string())
                {
                    partial_deploy_config.yes = Some(yes);
                }
                if partial_deploy_config.output_json.is_none()
                    || args_str.contains(&"--output-json".to_string())
                {
                    partial_deploy_config.output_json = Some(output_json);
                }
                if partial_deploy_config.deployed_addresses.is_none()
                    || args_str.contains(&"--deployed-addresses".to_string())
                {
                    partial_deploy_config.deployed_addresses = Some(deployed_addresses);
                }

                let deploy_config = DeployConfig::from(partial_deploy_config);
                ensure!(
                    deploy_config.modules_path.len() == deploy_config.addresses_name.len(),
                    "Modules path and addresses name must have the same length"
                );

                deploy_contracts(&deploy_config).await
            }
        },
    }
}
