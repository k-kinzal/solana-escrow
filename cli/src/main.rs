mod config;

use crate::config::Config;
use clap::{Parser, Subcommand};
use escrow_client::Client;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

/// Cli is a struct that represents the command line arguments.
#[derive(Parser)]
struct Cli {
    /// Path to the configuration file.
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Escrow program ID.
    #[arg(long)]
    escrow_program_id: Option<Pubkey>,

    /// Token program ID.
    #[arg(long)]
    token_program_id: Option<Pubkey>,

    /// Subcommands for the CLI.
    #[command(subcommand)]
    command: Commands,
}

/// Commands is an enum that represents the subcommands for the CLI.
#[derive(Subcommand, PartialEq, Eq, Debug)]
enum Commands {
    #[clap(about = "Initialize escrow agent")]
    #[clap(arg_required_else_help = true)]
    Init {
        #[clap(help = "Address of mint token to be sent")]
        send_mint_token_address: Pubkey,
        #[clap(help = "Amount of mint token to be sent")]
        send_amount: u64,
        #[clap(help = "Address of mint token to be received")]
        receive_mint_token_address: Pubkey,
        #[clap(help = "Expected amount of mint token to be received")]
        receive_expected_amount: u64,
    },
    #[clap(about = "Exchange tokens between parties")]
    #[clap(arg_required_else_help = true)]
    Exchange {
        #[clap(help = "Address of escrow account")]
        escrow_address: Pubkey,
    },
    #[clap(about = "Get account details of escrow account")]
    #[clap(arg_required_else_help = true)]
    Account {
        #[clap(help = "Address of escrow account")]
        escrow_address: Pubkey,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    let path = args
        .config
        .or_else(|| {
            env::var("HOME").ok().map(|v| {
                Path::new(&v)
                    .join(".config")
                    .join("solana")
                    .join("cli")
                    .join("config.yml")
            })
        })
        .unwrap();
    let config = Config::load(path)?;

    let keypair = config.load_keypair()?;
    let json_rpc_url = config.json_rpc_url().to_string();
    let commitment_config = CommitmentConfig::from_str(config.commitment())?;
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        json_rpc_url,
        commitment_config,
    ));
    let mut builder = Client::builder(rpc_client.clone(), keypair);
    if let Some(token_program_id) = args.token_program_id {
        builder = builder.with_token_program_id(token_program_id);
    }
    if let Some(escrow_program_id) = args.escrow_program_id {
        builder = builder.with_escrow_program_id(escrow_program_id);
    }
    let escrow = builder.build();

    match args.command {
        Commands::Init {
            send_mint_token_address,
            send_amount,
            receive_mint_token_address,
            receive_expected_amount,
        } => {
            let (signature, escrow_account_pubkey) = escrow
                .init(
                    send_mint_token_address,
                    send_amount,
                    receive_mint_token_address,
                    receive_expected_amount,
                )
                .await?;

            println!("Create Account: {:?}\n", escrow_account_pubkey);
            println!("Signature: {:?}", signature);
        }
        Commands::Exchange { escrow_address } => {
            let signature = escrow.exchange(escrow_address).await?;
            println!("Signature: {:?}", signature);
        }
        Commands::Account { escrow_address } => {
            let account = escrow.account(escrow_address).await?;
            println!("Seller: {:?}", account.seller_pubkey);
            println!(
                "Seller Token Account: {:?}",
                account.seller_token_account_pubkey
            );
            println!(
                "Escrow Token Account: {:?}",
                account.temp_token_account_pubkey
            );
            println!("Expected amount: {:?}", account.amount);

            return Ok(());
        }
    }

    Ok(())
}
