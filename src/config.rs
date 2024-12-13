use anyhow::Context;
use anyhow::Result;
use console::style;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::path::PathBuf;
use url::Url;
use selene_helius_sdk::{
    client::HeliusClient,
    websocket::WebsocketConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SolanaConfig {
    json_rpc_url: String,
    websocket_url: String,
    keypair_path: String,
    address_labels: std::collections::HashMap<String, String>,
    commitment: String,
}

impl SolanaConfig {
    pub fn get_api_key(&self) -> String {
        self.json_rpc_url
            .split("api-key=")
            .collect::<Vec<&str>>()
            .get(1)
            .unwrap()
            .to_string()
    }
}

fn get_config_file() -> Result<PathBuf> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    Ok(home_dir
        .join(".config")
        .join("solana")
        .join("cli")
        .join("config.yml"))
}

pub fn read_solana_config() -> Result<SolanaConfig> {
    let config_path = get_config_file()?;
    let config_file = std::fs::File::open(&config_path)
        .with_context(|| format!("Failed to open config file at {:?}", config_path))?;
    let config: SolanaConfig =
        serde_yaml::from_reader(config_file).with_context(|| "Failed to parse config file")?;
    Ok(config)
}

pub fn get_rpc_client() -> Result<RpcClient> {
    let config = read_solana_config()?;
    Ok(RpcClient::new(config.json_rpc_url))
}

pub fn get_helius_client() -> Result<HeliusClient> {
    let config = read_solana_config()?;
    let api_key = config.get_api_key();
    Ok(HeliusClient::new(&api_key))
}

pub fn get_websocket_config() -> Result<WebsocketConfig> {
    let config = read_solana_config()?;
    Ok(WebsocketConfig::new(&config.get_api_key()))
}

pub fn show_config() -> anyhow::Result<()> {
    let config = crate::config::read_solana_config()?;

    println!("{}", style("Current Configuration:").cyan().bold());
    println!(
        "{} RPC URL: {}",
        style("🌐").bold(),
        style(&config.json_rpc_url).yellow()
    );
    println!(
        "{} Keypair Path: {}",
        style("🔑").bold(),
        style(&config.keypair_path).yellow()
    );
    println!(
        "{} Commitment: {}",
        style("📌").bold(),
        style(&config.commitment).yellow()
    );

    Ok(())
}
