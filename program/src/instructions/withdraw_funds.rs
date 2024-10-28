use crate::state::game_state::GameState;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use spl_token::instruction::transfer as spl_transfer;

pub fn withdraw_funds(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {

    msg!("Entering the withdraw_funds instruction");

    let accounts_iter = &mut accounts.iter();

    let payer = next_account_info(accounts_iter)?; // Player 1
    let escrow_account = next_account_info(accounts_iter)?; // Escrow account holding the game state
    let escrow_token_account_authority = next_account_info(accounts_iter)?;
    let escrow_token_account = next_account_info(accounts_iter)?; // Escrow token account holding USDC
    let fund_token_account_player1 = next_account_info(accounts_iter)?; // Player 1's USDC token account
    let token_program = next_account_info(accounts_iter)?; // SPL token program

    let mut game_state = GameState::try_from_slice(&escrow_account.try_borrow_data()?)?;
    // Ensure Player 2 is not already set
    if game_state.player2 != Pubkey::default() {
        msg!("Impossible to withdraw: Player 2 already exists, withdrawal not allowed.");
        return Err(ProgramError::InvalidAccountData); // Return an error indicating Player 2 is already set
    }

    let usdc_amount: u64 = 1000_000_000; // USDC Token created has 9 decimals, so 1000 USDC is represented as 1,000,000,000 in smallest units

    // Ensure the escrow_token_account has the correct authority and ownership for SPL transfers
    invoke(
        &spl_transfer(
            token_program.key,                  // SPL token program
            escrow_token_account.key,           // Source account (escrow token account with USDC)
            fund_token_account_player1.key,     // Destination account (winner's USDC token account)
            escrow_token_account_authority.key, // Authority (payer’s account)
            &[],                                // No additional signers
            usdc_amount,                        // Amount of USDC to transfer
        )?,
        &[
            escrow_token_account_authority.clone(),
            escrow_token_account.clone(),
            fund_token_account_player1.clone(),
            token_program.clone(),
        ],
    )?;

    game_state.game_active = false;
    let game_state_data = game_state.try_to_vec()?; // Convert GameState to a byte vector
    escrow_account
        .try_borrow_mut_data()?
        .copy_from_slice(&game_state_data);

    Ok(())
}
