use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{ensure, Result};
use clap::{CommandFactory, Parser, Subcommand};
use jayce::deploy_config::{AptosNetwork, DeployConfig, DeployModuleType};
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
        #[arg(short, long)]
        private_key: Option<String>,
        #[arg(short, long, default_value_t = DeployModuleType::Object)]
        module_type: DeployModuleType,
        #[arg(short, long, num_args = 1.., value_delimiter = ',')]
        modules_path: Option<Vec<PathBuf>>,
        #[arg(short, long, num_args = 1.., value_delimiter = ',')]
        addresses_name: Option<Vec<String>>,
        #[arg(short, long, default_value_t = AptosNetwork::Devnet)]
        network: AptosNetwork,
        #[arg(short, long, default_value = "deploy-report.json")]
        output_json: PathBuf,
        #[arg(short, long, default_value_t = false)]
        yes: bool,
        /// Sets a custom config file
        #[arg(short, long)]
        config_path: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    if args.version {
        println!(env!("APP_VERSION"));
        return Ok(());
    }
    match args.command {
        None => {
            Cli::command().print_help()?;
            Ok(())
        }
        Some(command) => {
            match command {
                Commands::Deploy {
                    private_key,
                    addresses_name,
                    network,
                    output_json,
                    yes,
                    config_path,
                    module_type,
                    modules_path,
                } => {
                    let deploy_config = if let Some(config_path) = config_path {
                        DeployConfig::from_path(config_path.to_str().unwrap())?
                    } else {
                        let private_key = private_key.expect("Missing argument private key");
                        let modules_path = modules_path.expect("Missing argument modules path");
                        let addresses_name = addresses_name.expect("Missing argument modules path");
                        ensure!(
                            modules_path.len() == addresses_name.len(),
                            "Modules path and addresses name must have the same length"
                        );
                        ensure!(
                            addresses_name.iter().collect::<HashSet<_>>().len()
                                == addresses_name.len(),
                            "Addresses name must be unique"
                        );
                        DeployConfig {
                            private_key,
                            module_type,
                            modules_path,
                            addresses_name,
                            network,
                            yes,
                            output_json,
                        }
                    };

                    deploy_contracts(&deploy_config).await
                }
            }
        }
    }
}
