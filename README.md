# Jayce CLI Tool

Jayce is a CLI tool for deploying contracts on the Aptos blockchain. It provides a convenient way to manage and deploy
smart contracts using various configurations.

## Installation

To install the Jayce CLI tool, clone the repository and build the project using Cargo:

```sh
git clone https://github.com/sota-zk-labs/jayce
cd jayce
cargo install --path .
```

## Usage

### Using CLI arguments

Here’s an example of deploying a contract using CLI arguments:

```sh
jayce deploy --private-key 0x00 --modules-path examples/contracts/navori/libs --addresses-name lib_addr
```

After running the command, the CLI tool will deploy the contracts as an `object` to the Aptos Devnet. You should see the
report file `deploy-report.json` in the current directory.

For more information on the CLI arguments, run:

```sh
jayce deploy -h
```

### Configuration File

You can also specify deployment parameters using a TOML configuration file. Below is an example configuration:

```toml
module_type = "object" # default, can be ignored
private_key = "0x00"
network = "devnet" # default, can be ignored
modules_path = [
    "examples/contracts/navori/libs",
]
addresses_name = ["lib_addr"]
```

For more information on the configuration file, see
the [deploy-contracts.toml](examples/config-files/deploy-contracts.toml) file.

**Note**: CLI arguments will override the settings in the configuration file.

To deploy contracts using the configuration file, run:

```sh
jayce deploy --config-path your_file.toml
```