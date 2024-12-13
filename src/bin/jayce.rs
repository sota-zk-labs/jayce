use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

use anyhow::{ensure, Result};
use aptos_sdk::move_types::account_address::AccountAddress;
use clap::{CommandFactory, Parser, Subcommand};
use jayce::deploy_config::{AptosNetwork, DeployConfig, DeployModuleType, PartialDeployConfig};
use jayce::tasks::deploy_contracts::deploy_contracts;

#[derive(Parser, Debug)]
#[command(name = "jayce")]
#[command(about = "Jayce CLI tool for deploying contracts", long_about = None)]
struct Cli {
    /// The subcommand to execute
    #[command(subcommand)]
    command: Option<Commands>,

    /// Display the version of the CLI tool
    #[clap(short, long)]
    version: bool,
}

#[derive(Subcommand, Clone, Debug, PartialEq)]
enum Commands {
    /// Deploy contracts
    Deploy {
        /// The private key used for deployment
        #[arg(long)]
        private_key: Option<String>,
        /// The type of module to deploy
        #[arg(long, default_value_t = DeployModuleType::Object)]
        module_type: DeployModuleType,
        /// Paths to the modules to be deployed, separated by commas
        #[arg(long, num_args = 1.., value_delimiter = ',')]
        modules_path: Option<Vec<PathBuf>>,
        /// Names of the addresses corresponding to the modules (must identify with your Move.toml), separated by commas
        #[arg(long, num_args = 1.., value_delimiter = ',')]
        addresses_name: Option<Vec<String>>,
        /// The network to deploy to
        #[arg(long, default_value_t = AptosNetwork::Devnet)]
        network: AptosNetwork,
        /// The path to the output JSON file for the deployment report
        #[arg(long, default_value = "deploy-report.json")]
        output_json: PathBuf,
        /// A map of already deployed addresses, e.g. addr_1=0x1,addr_2=0x2
        #[arg(long, value_parser = aptos::common::utils::parse_map::<String, AccountAddress>, default_value = "")]
        deployed_addresses: BTreeMap<String, AccountAddress>,
        /// REST url for the network, used for local network
        #[arg(long)]
        rest_url: Option<String>,
        /// Faucet url for the network, used when private key is not provided
        #[arg(long)]
        faucet_url: Option<String>,
        /// Publish your code onchain
        #[arg(long, default_value_t = false)]
        publish_code: bool,
        /// Automatically confirm prompts
        #[arg(short, long, default_value_t = false)]
        yes: bool,
        /// Path to the toml configuration file
        #[arg(long)]
        config_path: Option<PathBuf>,
    },
}

#[allow(clippy::needless_return)]
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
                rest_url,
                faucet_url,
                publish_code,
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
                        rest_url: None,
                        faucet_url: None,
                        publish_code: None,
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
                if rest_url.is_some() {
                    partial_deploy_config.rest_url = rest_url;
                }
                if faucet_url.is_some() {
                    partial_deploy_config.faucet_url = faucet_url;
                }
                if partial_deploy_config.publish_code.is_none()
                    || args_str.contains(&"--publish-code".to_string())
                {
                    partial_deploy_config.publish_code = Some(publish_code);
                }

                let deploy_config = DeployConfig::from(partial_deploy_config);
                ensure!(
                    deploy_config.modules_path.len() == deploy_config.addresses_name.len(),
                    "Modules path and addresses name must have the same length"
                );

                deploy_contracts(deploy_config).await
            }
        },
    }
}
