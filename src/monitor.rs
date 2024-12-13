use crate::config::{get_rpc_client, get_helius_client, get_websocket_config};
use anyhow::Result;
use chrono::{DateTime, Local, Utc};
use clap::Parser;
use console::{style, Emoji};
use selene_helius_sdk::{
    client::HeliusClient,
    websocket::{WebsocketClient, WebsocketMessage, WebsocketConfig},
    types::AccountInfo,
};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status_client_types::UiTransactionEncoding;
use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};
use tokio::sync::{broadcast, Mutex};
use tokio::time::{sleep, Instant};
use url::Url;

#[derive(Parser, Debug)]
pub struct MonitorArgs {
    #[clap(long)]
    addresses: Vec<String>, // 要监控的地址列表

    #[clap(long, default_value = "10")]
    interval: u64, // 检查间隔(秒)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitorEvent {
    BalanceChange {
        address: String,
        old_balance: f64,
        new_balance: f64,
        timestamp: String,
    },
    NewTransaction {
        address: String,
        signature: String,
        timestamp: String,
        status: String,
    },
    TokenBalanceChange {
        address: String,
        token_address: String,
        old_balance: f64,
        new_balance: f64,
        timestamp: String,
    },
}

#[derive(Clone)]
pub struct Monitor {
    balance_cache: Arc<Mutex<HashMap<String, f64>>>,
    token_balance_cache: Arc<Mutex<HashMap<(String, String), f64>>>,
    tx_signature_cache: Arc<Mutex<HashMap<String, Vec<Signature>>>>,
    event_sender: broadcast::Sender<MonitorEvent>,
    helius_client: HeliusClient,
    websocket_config: WebsocketConfig,
}

impl Monitor {
    pub fn new() -> Result<(Self, broadcast::Receiver<MonitorEvent>)> {
        let (tx, rx) = broadcast::channel(100);
        let helius_client = get_helius_client()?;
        let websocket_config = get_websocket_config()?;

        Ok((
            Self {
                balance_cache: Arc::new(Mutex::new(HashMap::new())),
                token_balance_cache: Arc::new(Mutex::new(HashMap::new())),
                tx_signature_cache: Arc::new(Mutex::new(HashMap::new())),
                event_sender: tx,
                helius_client,
                websocket_config,
            },
            rx,
        ))
    }

    async fn start_websocket_monitor(&self, addresses: Vec<String>) -> Result<()> {
        let mut ws_client = WebsocketClient::connect(&self.websocket_config).await?;

        // Subscribe to account updates for all addresses
        for address in &addresses {
            let pubkey = Pubkey::from_str(address)?;
            ws_client.subscribe_account(&pubkey).await?;
        }

        while let Ok(msg) = ws_client.receive_message().await {
            match msg {
                WebsocketMessage::AccountUpdate(account_info) => {
                    let address = account_info.pubkey.to_string();
                    let new_balance = account_info.lamports as f64 / 1e9;

                    let mut cache = self.balance_cache.lock().await;
                    let old_balance = *cache.get(&address).unwrap_or(&0.0);

                    if (new_balance - old_balance).abs() > 0.000001 {
                        let event = MonitorEvent::BalanceChange {
                            address: address.clone(),
                            old_balance,
                            new_balance,
                            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                        };

                        if let Err(e) = self.event_sender.send(event) {
                            println!("Failed to send balance change event: {:?}", e);
                        }
                        cache.insert(address, new_balance);
                    }
                }
                WebsocketMessage::Transaction(transaction) => {
                    for address in &addresses {
                        if transaction.involved_accounts.contains(&Pubkey::from_str(address)?) {
                            let signature = transaction.signature.to_string();
                            let mut cache = self.tx_signature_cache.lock().await;
                            let known_signatures = cache.entry(address.clone()).or_insert_with(Vec::new);

                            if !known_signatures.contains(&transaction.signature) {
                                let status = if transaction.successful { "Success" } else { "Failed" };
                                let event = MonitorEvent::NewTransaction {
                                    address: address.clone(),
                                    signature,
                                    timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                                    status: status.to_string(),
                                };

                                if let Err(e) = self.event_sender.send(event) {
                                    println!("Failed to send transaction event: {:?}", e);
                                }
                                known_signatures.push(transaction.signature);

                                if known_signatures.len() > 50 {
                                    known_signatures.drain(0..known_signatures.len() - 50);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn monitor_token_balance(&self, wallet_address: &str, token_address: &str) -> Result<()> {
        static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍 ", "");
        static WALLET: Emoji<'_, '_> = Emoji("👛 ", "");
        static TOKEN: Emoji<'_, '_> = Emoji("🪙 ", "");
        static WARNING: Emoji<'_, '_> = Emoji("⚠️ ", "");
        static CHECK: Emoji<'_, '_> = Emoji("✅ ", "");
        static ALERT: Emoji<'_, '_> = Emoji("🔔 ", "");

        println!("{}", style("Token Balance Monitor").bold().bright().cyan());
        println!(
            "{}{}: {}",
            WALLET,
            style("Wallet").dim(),
            style(wallet_address).green()
        );
        println!(
            "{}{}: {}",
            TOKEN,
            style("Token").dim(),
            style(token_address).green()
        );
        println!();

        let rpc_client = match get_rpc_client() {
            Ok(client) => client,
            Err(e) => {
                println!(
                    "{} {}",
                    style("ERROR:").red().bold(),
                    style(format!("Failed to get RPC client: {:?}", e)).red()
                );
                return Err(e);
            }
        };

        let wallet_pubkey = match Pubkey::from_str(wallet_address) {
            Ok(pubkey) => pubkey,
            Err(e) => {
                println!(
                    "{} {}",
                    style("ERROR:").red().bold(),
                    style(format!(
                        "Invalid wallet address {}: {:?}",
                        wallet_address, e
                    ))
                    .red()
                );
                return Err(e.into());
            }
        };

        let token_pubkey = match Pubkey::from_str(token_address) {
            Ok(pubkey) => pubkey,
            Err(e) => {
                println!(
                    "{} {}",
                    style("ERROR:").red().bold(),
                    style(format!("Invalid token address {}: {:?}", token_address, e)).red()
                );
                return Err(e.into());
            }
        };

        let mut last_print_time = Instant::now();
        let print_interval = Duration::from_secs(60);

        loop {
            let filter = solana_client::rpc_request::TokenAccountsFilter::Mint(token_pubkey);

            match rpc_client
                .get_token_accounts_by_owner(&wallet_pubkey, filter)
                .await
            {
                Ok(accounts) => {
                    if accounts.is_empty() {
                        if last_print_time.elapsed() >= print_interval {
                            println!(
                                "{}{}",
                                WARNING,
                                style("No token accounts found. Wallet might not hold this token.")
                                    .yellow()
                            );
                            last_print_time = Instant::now();
                        }
                        continue;
                    }

                    for account in accounts {
                        match rpc_client
                            .get_token_account_balance(&Pubkey::from_str(&account.pubkey)?)
                            .await
                        {
                            Ok(balance) => {
                                let new_balance = balance.ui_amount.unwrap_or_default();
                                let mut cache = self.token_balance_cache.lock().await;
                                let key = (wallet_address.to_string(), token_address.to_string());
                                let old_balance = *cache.get(&key).unwrap_or(&0.0);

                                if (new_balance - old_balance).abs() > 0.000001 {
                                    println!(
                                        "\n{} {}",
                                        ALERT,
                                        style("Balance Change Detected").bold().yellow()
                                    );
                                    println!("{}", style("─".repeat(50)).dim());
                                    println!(
                                        "{}{}: {}",
                                        LOOKING_GLASS,
                                        style("Account").dim(),
                                        style(&account.pubkey).cyan()
                                    );

                                    // 计算余额变化
                                    let change = new_balance - old_balance;
                                    let change_str = format!("{:+.6}", change);
                                    let change_style = if change > 0.0 {
                                        style(change_str).green()
                                    } else {
                                        style(change_str).red()
                                    };

                                    println!(
                                        "Old Balance: {}",
                                        style(format!("{:.6}", old_balance)).red()
                                    );
                                    println!(
                                        "New Balance: {}",
                                        style(format!("{:.6}", new_balance)).green()
                                    );
                                    println!("Change:      {}", change_style);
                                    println!("{}", style("─".repeat(50)).dim());

                                    let event = MonitorEvent::TokenBalanceChange {
                                        address: wallet_address.to_string(),
                                        token_address: token_address.to_string(),
                                        old_balance,
                                        new_balance,
                                        timestamp: Local::now()
                                            .format("%Y-%m-%d %H:%M:%S")
                                            .to_string(),
                                    };

                                    if let Err(e) = self.event_sender.send(event) {
                                        println!(
                                            "{} {}",
                                            style("ERROR:").red().bold(),
                                            style(format!("Failed to send event: {:?}", e)).red()
                                        );
                                    }

                                    cache.insert(key, new_balance);
                                } else if last_print_time.elapsed() >= print_interval {
                                    println!(
                                        "{} {} {} {}",
                                        CHECK,
                                        style("Balance").dim(),
                                        style(format!("{:.6}", new_balance)).cyan(),
                                        style(Local::now().format("%H:%M:%S")).dim()
                                    );
                                    last_print_time = Instant::now();
                                }
                            }
                            Err(e) => {
                                println!(
                                    "{} {}",
                                    style("ERROR:").red().bold(),
                                    style(format!(
                                        "Error getting balance for account {}: {:?}",
                                        account.pubkey, e
                                    ))
                                    .red()
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    println!(
                        "{} {}",
                        style("ERROR:").red().bold(),
                        style(format!("Error getting token accounts: {:?}", e)).red()
                    );
                }
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}

pub async fn run_monitor(args: &MonitorArgs) -> Result<()> {
    let (monitor, mut rx) = Monitor::new()?;

    // Start event handler
    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            match event {
                MonitorEvent::BalanceChange {
                    address,
                    old_balance,
                    new_balance,
                    timestamp,
                } => {
                    println!("\n{}", style("Balance Change Detected:").bold().cyan());
                    println!("Address: {}", style(&address).yellow());
                    println!("Old Balance: {} SOL", style(old_balance).yellow());
                    println!("New Balance: {} SOL", style(new_balance).yellow());
                    println!("Time: {}", style(timestamp).yellow());
                }
                MonitorEvent::NewTransaction {
                    address,
                    signature,
                    timestamp,
                    status,
                } => {
                    println!("\n{}", style("New Transaction Detected:").bold().cyan());
                    println!("Address: {}", style(&address).yellow());
                    println!("Signature: {}", style(&signature).yellow());
                    println!("Status: {}", style(&status).yellow());
                    println!("Time: {}", style(timestamp).yellow());
                }
                MonitorEvent::TokenBalanceChange {
                    address,
                    token_address,
                    old_balance,
                    new_balance,
                    timestamp,
                } => {
                    println!(
                        "\n{}",
                        style("Token Balance Change Detected:").bold().cyan()
                    );
                    println!("Wallet: {}", style(&address).yellow());
                    println!("Token: {}", style(&token_address).yellow());
                    println!("Old Balance: {}", style(old_balance).yellow());
                    println!("New Balance: {}", style(new_balance).yellow());
                    println!("Time: {}", style(timestamp).yellow());
                }
            }
        }
    });

    let mut handles = vec![];

    // Start websocket monitor for all addresses
    {
        let monitor = monitor.clone();
        let addresses = args.addresses.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = monitor.start_websocket_monitor(addresses).await {
                println!("Websocket monitor error: {:?}", e);
            }
        });
        handles.push(handle);
    }

    // Start token monitoring tasks (keeping existing implementation)
    for address in &args.addresses {
        let token_addresses = vec![
            "GJAFwWjJ3vnTsrQVabjBVK2TYB1YtRCQXRDfDgUnpump", // Example token address
        ];

        for token_address in token_addresses {
            let monitor = monitor.clone();
            let address = address.clone();
            let token_address = token_address.to_string();
            let interval = args.interval;

            let handle = tokio::spawn(async move {
                loop {
                    if let Err(e) = monitor.monitor_token_balance(&address, &token_address).await {
                        println!("Token monitor error for {}: {:?}", token_address, e);
                    }
                    sleep(Duration::from_secs(interval)).await;
                }
            });
            handles.push(handle);
        }
    }

    futures::future::join_all(handles).await;
    Ok(())
}
