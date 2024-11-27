use clap::ValueEnum;
use config::{Config as ConfigLoader, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use strum_macros::Display;

#[derive(Deserialize, Clone, Debug, PartialEq, ValueEnum, Display)]
pub enum DeployModuleType {
    Account,
    Object,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, ValueEnum, Display)]
pub enum AptosNetwork {
    Mainnet,
    Testnet,
    Devnet,
    Local,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DeployConfig {
    pub private_key: String,
    pub module_type: DeployModuleType,
    pub modules_path: Vec<PathBuf>,
    pub addresses_name: Vec<String>,
    pub network: AptosNetwork,
    pub yes: bool,
    pub output_json: PathBuf,
}

impl DeployConfig {
    pub fn from_path(path: &str) -> anyhow::Result<DeployConfig> {
        let content = ConfigLoader::builder()
            .add_source(File::new(path, FileFormat::Toml))
            .build()?;
        let args: DeployConfig = content.try_deserialize()?;

        Ok(args)
    }
}

impl AptosNetwork {
    pub fn rest_url(&self) -> &str {
        match self {
            AptosNetwork::Mainnet => {
                "https://api.mainnet.aptoslabs.com/v1"
            }
            AptosNetwork::Testnet => {
                "https://api.testnet.aptoslabs.com/v1"
            }
            AptosNetwork::Devnet => {
                "https://api.devnet.aptoslabs.com/v1"
            }
            AptosNetwork::Local => {
                panic!("Local network is not supported")
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::deploy_config::DeployConfig;

    #[test]
    fn test_read_deploy_config() {
        DeployConfig::from_path("examples/config-files/deploy-contracts.toml").unwrap();
    }
}