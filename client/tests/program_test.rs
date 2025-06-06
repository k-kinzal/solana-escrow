mod accounts;
mod validator;

use crate::validator::Validator;
use borsh::BorshDeserialize;
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_sdk::account::AccountSharedData;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::program_option::COption;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signer;
use solana_sdk::system_program;
use spl_token::state::AccountState;
use std::sync::Arc;

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
                accounts::mint_account(None, 1_000_000_000, 9, None),
            ),
            (
                send_associated_token_account_pubkey,
                accounts::associated_token_account(
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
                accounts::mint_account(None, 1_000_000_000, 9, None),
            ),
            (
                receive_associated_token_account_pubkey,
                accounts::associated_token_account(
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

#[tokio::test]
async fn test_exchange() -> anyhow::Result<()> {
    let sender = Keypair::new();
    let receiver = Keypair::new();
    let send_mint_token_account = Keypair::new();
    let receive_mint_token_account = Keypair::new();
    let sender_send_associated_token_account_pubkey =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &sender.pubkey(),
            &send_mint_token_account.pubkey(),
            &spl_token::id(),
        );
    let sender_receive_associated_token_account_pubkey =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &sender.pubkey(),
            &receive_mint_token_account.pubkey(),
            &spl_token::id(),
        );
    let receiver_send_associated_token_account_pubkey =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &receiver.pubkey(),
            &receive_mint_token_account.pubkey(),
            &spl_token::id(),
        );
    let receiver_receive_associated_token_account_pubkey =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &receiver.pubkey(),
            &send_mint_token_account.pubkey(),
            &spl_token::id(),
        );
    let tmp_token_account = Keypair::new();
    let escrow_account = Keypair::new();
    let (pda, _) = Pubkey::find_program_address(&[b"escrow"], &escrow_program::id());

    let (validator, _) = Validator::default()
        .with_accounts(vec![
            (
                receiver.pubkey(),
                AccountSharedData::new(1_000_000_000, 0, &system_program::id()),
            ),
            (
                send_mint_token_account.pubkey(),
                accounts::mint_account(None, 1_000_000_000, 0, None),
            ),
            (
                receive_mint_token_account.pubkey(),
                accounts::mint_account(None, 1_000_000_000, 0, None),
            ),
            (
                sender_send_associated_token_account_pubkey,
                accounts::associated_token_account(
                    send_mint_token_account.pubkey(),
                    sender.pubkey(),
                    0,
                    None,
                    AccountState::Initialized,
                    None,
                    0,
                    None,
                ),
            ),
            (
                sender_receive_associated_token_account_pubkey,
                accounts::associated_token_account(
                    receive_mint_token_account.pubkey(),
                    sender.pubkey(),
                    0,
                    None,
                    AccountState::Initialized,
                    None,
                    0,
                    None,
                ),
            ),
            (
                receiver_send_associated_token_account_pubkey,
                accounts::associated_token_account(
                    receive_mint_token_account.pubkey(),
                    receiver.pubkey(),
                    100,
                    None,
                    AccountState::Initialized,
                    None,
                    0,
                    None,
                ),
            ),
            (
                receiver_receive_associated_token_account_pubkey,
                accounts::associated_token_account(
                    send_mint_token_account.pubkey(),
                    receiver.pubkey(),
                    0,
                    None,
                    AccountState::Initialized,
                    None,
                    0,
                    None,
                ),
            ),
            (
                tmp_token_account.pubkey(),
                accounts::associated_token_account(
                    send_mint_token_account.pubkey(),
                    pda,
                    100,
                    None,
                    AccountState::Initialized,
                    None,
                    0,
                    None,
                ),
            ),
            (
                escrow_account.pubkey(),
                accounts::escrow_account(
                    sender.pubkey(),
                    sender_receive_associated_token_account_pubkey,
                    tmp_token_account.pubkey(),
                    100,
                ),
            ),
        ])
        .start()
        .await?;

    let client = Arc::new(validator.get_async_rpc_client());
    let escrow = escrow_client::Client::builder(client.clone(), receiver.insecure_clone())
        .with_rpc_send_transaction_config(RpcSendTransactionConfig {
            skip_preflight: true,
            preflight_commitment: Some(CommitmentLevel::Processed),
            ..RpcSendTransactionConfig::default()
        })
        .with_escrow_program_id(escrow_program::id())
        .with_token_program_id(spl_token::id())
        .build();

    let _ = escrow.exchange(escrow_account.pubkey()).await?;

    let sender_send_associated_token_account = client
        .get_account(&sender_send_associated_token_account_pubkey)
        .await?;
    let sender_send_associated_token_account_data =
        spl_token::state::Account::unpack(&sender_send_associated_token_account.data)?;
    assert_eq!(
        sender_send_associated_token_account_data.mint,
        send_mint_token_account.pubkey()
    );
    assert_eq!(
        sender_send_associated_token_account_data.owner,
        sender.pubkey()
    );
    assert_eq!(sender_send_associated_token_account_data.amount, 0);
    assert_eq!(
        sender_send_associated_token_account_data.state,
        AccountState::Initialized
    );
    assert_eq!(
        sender_send_associated_token_account_data.delegate,
        COption::None
    );
    assert_eq!(
        sender_send_associated_token_account_data.delegated_amount,
        0
    );
    assert_eq!(
        sender_send_associated_token_account_data.is_native,
        COption::None
    );
    assert_eq!(
        sender_send_associated_token_account_data.close_authority,
        COption::None
    );

    let sender_receive_associated_token_account = client
        .get_account(&sender_receive_associated_token_account_pubkey)
        .await?;
    let sender_receive_associated_token_account_data =
        spl_token::state::Account::unpack(&sender_receive_associated_token_account.data)?;
    assert_eq!(
        sender_receive_associated_token_account_data.mint,
        receive_mint_token_account.pubkey()
    );
    assert_eq!(
        sender_receive_associated_token_account_data.owner,
        sender.pubkey()
    );
    assert_eq!(sender_receive_associated_token_account_data.amount, 100);
    assert_eq!(
        sender_receive_associated_token_account_data.state,
        AccountState::Initialized
    );
    assert_eq!(
        sender_receive_associated_token_account_data.delegate,
        COption::None
    );
    assert_eq!(
        sender_receive_associated_token_account_data.delegated_amount,
        0
    );
    assert_eq!(
        sender_receive_associated_token_account_data.is_native,
        COption::None
    );
    assert_eq!(
        sender_receive_associated_token_account_data.close_authority,
        COption::None
    );

    let receiver_send_associated_token_account = client
        .get_account(&receiver_send_associated_token_account_pubkey)
        .await?;
    let receiver_send_associated_token_account_data =
        spl_token::state::Account::unpack(&receiver_send_associated_token_account.data)?;
    assert_eq!(
        receiver_send_associated_token_account_data.mint,
        receive_mint_token_account.pubkey()
    );
    assert_eq!(
        receiver_send_associated_token_account_data.owner,
        receiver.pubkey()
    );
    assert_eq!(receiver_send_associated_token_account_data.amount, 0);
    assert_eq!(
        receiver_send_associated_token_account_data.state,
        AccountState::Initialized
    );
    assert_eq!(
        receiver_send_associated_token_account_data.delegate,
        COption::None
    );
    assert_eq!(
        receiver_send_associated_token_account_data.delegated_amount,
        0
    );
    assert_eq!(
        receiver_send_associated_token_account_data.is_native,
        COption::None
    );
    assert_eq!(
        receiver_send_associated_token_account_data.close_authority,
        COption::None
    );

    let receiver_receive_associated_token_account = client
        .get_account(&receiver_receive_associated_token_account_pubkey)
        .await?;
    let receiver_receive_associated_token_account_data =
        spl_token::state::Account::unpack(&receiver_receive_associated_token_account.data)?;
    assert_eq!(
        receiver_receive_associated_token_account_data.mint,
        send_mint_token_account.pubkey()
    );
    assert_eq!(
        receiver_receive_associated_token_account_data.owner,
        receiver.pubkey()
    );
    assert_eq!(receiver_receive_associated_token_account_data.amount, 100);
    assert_eq!(
        receiver_receive_associated_token_account_data.state,
        AccountState::Initialized
    );
    assert_eq!(
        receiver_receive_associated_token_account_data.delegate,
        COption::None
    );
    assert_eq!(
        receiver_receive_associated_token_account_data.delegated_amount,
        0
    );
    assert_eq!(
        receiver_receive_associated_token_account_data.is_native,
        COption::None
    );
    assert_eq!(
        receiver_receive_associated_token_account_data.close_authority,
        COption::None
    );

    let tmp_token_account = client.get_account(&tmp_token_account.pubkey()).await;
    assert_eq!(tmp_token_account.is_err(), true);

    let escrow_account = client.get_account(&escrow_account.pubkey()).await;
    assert_eq!(escrow_account.is_err(), true);

    Ok(())
}
