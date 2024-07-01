use crate::instruction::Instruction;
use crate::state::Escrow;
use borsh::BorshDeserialize;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack};
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use spl_token::instruction::AuthorityType;

/// Processor is processing the instructions.
pub struct Processor;

impl Processor {
    fn process_init(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        // Retrieving an accounts
        let account_iter = &mut accounts.iter();
        let seller_account = next_account_info(account_iter)?;
        let seller_token_account = next_account_info(account_iter)?;
        let temp_token_account = next_account_info(account_iter)?;
        let escrow_account = next_account_info(account_iter)?;
        let rent = Rent::from_account_info(next_account_info(account_iter)?)?;
        let token_program = next_account_info(account_iter)?;

        // Validating the accounts
        if seller_token_account.owner != token_program.key {
            return Err(ProgramError::IncorrectProgramId);
        }
        if !rent.is_exempt(escrow_account.lamports(), escrow_account.data_len()) {
            return Err(ProgramError::AccountNotRentExempt);
        }

        // Initializing the escrow account
        let data = &mut escrow_account.data.borrow_mut();
        let mut state = borsh::from_slice::<Escrow>(data)?;
        if state.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        state.is_initialized = true;
        state.seller_pubkey = *seller_account.key;
        state.seller_token_account_pubkey = *seller_token_account.key;
        state.temp_token_account_pubkey = *temp_token_account.key;
        state.amount = amount;

        data.copy_from_slice(borsh::to_vec(&state)?.as_slice());

        // Change the ownership of the temporary token account to the PDA
        let (pda, _) = Pubkey::find_program_address(&[b"escrow"], program_id);
        let ix = spl_token::instruction::set_authority(
            token_program.key,
            temp_token_account.key,
            Some(&pda),
            AuthorityType::AccountOwner,
            seller_account.key,
            &[seller_account.key],
        )?;
        invoke(&ix, &[temp_token_account.clone(), seller_account.clone()])?;

        Ok(())
    }

    fn process_exchange(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        // Retrieving an accounts
        let account_iter = &mut accounts.iter();
        let buyer_account = next_account_info(account_iter)?;
        let buyer_send_token_account = next_account_info(account_iter)?;
        let buyer_receive_token_account = next_account_info(account_iter)?;
        let temp_token_account = next_account_info(account_iter)?;
        let seller_account = next_account_info(account_iter)?;
        let seller_token_account = next_account_info(account_iter)?;
        let escrow_account = next_account_info(account_iter)?;
        let token_program = next_account_info(account_iter)?;
        let pda_account = next_account_info(account_iter)?;

        // Validating the accounts
        let temp_token_account_state =
            spl_token::state::Account::unpack(&temp_token_account.try_borrow_data()?)?;
        if amount != temp_token_account_state.amount {
            return Err(ProgramError::InvalidAccountData);
        }

        let state = borsh::from_slice::<Escrow>(&escrow_account.data.borrow())?;
        if !state.is_initialized() {
            return Err(ProgramError::InvalidAccountData);
        }
        if state.temp_token_account_pubkey != *temp_token_account.key {
            return Err(ProgramError::InvalidAccountData);
        }
        if state.seller_pubkey != *seller_account.key {
            return Err(ProgramError::InvalidAccountData);
        }
        if state.seller_token_account_pubkey != *seller_token_account.key {
            return Err(ProgramError::InvalidAccountData);
        }

        // Transfer the token from the buyer to the seller
        let ix = spl_token::instruction::transfer(
            token_program.key,
            buyer_send_token_account.key,
            seller_token_account.key,
            buyer_account.key,
            &[buyer_account.key],
            state.amount,
        )?;
        invoke(
            &ix,
            &[
                buyer_send_token_account.clone(),
                seller_token_account.clone(),
                buyer_account.clone(),
                token_program.clone(),
            ],
        )?;

        // Transfer the token from the seller (temporary deposit) to the buyer
        let (pda, nonce) = Pubkey::find_program_address(&[b"escrow"], program_id);
        let ix = spl_token::instruction::transfer(
            token_program.key,
            temp_token_account.key,
            buyer_receive_token_account.key,
            &pda,
            &[&pda],
            temp_token_account_state.amount,
        )?;
        invoke_signed(
            &ix,
            &[
                temp_token_account.clone(),
                buyer_receive_token_account.clone(),
                pda_account.clone(),
                token_program.clone(),
            ],
            &[&[&b"escrow"[..], &[nonce]]],
        )?;

        // Close the temporary account
        let ix = spl_token::instruction::close_account(
            token_program.key,
            temp_token_account.key,
            seller_account.key,
            &pda,
            &[&pda],
        )?;
        invoke_signed(
            &ix,
            &[
                temp_token_account.clone(),
                seller_account.clone(),
                pda_account.clone(),
                token_program.clone(),
            ],
            &[&[&b"escrow"[..], &[nonce]]],
        )?;

        // Close the escrow account
        let mut seller_account_lamports = seller_account.lamports.borrow_mut();
        **seller_account_lamports = seller_account_lamports
            .checked_add(escrow_account.lamports())
            .ok_or(ProgramError::ArithmeticOverflow)?;
        let mut escrow_account_lamports = escrow_account.lamports.borrow_mut();
        **escrow_account_lamports = 0u64;
        let mut escrow_account_data = escrow_account.data.borrow_mut();
        *escrow_account_data = &mut [];

        Ok(())
    }

    /// Handle the instruction.
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = Instruction::deserialize(&mut &input[..])?;
        match instruction {
            Instruction::Initialize(amount) => Self::process_init(program_id, accounts, amount),
            Instruction::Exchange(amount) => Self::process_exchange(program_id, accounts, amount),
        }
    }
}
