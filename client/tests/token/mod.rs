use solana_sdk::account::AccountSharedData;
use solana_sdk::program_option::COption;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use spl_token::state::AccountState;

pub fn mint_account(
    mint_authority: Option<Pubkey>,
    supply: u64,
    decimals: u8,
    freeze_authority: Option<Pubkey>,
) -> AccountSharedData {
    let mut account =
        AccountSharedData::new(1_000_000_000, spl_token::state::Mint::LEN, &spl_token::id());
    let mint = spl_token::state::Mint {
        mint_authority: COption::from(mint_authority),
        supply,
        decimals,
        is_initialized: true,
        freeze_authority: COption::from(freeze_authority),
    };
    let data = &mut [0; spl_token::state::Mint::LEN];
    mint.pack_into_slice(data);
    account.set_data_from_slice(data);

    account
}

pub fn associated_token_account(
    mint: Pubkey,
    owner: Pubkey,
    amount: u64,
    delegate: Option<Pubkey>,
    state: AccountState,
    is_native: Option<u64>,
    delegated_amount: u64,
    close_authority: Option<Pubkey>,
) -> AccountSharedData {
    let mut account = AccountSharedData::new(
        1_000_000_000,
        spl_token::state::Account::LEN,
        &spl_token::id(),
    );
    let token_account = spl_token::state::Account {
        mint,
        owner,
        amount,
        delegate: COption::from(delegate),
        state,
        is_native: COption::from(is_native),
        delegated_amount,
        close_authority: COption::from(close_authority),
    };
    let data = &mut [0; spl_token::state::Account::LEN];
    token_account.pack_into_slice(data);
    account.set_data_from_slice(data);

    account
}

// use std::sync::Arc;
// use solana_client::nonblocking::rpc_client::RpcClient;
// use solana_keypair::Keypair;
// use solana_sdk::pubkey::Pubkey;
// use solana_sdk::signature::Signature;
// use solana_sdk::signer::Signer;
// use solana_sdk::system_instruction;
// use solana_sdk::transaction::Transaction;
// use spl_token::solana_program;
// use spl_token::solana_program::program_pack::Pack;
//
// #[derive(Debug, thiserror::Error)]
// pub enum ClientError {
//     #[error("{0}")]
//     RpcError(#[from] solana_rpc_client_api::client_error::Error),
//     #[error("{0}")]
//     ProgramError(#[from] solana_program::program_error::ProgramError),
// }
//
// type Result<T> = std::result::Result<T, ClientError>;
//
// pub struct Client {
//     /// RPC client.
//     client: Arc<RpcClient>,
//
//     /// Keypair of the payer.
//     payer: Keypair,
//
//     /// Token program ID.
//     token_program_id: Pubkey,
// }
//
// impl Client {
//     pub fn builder(client: Arc<RpcClient>, payer: Keypair) -> ClientBuilder {
//         ClientBuilder::new(client, payer)
//     }
//
//     pub async fn initialize_mint(
//         &self,
//         mint_account: Keypair,
//         mint_authority_pubkey: &Pubkey,
//         freeze_authority_pubkey: Option<&Pubkey>,
//         decimals: u8,
//     ) -> Result<Signature> {
//         let blockhash = self.client.get_latest_blockhash().await?;
//
//         let mint_account_len = spl_token::state::Mint::LEN;
//         let mint_account_lamports = self
//             .client
//             .get_minimum_balance_for_rent_exemption(mint_account_len)
//             .await?;
//
//         let tx = Transaction::new_signed_with_payer(
//             &[
//                 system_instruction::create_account(
//                     &self.payer.pubkey(),
//                     &mint_account.pubkey(),
//                     mint_account_lamports,
//                     mint_account_len as u64,
//                     &spl_token::id(),
//                 ),
//                 spl_token::instruction::initialize_mint(
//                     &self.token_program_id,
//                     &mint_account.pubkey(),
//                     mint_authority_pubkey,
//                     freeze_authority_pubkey,
//                     decimals
//                 )?
//             ],
//             Some(&self.payer.pubkey()),
//             &[&self.payer, &mint_account],
//             blockhash
//         );
//         Ok(self.client.send_transaction(&tx).await?)
//     }
//
//     pub async fn mint_to(
//         &self,
//         mint_pubkey: &Pubkey,
//         account_pubkey: &Pubkey,
//         amount: u64,
//     ) -> Result<Signature> {
//         let blockhash = self.client.get_latest_blockhash().await?;
//
//         let tx = Transaction::new_signed_with_payer(
//             &[
//                 spl_token::instruction::mint_to(
//                     &self.token_program_id,
//                     mint_pubkey,
//                     account_pubkey,
//                     &self.payer.pubkey(),
//                     &[],
//                     amount,
//                 )?
//             ],
//             Some(&self.payer.pubkey()),
//             &[&self.payer],
//             blockhash
//         );
//         Ok(self.client.send_transaction(&tx).await?)
//     }
//
//     pub async fn initialize_account(
//         &self,
//         mint_pubkey: &Pubkey,
//     ) -> Result<Signature> {
//         let blockhash = self.client.get_latest_blockhash().await?;
//
//         let account = Keypair::new();
//
//         let account_len = spl_token::state::Account::LEN;
//         let account_lamports = self
//             .client
//             .get_minimum_balance_for_rent_exemption(account_len)
//             .await?;
//
//         let tx = Transaction::new_signed_with_payer(
//             &[
//                 system_instruction::create_account(
//                     &self.payer.pubkey(),
//                     &account.pubkey(),
//                     account_lamports,
//                     account_len as u64,
//                     &spl_token::id(),
//                 ),
//                 spl_token::instruction::initialize_account(
//                     &self.token_program_id,
//                     &account.pubkey(),
//                     mint_pubkey,
//                     &self.payer.pubkey(),
//                 )?
//             ],
//             Some(&self.payer.pubkey()),
//             &[&self.payer, &account],
//             blockhash
//         );
//         Ok(self.client.send_transaction(&tx).await?)
//     }
// }
//
// pub struct ClientBuilder {
//     /// RPC client.
//     client: Arc<RpcClient>,
//
//     /// Keypair of the payer.
//     payer: Keypair,
//
//     /// Token program ID.
//     token_program_id: Option<Pubkey>,
// }
//
// impl ClientBuilder {
//     fn new(client: Arc<RpcClient>, payer: Keypair) -> Self {
//         Self {
//             client,
//             payer,
//             token_program_id: None,
//         }
//     }
//
//     pub fn with_token_program_id(mut self, token_program_id: Pubkey) -> Self {
//         self.token_program_id = Some(token_program_id);
//         self
//     }
//
//     /// Build the client for interacting with the token program.
//     pub fn build(self) -> Client {
//         Client {
//             client: self.client,
//             payer: self.payer,
//             token_program_id: self.token_program_id.unwrap_or_else(spl_token::id),
//         }
//     }
// }
//
