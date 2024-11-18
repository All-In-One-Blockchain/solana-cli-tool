use anyhow::Result;
use clap::Parser;
use console::{style, Emoji};
use solana_sdk::signature::{Keypair, Signer};

#[derive(Parser, Debug)]
pub struct NewWalletArgs {
    /// the keypair file name and save to ~/.config/solana/ if not specific name use public key as name
    #[clap(short, long)]
    keypair_file: Option<String>,
}

pub fn create_new_wallet(args: &NewWalletArgs) -> Result<()> {
    let keypair = Keypair::new();
    let pubkey = keypair.pubkey();
    // 使用不同颜色和样式
    println!(
        "{} {}",
        style("Your new wallet address is:").cyan().bold(),
        style(pubkey).green()
    );

    // 使用 emoji
    static WALLET: Emoji<'_, '_> = Emoji("💳 ", "");
    static SAVE: Emoji<'_, '_> = Emoji("💾 ", "");

    let keypair_file = args
        .keypair_file
        .clone()
        .unwrap_or_else(|| format!("{}.json", pubkey));

    // Get home directory and construct the full path
    let home_dir = dirs::home_dir().expect("Could not find home directory");
    let keypair_path = home_dir.join(".config").join("solana").join(&keypair_file);

    println!("{} Created new wallet", WALLET);
    println!(
        "{} Saving keypair to {}",
        SAVE,
        style(keypair_path.display()).yellow()
    );

    // Create directories if they don't exist
    std::fs::create_dir_all(keypair_path.parent().unwrap())?;

    write_keypair_file(&keypair, &keypair_path.to_str().unwrap())?;
    // 可以添加成功标记
    println!(
        "{} {}",
        style("✔").green(),
        style("Wallet created successfully!").green().bold()
    );
    Ok(())
}

pub fn write_keypair_file(keypair: &Keypair, filename: &str) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(filename)?;
    let secret_key_str = keypair.to_base58_string();
    // Create JSON structure
    let json_content = format!(r#"[{secret_key_str}]"#);
    file.write_all(&json_content.as_bytes())?;
    Ok(())
}
