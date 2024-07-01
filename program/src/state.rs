use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::program_pack::IsInitialized;
use solana_program::pubkey::Pubkey;

/// Escrow represents a state for intermediate safe transactions.
///
/// # Example
///
/// ```rust
/// # use borsh::BorshDeserialize;
/// # use escrow_program::state::Escrow;
/// #
/// let escrow = Escrow::default();
/// let serialized = borsh::to_vec(&escrow).unwrap();
/// let deserialized = borsh::from_slice::<Escrow>(&serialized).unwrap();
///
/// assert_eq!(escrow.is_initialized, deserialized.is_initialized);
/// assert_eq!(escrow.seller_pubkey, deserialized.seller_pubkey);
/// assert_eq!(escrow.seller_token_account_pubkey, deserialized.seller_token_account_pubkey);
/// assert_eq!(escrow.temp_token_account_pubkey, deserialized.temp_token_account_pubkey);
/// assert_eq!(escrow.amount, deserialized.amount);
/// ```
#[derive(Default, BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct Escrow {
    /// If true, state has been initialized
    pub is_initialized: bool,

    /// Seller's public key
    pub seller_pubkey: Pubkey,

    /// Token account to be received by the seller at the conclusion of the transaction
    pub seller_token_account_pubkey: Pubkey,

    /// Token account temporarily deposited in escrow by the seller
    pub temp_token_account_pubkey: Pubkey,

    /// Amount of tokens expected by the seller
    pub amount: u64,
}

impl IsInitialized for Escrow {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
