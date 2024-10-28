use crate::{instructions::fetch_price::fetch_price, state::game_state::GameState};
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

pub fn join_game(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {

    msg!("Entering the join_game instruction");

    let accounts_iter = &mut accounts.iter();

    let player2 = next_account_info(accounts_iter)?; // Player 2
    let escrow_account = next_account_info(accounts_iter)?; // Escrow account holding game state
    let escrow_token_account = next_account_info(accounts_iter)?; // Escrow token account (where Player 2's USDC will be deposited)
    let player2_token_account = next_account_info(accounts_iter)?; // Player 2's USDC token account
    let token_program = next_account_info(accounts_iter)?; // Token program account for SPL tokens
    let oracle_account = next_account_info(accounts_iter)?;

    // Check if instruction_data contains last_price
    let last_price_got = if instruction_data.len() >= 9 {
        u64::from_le_bytes(instruction_data[1..9].try_into().unwrap())
    } else {
        0 // Default to 0 if last_price is not provided
    };

    msg!("last_price_got {:?}", last_price_got);

    // Deserialize the current game state from the escrow account
    let mut game_state = GameState::try_from_slice(&escrow_account.try_borrow_data()?)?;

    // Check if `last_price_got` is invalid (e.g., zero or a placeholder value), then fetch from the oracle
    if instruction_data.len() < 9 || last_price_got == 0 {
        msg!("last_price_got is invalid, fetching price from the oracle");

        // Fetch the price from the oracle and update game state (stored in escrow account)
        fetch_price(
            program_id,
            &[oracle_account.clone(), escrow_account.clone()],
        )?;

        // Re-fetch the updated game state from the escrow account
        game_state = GameState::try_from_slice(&escrow_account.try_borrow_data()?)?;

        // After fetch_price, the last_price should now be updated in the game state
        msg!("Price fetched from oracle: {}", game_state.last_price);
    } else {
        game_state.last_price = last_price_got;
        msg!("Using manually provided last_price: {}", last_price_got);
    }

    let fluctuation = game_state.last_price - game_state.entry_price;

    let percentage = (fluctuation * 100) / game_state.entry_price;
    msg!("Percentage {:?}", percentage);

    if percentage > 1 {
        msg!("Impossible to join Player 2, price fluctuation more than 1%.");
        return Err(ProgramError::InvalidAccountData);
    }

    // Ensure the game is still active
    if !game_state.game_active {
        return Err(ProgramError::InvalidAccountData);
    }

    // Ensure Player 2 is not already set
    if game_state.player2 != Pubkey::default() {
        return Err(ProgramError::InvalidAccountData);
    }
    // Set Player 2 in the game state
    game_state.player2 = *player2.key;

    // Update and serialize the game state
    let game_state_data = game_state.try_to_vec()?; // Convert GameState to a byte vector

    escrow_account
        .try_borrow_mut_data()?
        .copy_from_slice(&game_state_data); // Store the serialized data into the account's data

    // Transfer 1000 USDC (1,000,000,000 micro USDC) from Player 2's token account to the escrow token account
    let usdc_amount: u64 = 1_000_000_000; // 1000 USDC in micro units
    invoke(
        &spl_transfer(
            token_program.key,         // SPL token program
            player2_token_account.key, // Source account (Player 2's USDC token account)
            escrow_token_account.key,  // Destination account (escrow token account)
            player2.key,               // Authority (Player 2's account)
            &[],                       // No additional signers
            usdc_amount,               // Amount of USDC to transfer
        )?,
        &[
            player2.clone(),
            player2_token_account.clone(),
            escrow_token_account.clone(),
            token_program.clone(),
        ],
    )?;

    msg!("Player 2 joined the game successfully.");

    Ok(())
}
