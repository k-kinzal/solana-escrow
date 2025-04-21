use solana_faucet::faucet;
use solana_sdk::account::AccountSharedData;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{EncodableKey, Keypair, Signer};
use solana_sdk::{bpf_loader_upgradeable, system_program};
use solana_test_validator::{TestValidator, TestValidatorGenesis, UpgradeableProgramInfo};
use std::path::PathBuf;
use std::{env, fs};
use uuid::Uuid;

/// A struct to configure the validator for testing.
pub struct Validator {
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
    /// Set the ledger path for the validator.
    #[allow(dead_code)]
    pub fn with_ledger_path(mut self, ledger_path: PathBuf) -> Self {
        self.ledger_path = Some(ledger_path);
        self
    }

    /// Set the program directory for the validator.
    #[allow(dead_code)]
    pub fn with_program_dir(mut self, program_dir: PathBuf) -> Self {
        self.program_dir = Some(program_dir);
        self
    }

    /// Set the accounts for the validator.
    pub fn with_accounts(mut self, accounts: Vec<(Pubkey, AccountSharedData)>) -> Self {
        self.accounts.extend(accounts);
        self
    }

    /// Get the ledger path for the validator.
    fn ledger_path(&self) -> Option<PathBuf> {
        self.ledger_path.clone().or_else(|| {
            let temp_dir = env::temp_dir();
            let temp_dir = temp_dir.join(Uuid::new_v4().to_string());
            fs::create_dir(&temp_dir).map(move |_| temp_dir).ok()
        })
    }

    /// Get the program directory for the validator.
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

    /// Find all upgradeable programs in the program directory.
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

    /// Start the validator with the configured settings.
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
