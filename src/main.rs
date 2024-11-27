use std::env;
use std::io;
use std::process;
use std::sync::Arc;

use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use tokio::sync::Mutex;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

mod app;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    
    // Function to select RPC URL
    fn select_rpc_url(flag: &str) -> String {
        match flag {
            "-D" => "https://rpc.devnet.soo.network/rpc".to_string(),
            "-T" => "https://rpc.testnet.soo.network/rpc".to_string(),
            "-M" => "https://api.mainnet-beta.solana.com".to_string(),
            _ => "https://api.mainnet-beta.solana.com".to_string(), // default to mainnet
        }
    }

    // Determine action based on arguments
    match args.len() {
        1 => {
            // No arguments - run TUI
            run_tui().await
        },
        2 => {
            // Check if first arg is a flag or transaction
            if ["-D", "-T", "-M"].contains(&args[1].as_str()) {
                println!("Error: Transaction hash is required when using RPC flag");
                println!("Usage: {} [flag] <transaction_signature>", args[0]);
                println!("Flags: -D (devnet), -T (testnet), -M (mainnet)");
                run_tui().await
            } else {
                // Assume it's a transaction signature on mainnet
                let rpc_url = "https://api.mainnet-beta.solana.com".to_string();
                check_transaction(rpc_url, &args[1]).await
            }
        },
        3 => {
            // RPC flag and transaction signature
            let rpc_url = select_rpc_url(&args[1]);
            check_transaction(rpc_url, &args[2]).await
        },
        _ => {
            println!("Too many arguments");
            println!("Usage: {} [flag] <transaction_signature>", args[0]);
            println!("Flags: -D (devnet), -T (testnet), -M (mainnet)");
            run_tui().await
        }
    }
}

// Separate function to check transaction status
async fn check_transaction(rpc_url: String, signature_str: &str) -> io::Result<()> {
    // Parse the transaction signature
    let signature = match signature_str.parse::<Signature>() {
        Ok(sig) => sig,
        Err(_) => {
            eprintln!("Invalid transaction signature format");
            process::exit(1);
        }
    };

    // Create RPC client
    let client = RpcClient::new(rpc_url.clone());

    // Fetch transaction statuses
    match client.get_signature_statuses_with_history(&[signature]) {
        Ok(response) => {
            println!("Using RPC: {}", rpc_url);
            if let Some(status) = response.value.first() {
                match status {
                    Some(tx_status) => {
                        println!("Transaction Status Details:");
                        println!("Slot: {}", tx_status.slot);
                        println!("Confirmations: {:?}", tx_status.confirmations);
                        println!("Confirmation Status: {:?}", tx_status.confirmation_status);
                        
                        // Check for transaction success
                        if tx_status.status.is_ok() {
                            println!("Transaction Status: Successful ✅");
                        } else {
                            println!("Transaction Status: Failed ❌");
                            if let Some(err) = &tx_status.err {
                                println!("Error: {:?}", err);
                            }
                        }
                    },
                    None => {
                        println!("Transaction not found or does not exist");
                    }
                }
            } else {
                println!("No status information available");
            }
        },
        Err(e) => {
            eprintln!("Error fetching transaction status: {}", e);
            process::exit(1);
        }
    }

    Ok(())
}

// Separate function to run TUI
async fn run_tui() -> io::Result<()> {
    // Initialize terminal
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    crossterm::terminal::enable_raw_mode()?;
    
    // Create app state
    let app = Arc::new(Mutex::new(app::App::default()));
    
    // Run app
    let result = app::App::run(app, &mut terminal).await;
    
    // Cleanup
    crossterm::terminal::disable_raw_mode()?;
    terminal.clear()?;
    terminal.show_cursor()?;
    
    result
}
