use solana_sdk::account::AccountSharedData;
use solana_sdk::program_option::COption;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::rent::Rent;
use spl_token::state::AccountState;

pub fn mint_account(
    mint_authority: Option<Pubkey>,
    supply: u64,
    decimals: u8,
    freeze_authority: Option<Pubkey>,
) -> AccountSharedData {
    let mut account = AccountSharedData::new(
        Rent::default().minimum_balance(spl_token::state::Mint::LEN),
        spl_token::state::Mint::LEN,
        &spl_token::id(),
    );
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
        Rent::default().minimum_balance(spl_token::state::Account::LEN),
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
