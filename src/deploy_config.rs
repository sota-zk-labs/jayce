use std::collections::BTreeMap;
use clap::ValueEnum;
use config::{Config as ConfigLoader, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use aptos_sdk::move_types::account_address::AccountAddress;
use strum_macros::Display;

#[derive(Deserialize, Clone, Debug, PartialEq, ValueEnum, Display)]
#[strum(serialize_all = "snake_case")]
pub enum DeployModuleType {
    Account,
    Object,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, ValueEnum, Display)]
#[strum(serialize_all = "snake_case")]
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
    pub deployed_addresses: BTreeMap<String, AccountAddress>,
    pub rpc_url: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PartialDeployConfig {
    pub private_key: Option<String>,
    pub module_type: Option<DeployModuleType>,
    pub modules_path: Option<Vec<PathBuf>>,
    pub addresses_name: Option<Vec<String>>,
    pub network: Option<AptosNetwork>,
    pub yes: Option<bool>,
    pub output_json: Option<PathBuf>,
    pub deployed_addresses: Option<BTreeMap<String, AccountAddress>>,
    pub rpc_url: Option<String>,
}

impl PartialDeployConfig {
    pub fn from_path(path: &str) -> anyhow::Result<PartialDeployConfig> {
        let content = ConfigLoader::builder()
            .add_source(File::new(path, FileFormat::Toml))
            .build()?;
        let args: PartialDeployConfig = content.try_deserialize()?;

        Ok(args)
    }
}

impl From<PartialDeployConfig> for DeployConfig {
    fn from(value: PartialDeployConfig) -> Self {
        DeployConfig {
            private_key: value.private_key.expect("Missing argument 'private-key'"),
            module_type: value.module_type.expect("Missing argument 'module type'"),
            modules_path: value.modules_path.expect("Missing argument 'modules-path'"),
            addresses_name: value.addresses_name.expect("Missing argument 'addresses-name'"),
            network: value.network.expect("Missing argument 'network'"),
            yes: value.yes.expect("Missing argument 'yes'"),
            output_json: value.output_json.expect("Missing argument 'output-json'"),
            deployed_addresses: value.deployed_addresses.expect("Missing argument 'deployed-addresses'"),
            rpc_url: value.rpc_url,
        }
    }
}

impl AptosNetwork {
    pub fn rpc_url(&self) -> Option<&str> {
        match self {
            AptosNetwork::Mainnet => {
                Some("https://api.mainnet.aptoslabs.com/v1")
            }
            AptosNetwork::Testnet => {
                Some("https://api.testnet.aptoslabs.com/v1")
            }
            AptosNetwork::Devnet => {
                Some("https://api.devnet.aptoslabs.com/v1")
            }
            AptosNetwork::Local => {
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::deploy_config::PartialDeployConfig;

    #[test]
    fn test_read_deploy_config() {
        let x = PartialDeployConfig::from_path("examples/config-files/deploy-contracts.toml").unwrap();
        dbg!(x);
    }
}