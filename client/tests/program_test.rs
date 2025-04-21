mod token;

use borsh::BorshDeserialize;
use solana_faucet::faucet;
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_sdk::account::AccountSharedData;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::program_option::COption;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::signature::{EncodableKey, Keypair};
use solana_sdk::{bpf_loader_upgradeable, system_program};
use solana_test_validator::{TestValidator, TestValidatorGenesis, UpgradeableProgramInfo};
use spl_token::state::AccountState;
use std::path::PathBuf;
use std::sync::Arc;
use std::{env, fs};
use uuid::Uuid;

struct Validator {
    ledger_path: Option<PathBuf>,
    program_dir: Option<PathBuf>,
    accounts: Vec<(Pubkey, AccountSharedData)>,
}

impl Default for Validator {
    fn default() -> Self {
        Self {
            ledger_path: None,
            program_dir: None,
            accounts: vec![],
        }
    }
}

impl Validator {
    pub fn with_ledger_path(mut self, ledger_path: PathBuf) -> Self {
        self.ledger_path = Some(ledger_path);
        self
    }

    pub fn with_program_dir(mut self, program_dir: PathBuf) -> Self {
        self.program_dir = Some(program_dir);
        self
    }

    pub fn with_accounts(mut self, accounts: Vec<(Pubkey, AccountSharedData)>) -> Self {
        self.accounts.extend(accounts);
        self
    }

    fn ledger_path(&self) -> Option<PathBuf> {
        self.ledger_path.clone().or_else(|| {
            let temp_dir = env::temp_dir();
            let temp_dir = temp_dir.join(Uuid::new_v4().to_string());
            fs::create_dir(&temp_dir).map(move |_| temp_dir).ok()
        })
    }

    fn program_default_paths(&self) -> Vec<PathBuf> {
        vec![
            env::var("SBF_OUT_DIR").ok(),
            env::var("BPF_OUT_DIR").ok(),
            env::current_exe()
                .ok()
                .and_then(|path| {
                    path.ancestors()
                        .find(|ancestor| ancestor.ends_with("target"))
                        .map(|ancestor| ancestor.to_path_buf())
                })
                .map(|path| path.join("deploy"))
                .map(|path| path.to_str().unwrap().to_string()),
            self.program_dir
                .clone()
                .map(|path| path.to_str().unwrap().to_string()),
        ]
        .iter()
        .filter_map(|x| x.clone())
        .map(PathBuf::from)
        .collect::<Vec<_>>()
    }

    fn find_programs(&self) -> Vec<UpgradeableProgramInfo> {
        self.program_default_paths()
            .iter()
            .filter_map(|path| fs::read_dir(path).ok())
            .flatten()
            .filter_map(|path| path.ok())
            .filter(|entry| entry.path().is_file())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "so")
                    .unwrap_or(false)
            })
            .filter_map(|entry| {
                let keypair_file_name = format!(
                    "{}-keypair.json",
                    entry.path().file_stem().unwrap().to_str().unwrap()
                );
                let keypair_path = entry.path().parent().unwrap().join(keypair_file_name);
                if !keypair_path.exists() {
                    return None;
                }
                let program_keypair = Keypair::read_from_file(keypair_path).ok()?;
                let program_id = program_keypair.pubkey();

                Some(UpgradeableProgramInfo {
                    program_id,
                    loader: bpf_loader_upgradeable::id(),
                    upgrade_authority: Pubkey::default(),
                    program_path: entry.path(),
                })
            })
            .collect::<Vec<_>>()
    }

    pub async fn start(self) -> anyhow::Result<(TestValidator, Keypair)> {
        let faucet_keypair = Keypair::new();
        let faucet_account = AccountSharedData::new(1_000_000_000 * 100, 0, &system_program::id());
        let socket = faucet::run_local_faucet(faucet_keypair.insecure_clone(), None);

        let mut validator = &mut TestValidatorGenesis::default();
        if let Some(path) = self.ledger_path() {
            validator = validator.ledger_path(path);
        }
        Ok(validator
            .faucet_addr(Some(socket))
            .add_upgradeable_programs_with_path(&self.find_programs())
            .add_account(faucet_keypair.pubkey(), faucet_account)
            .add_accounts(self.accounts)
            .start_async()
            .await)
    }
}

#[tokio::test]
async fn test_initialize() -> anyhow::Result<()> {
    let payer = Keypair::new();
    let send_mint_token_account = Keypair::new();
    let send_associated_token_account_pubkey =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &payer.pubkey(),
            &send_mint_token_account.pubkey(),
            &spl_token::id(),
        );
    let receive_mint_token_account = Keypair::new();
    let receive_associated_token_account_pubkey =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &payer.pubkey(),
            &receive_mint_token_account.pubkey(),
            &spl_token::id(),
        );
    let (validator, _) = Validator::default()
        .with_accounts(vec![
            (
                payer.pubkey(),
                AccountSharedData::new(1_000_000_000, 0, &system_program::id()),
            ),
            (
                send_mint_token_account.pubkey(),
                token::mint_account(None, 1_000_000_000, 9, None),
            ),
            (
                send_associated_token_account_pubkey,
                token::associated_token_account(
                    send_mint_token_account.pubkey(),
                    payer.pubkey(),
                    100,
                    None,
                    AccountState::Initialized,
                    None,
                    0,
                    None,
                ),
            ),
            (
                receive_mint_token_account.pubkey(),
                token::mint_account(None, 1_000_000_000, 9, None),
            ),
            (
                receive_associated_token_account_pubkey,
                token::associated_token_account(
                    receive_mint_token_account.pubkey(),
                    payer.pubkey(),
                    0,
                    None,
                    AccountState::Initialized,
                    None,
                    0,
                    None,
                ),
            ),
        ])
        .start()
        .await?;

    let client = Arc::new(validator.get_async_rpc_client());
    let escrow = escrow_client::Client::builder(client.clone(), payer.insecure_clone())
        .with_rpc_send_transaction_config(RpcSendTransactionConfig {
            skip_preflight: true,
            preflight_commitment: Some(CommitmentLevel::Processed),
            ..RpcSendTransactionConfig::default()
        })
        .with_escrow_program_id(escrow_program::id())
        .with_token_program_id(spl_token::id())
        .build();

    let (_, escrow_account_pubkey) = escrow
        .init(
            send_mint_token_account.pubkey(),
            100,
            receive_mint_token_account.pubkey(),
            100,
        )
        .await?;

    let send_associated_token_account = client
        .get_account(&send_associated_token_account_pubkey)
        .await?;
    let send_associated_token_account_data =
        spl_token::state::Account::unpack(&send_associated_token_account.data)?;
    assert_eq!(
        send_associated_token_account_data.mint,
        send_mint_token_account.pubkey()
    );
    assert_eq!(send_associated_token_account_data.owner, payer.pubkey());
    assert_eq!(send_associated_token_account_data.amount, 0);
    assert_eq!(
        send_associated_token_account_data.state,
        AccountState::Initialized
    );
    assert_eq!(send_associated_token_account_data.delegate, COption::None);
    assert_eq!(send_associated_token_account_data.delegated_amount, 0);
    assert_eq!(send_associated_token_account_data.is_native, COption::None);
    assert_eq!(
        send_associated_token_account_data.close_authority,
        COption::None
    );

    let receive_associated_token_account = client
        .get_account(&receive_associated_token_account_pubkey)
        .await?;
    let receive_associated_token_account_data =
        spl_token::state::Account::unpack(&receive_associated_token_account.data)?;
    assert_eq!(
        receive_associated_token_account_data.mint,
        receive_mint_token_account.pubkey()
    );
    assert_eq!(receive_associated_token_account_data.owner, payer.pubkey());
    assert_eq!(receive_associated_token_account_data.amount, 0);
    assert_eq!(
        receive_associated_token_account_data.state,
        AccountState::Initialized
    );
    assert_eq!(
        receive_associated_token_account_data.delegate,
        COption::None
    );
    assert_eq!(receive_associated_token_account_data.delegated_amount, 0);
    assert_eq!(
        receive_associated_token_account_data.is_native,
        COption::None
    );
    assert_eq!(
        receive_associated_token_account_data.close_authority,
        COption::None
    );

    let escrow_account = client.get_account(&escrow_account_pubkey).await?;
    let escrow_account_data = escrow_program::state::Escrow::try_from_slice(&escrow_account.data)?;
    assert_eq!(escrow_account_data.is_initialized, true);
    assert_eq!(escrow_account_data.seller_pubkey, payer.pubkey());
    assert_eq!(
        escrow_account_data.seller_token_account_pubkey,
        receive_associated_token_account_pubkey
    );
    assert_eq!(escrow_account_data.amount, 100);

    let temp_token_account = client
        .get_account(&escrow_account_data.temp_token_account_pubkey)
        .await?;
    let temp_token_account_data = spl_token::state::Account::unpack(&temp_token_account.data)?;
    let (pda, _) = Pubkey::find_program_address(&[b"escrow"], &escrow_program::id());
    assert_eq!(
        temp_token_account_data.mint,
        send_mint_token_account.pubkey()
    );
    assert_eq!(temp_token_account_data.owner, pda);
    assert_eq!(temp_token_account_data.amount, 100);
    assert_eq!(temp_token_account_data.state, AccountState::Initialized);
    assert_eq!(temp_token_account_data.delegate, COption::None);
    assert_eq!(temp_token_account_data.delegated_amount, 0);
    assert_eq!(temp_token_account_data.is_native, COption::None);
    assert_eq!(temp_token_account_data.close_authority, COption::None);

    Ok(())
}
