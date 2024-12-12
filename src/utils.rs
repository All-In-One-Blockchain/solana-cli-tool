use solana_sdk::signature::Keypair;

/// default accout at ~/.config/solana/id.json
pub fn default_account() -> anyhow::Result<Keypair> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let keypair_path = home_dir
        .join(".config")
        .join("solana")
        .join("id.json");

    let keypair_path_str = keypair_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid path to keypair file"))?;

    solana_sdk::signature::read_keypair_file(keypair_path_str)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair file: {}", e))
}
