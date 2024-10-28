use crate::{instructions::fetch_price::fetch_price, state::game_state::GameState};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::Pack;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_token::instruction::transfer as spl_transfer;
use spl_token::state::Account as TokenAccount;

pub fn close_game(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    
    msg!("Entering the close_game instruction");
    let accounts_iter = &mut accounts.iter();

    let payer = next_account_info(accounts_iter)?; // Player 1
    let player2 = next_account_info(accounts_iter)?; // Player 2
    let escrow_account = next_account_info(accounts_iter)?; // Escrow account holding the game state
    let escrow_token_account_authority = next_account_info(accounts_iter)?;
    let escrow_token_account = next_account_info(accounts_iter)?; // Escrow token account holding USDC
    let fund_token_account_player1 = next_account_info(accounts_iter)?; // Player 1's USDC token account
    let fund_token_account_player2 = next_account_info(accounts_iter)?; // Player 2's USDC token account
    let token_program = next_account_info(accounts_iter)?; // SPL token program
    let oracle_account = next_account_info(accounts_iter)?;

    // Deserialize the current game state from the escrow account
    let mut game_state = GameState::try_from_slice(&escrow_account.try_borrow_data()?)?;

    // Ensure the game is  inactive
    if game_state.game_active {
        msg!("Impossible to close game, game is still active");
        return Err(ProgramError::InvalidAccountData);
    }

    // Ensure that there is a winner
    if game_state.winner == Pubkey::default() {
        msg!("Impossible to close game, there is no winner");
        return Err(ProgramError::InvalidAccountData);
    }

    // Logic to check the winner and assign the correct token account
    let token_account_data_player1 = TokenAccount::unpack(&fund_token_account_player1.try_borrow_data()?)?;
    let authority_player1 = token_account_data_player1.owner;

    let token_account_data_player2 = TokenAccount::unpack(&fund_token_account_player2.try_borrow_data()?)?;
    let authority_player2 = token_account_data_player2.owner;

    let winner_token_account: &AccountInfo;

    // Check if the authority matches the winner
    if authority_player1 == game_state.winner {
        winner_token_account = fund_token_account_player1;
        msg!("Winner is Player 1");
    } else if authority_player2 == game_state.winner {
        winner_token_account = fund_token_account_player2;
        msg!("Winner is Player 2");
    } else {
        msg!("No valid winner found");
        return Err(ProgramError::InvalidAccountData);
    }


    // Transfer 2000 USDC (2,000,000 micro USDC) from the escrow token account to the winner's token account
    let usdc_amount: u64 = 2000_000_000_000; // 2000 USDC in micro units

    // Ensure the escrow_token_account has the correct authority and ownership for SPL transfers
    invoke(
        &spl_transfer(
            token_program.key,                  // SPL token program
            escrow_token_account.key,           // Source account (escrow token account with USDC)
            winner_token_account.key,           // Destination account (winner's USDC token account)
            escrow_token_account_authority.key, // Authority (payer’s account)
            &[],                                // No additional signers
            usdc_amount,                        // Amount of USDC to transfer
        )?,
        &[
            escrow_token_account_authority.clone(),
            escrow_token_account.clone(),
            winner_token_account.clone(),
            token_program.clone(),
        ],
    )?;

    msg!("Game closed successfully. Winner has been paid.");
    Ok(())
}
