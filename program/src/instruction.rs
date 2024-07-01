use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;

/// Instruction definition.
#[derive(BorshSerialize, BorshDeserialize, BorshSchema)]
pub enum Instruction {
    /// Initialize the escrow agent and enable the transaction.
    ///
    ///
    /// Accounts expected:
    ///
    ///   0. `[signer]` The account of the person initializing the escrow
    ///   1. `[]` The initializer's token account for the token they will receive should the trade go through
    ///   2. `[writable]` Temporary token account that should be created prior to this instruction and owned by the initializer
    ///   3. `[writable]` The escrow account, it will hold all necessary info about the trade.
    ///   4. `[]` The rent sysvar
    ///   5. `[]` The token program
    Initialize(u64),
    /// Accepts a trade
    ///
    ///
    /// Accounts expected:
    ///
    ///   0. `[signer]` The account of the person taking the trade
    ///   1. `[writable]` The taker's token account for the token they send
    ///   2. `[writable]` The taker's token account for the token they will receive should the trade go through
    ///   3. `[writable]` The PDA's temp token account to get tokens from and eventually close
    ///   4. `[writable]` The initializer's main account to send their rent fees to
    ///   5. `[writable]` The initializer's token account that will receive tokens
    ///   6. `[writable]` The escrow account holding the escrow info
    ///   7. `[]` The token program
    ///   8. `[]` The PDA account
    Exchange(u64),
}

/// Create initialization instructions for escrow.
#[allow(clippy::too_many_arguments)]
pub fn init(
    escrow_program_id: Pubkey,
    seller_account_pubkey: Pubkey,
    seller_token_account_pubkey: Pubkey,
    temp_token_account_pubkey: Pubkey,
    escrow_account_pubkey: Pubkey,
    rent_pubkey: Pubkey,
    token_program_pubkey: Pubkey,
    amount: u64,
) -> solana_program::instruction::Instruction {
    solana_program::instruction::Instruction::new_with_borsh(
        escrow_program_id,
        &Instruction::Initialize(amount),
        vec![
            AccountMeta::new(seller_account_pubkey, true),
            AccountMeta::new_readonly(seller_token_account_pubkey, false),
            AccountMeta::new(temp_token_account_pubkey, false),
            AccountMeta::new(escrow_account_pubkey, false),
            AccountMeta::new_readonly(rent_pubkey, false),
            AccountMeta::new_readonly(token_program_pubkey, false),
        ],
    )
}

/// Create exchange instructions for escrow.
#[allow(clippy::too_many_arguments)]
pub fn exchange(
    escrow_program_id: Pubkey,
    buyer_account_pubkey: Pubkey,
    buyer_send_token_account_pubkey: Pubkey,
    buyer_receive_token_account_pubkey: Pubkey,
    temp_token_account_pubkey: Pubkey,
    seller_account_pubkey: Pubkey,
    seller_token_account_pubkey: Pubkey,
    escrow_account_pubkey: Pubkey,
    token_program_pubkey: Pubkey,
    pda_account_pubkey: Pubkey,
    amount: u64,
) -> solana_program::instruction::Instruction {
    solana_program::instruction::Instruction::new_with_borsh(
        escrow_program_id,
        &Instruction::Exchange(amount),
        vec![
            AccountMeta::new(buyer_account_pubkey, true),
            AccountMeta::new(buyer_send_token_account_pubkey, false),
            AccountMeta::new(buyer_receive_token_account_pubkey, false),
            AccountMeta::new(temp_token_account_pubkey, false),
            AccountMeta::new(seller_account_pubkey, false),
            AccountMeta::new(seller_token_account_pubkey, false),
            AccountMeta::new(escrow_account_pubkey, false),
            AccountMeta::new_readonly(token_program_pubkey, false),
            AccountMeta::new_readonly(pda_account_pubkey, false),
        ],
    )
}
