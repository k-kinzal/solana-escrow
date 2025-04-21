use solana_sdk::account::AccountSharedData;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::rent::Rent;

pub fn escrow_account(
    seller_pubkey: Pubkey,
    seller_token_account_pubkey: Pubkey,
    temp_token_account_pubkey: Pubkey,
    amount: u64,
) -> AccountSharedData {
    let len = borsh::max_serialized_size::<escrow_program::state::Escrow>()
        .expect("Failed to get max serialized size");
    let mut account = AccountSharedData::new(
        Rent::default().minimum_balance(len),
        len,
        &escrow_program::id(),
    );
    let escrow = escrow_program::state::Escrow {
        is_initialized: true,
        seller_pubkey,
        seller_token_account_pubkey,
        temp_token_account_pubkey,
        amount,
    };
    let data = borsh::to_vec(&escrow).unwrap();
    account.set_data_from_slice(&data);

    account
}
