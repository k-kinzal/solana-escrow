use borsh::BorshDeserialize;
use escrow_program::state::Escrow;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_sdk::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::rent::Rent;
use solana_sdk::signature::{Keypair, Signature, Signer};
use solana_sdk::system_instruction;
use solana_sdk::sysvar::SysvarId;
use solana_sdk::transaction::Transaction;
use spl_token::solana_program::program_pack::Pack;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("{0}")]
    RpcError(#[from] solana_rpc_client_api::client_error::Error),
    #[error("{0}")]
    ProgramError(#[from] ProgramError),
    #[error("{0:?}")]
    SerializeSizeError(borsh::schema::SchemaMaxSerializedSizeError),
    #[error("{0}")]
    IoError(#[from] std::io::Error),
}

impl From<borsh::schema::SchemaMaxSerializedSizeError> for ClientError {
    fn from(err: borsh::schema::SchemaMaxSerializedSizeError) -> Self {
        ClientError::SerializeSizeError(err)
    }
}

pub type Result<T> = std::result::Result<T, ClientError>;

/// Client for interacting with the escrow program.
pub struct Client {
    /// RPC client.
    client: Arc<RpcClient>,

    /// Keypair of the payer.
    payer: Keypair,

    /// Configuration for sending transactions.
    rpc_send_transaction_config: RpcSendTransactionConfig,

    /// Escrow program ID.
    escrow_program_id: Pubkey,

    /// Token program ID.
    token_program_id: Pubkey,
}

impl Client {
    pub fn builder(client: Arc<RpcClient>, payer: Keypair) -> ClientBuilder {
        ClientBuilder::new(client, payer)
    }

    /// Initialize the escrow account.
    pub async fn init(
        &self,
        send_mint_token_account_pubkey: Pubkey,
        send_amount: u64,
        receive_mint_token_account_pubkey: Pubkey,
        receive_expected_amount: u64,
    ) -> Result<(Signature, Pubkey)> {
        let send_seller_token_account_pubkey =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &self.payer.pubkey(),
                &send_mint_token_account_pubkey,
                &self.token_program_id,
            );

        let receive_seller_token_account_pubkey =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &self.payer.pubkey(),
                &receive_mint_token_account_pubkey,
                &self.token_program_id,
            );

        let temp_token_account = Keypair::new();
        let temp_token_account_len = spl_token::state::Account::LEN;
        let temp_token_account_lamports = self
            .client
            .get_minimum_balance_for_rent_exemption(temp_token_account_len)
            .await?;

        let escrow_account = Keypair::new();
        let escrow_account_len = borsh::max_serialized_size::<Escrow>()?;
        let escrow_account_lamports = self
            .client
            .get_minimum_balance_for_rent_exemption(escrow_account_len)
            .await?;

        let blockhash = self.client.get_latest_blockhash().await?;

        let tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &self.payer.pubkey(),
                    &temp_token_account.pubkey(),
                    temp_token_account_lamports,
                    temp_token_account_len as u64,
                    &self.token_program_id,
                ),
                spl_token::instruction::initialize_account(
                    &self.token_program_id,
                    &temp_token_account.pubkey(),
                    &send_mint_token_account_pubkey,
                    &self.payer.pubkey(),
                )?,
                spl_token::instruction::transfer(
                    &self.token_program_id,
                    &send_seller_token_account_pubkey,
                    &temp_token_account.pubkey(),
                    &self.payer.pubkey(),
                    &[&self.payer.pubkey()],
                    send_amount,
                )?,
                system_instruction::create_account(
                    &self.payer.pubkey(),
                    &escrow_account.pubkey(),
                    escrow_account_lamports,
                    escrow_account_len as u64,
                    &self.escrow_program_id,
                ),
                escrow_program::instruction::init(
                    self.escrow_program_id,
                    self.payer.pubkey(),
                    receive_seller_token_account_pubkey,
                    temp_token_account.pubkey(),
                    escrow_account.pubkey(),
                    Rent::id(),
                    self.token_program_id,
                    receive_expected_amount,
                ),
            ],
            Some(&self.payer.pubkey()),
            &[&self.payer, &temp_token_account, &escrow_account],
            blockhash,
        );

        let signature = self
            .client
            .send_and_confirm_transaction_with_spinner_and_config(
                &tx,
                self.client.commitment(),
                self.rpc_send_transaction_config.clone(),
            )
            .await?;

        Ok((signature, escrow_account.pubkey()))
    }

    /// Exchange the tokens in the escrow account.
    pub async fn exchange(&self, escrow_account_pubkey: Pubkey) -> Result<Signature> {
        let escrow_account = self.client.get_account(&escrow_account_pubkey).await?;
        let escrow_state = Escrow::try_from_slice(&escrow_account.data)?;

        let seller_token_account = self
            .client
            .get_account(&escrow_state.seller_token_account_pubkey)
            .await?;
        let seller_token_account_state =
            spl_token::state::Account::unpack(&seller_token_account.data)?;

        let temp_token_account = self
            .client
            .get_account(&escrow_state.temp_token_account_pubkey)
            .await?;
        let temp_token_account_state = spl_token::state::Account::unpack(&temp_token_account.data)?;

        let buyer_send_token_account_pubkey =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &self.payer.pubkey(),
                &seller_token_account_state.mint,
                &self.token_program_id,
            );

        let buyer_receive_token_account_pubkey =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &self.payer.pubkey(),
                &temp_token_account_state.mint,
                &self.token_program_id,
            );

        let (pda_account_pubkey, _) =
            Pubkey::find_program_address(&[b"escrow"], &self.escrow_program_id);

        let blockhash = self.client.get_latest_blockhash().await?;

        let tx = Transaction::new_signed_with_payer(
            &[escrow_program::instruction::exchange(
                self.escrow_program_id,
                self.payer.pubkey(),
                buyer_send_token_account_pubkey,
                buyer_receive_token_account_pubkey,
                escrow_state.temp_token_account_pubkey,
                escrow_state.seller_pubkey,
                escrow_state.seller_token_account_pubkey,
                escrow_account_pubkey,
                self.token_program_id,
                pda_account_pubkey,
                temp_token_account_state.amount,
            )],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        );

        let signature = self
            .client
            .send_and_confirm_transaction_with_spinner_and_config(
                &tx,
                self.client.commitment(),
                self.rpc_send_transaction_config.clone(),
            )
            .await?;
        Ok(signature)
    }

    /// Get the escrow account state.
    pub async fn account(&self, account_pubkey: Pubkey) -> Result<Escrow> {
        let account = self.client.get_account(&account_pubkey).await?;
        let state = Escrow::try_from_slice(&account.data)?;

        Ok(state)
    }
}

/// Builder for the client for interacting with the escrow program.
pub struct ClientBuilder {
    /// RPC client.
    client: Arc<RpcClient>,

    /// Keypair of the payer.
    payer: Keypair,

    /// Configuration for sending transactions.
    rpc_send_transaction_config: RpcSendTransactionConfig,

    /// Escrow program ID.
    /// Default is the escrow program ID.
    escrow_program_id: Option<Pubkey>,

    /// Token program ID.
    /// Default is the token program ID.
    token_program_id: Option<Pubkey>,
}

impl ClientBuilder {
    fn new(client: Arc<RpcClient>, payer: Keypair) -> Self {
        Self {
            client,
            payer,
            rpc_send_transaction_config: Default::default(),
            escrow_program_id: None,
            token_program_id: None,
        }
    }

    pub fn with_rpc_send_transaction_config(
        mut self,
        rpc_send_transaction_config: RpcSendTransactionConfig,
    ) -> Self {
        self.rpc_send_transaction_config = rpc_send_transaction_config;
        self
    }

    pub fn with_escrow_program_id(mut self, escrow_program_id: Pubkey) -> Self {
        self.escrow_program_id = Some(escrow_program_id);
        self
    }

    pub fn with_token_program_id(mut self, token_program_id: Pubkey) -> Self {
        self.token_program_id = Some(token_program_id);
        self
    }

    /// Build the client for interacting with the escrow program.
    pub fn build(self) -> Client {
        Client {
            client: self.client,
            payer: self.payer,
            rpc_send_transaction_config: self.rpc_send_transaction_config,
            escrow_program_id: self.escrow_program_id.unwrap_or_else(escrow_program::id),
            token_program_id: self.token_program_id.unwrap_or_else(spl_token::id),
        }
    }
}
