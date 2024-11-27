use anyhow::ensure;
use std::process::Command;

pub fn check_aptos_installed() -> anyhow::Result<()> {
    ensure!(Command::new("aptos").output().is_ok(), "Aptos CLI not found. Please install it from https://aptos.dev/en/build/cli");
    Ok(())
}
