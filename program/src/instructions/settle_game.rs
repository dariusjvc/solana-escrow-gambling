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

pub fn settle_game(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    
    msg!("Entering the settle_game instruction");
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

    //msg!("game_state {:?}", game_state);

    let last_price_got = if instruction_data.len() >= 9 {
        u64::from_le_bytes(instruction_data[1..9].try_into().unwrap())
    } else {
        0 // Default to 0 if last_price is not provided
    };

    msg!("last_price_got {:?}", last_price_got);

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

    // Ensure the game is still active
    if !game_state.game_active {
        msg!("Impossible to settle game, game is inactive");
        return Err(ProgramError::InvalidAccountData);
    }

    // Ensure Player 2 is already set
    if game_state.player2 == Pubkey::default() {
        msg!("Impossible to settle game, there is not a player2");
        return Err(ProgramError::InvalidAccountData);
    }

    // Mock price data (replace with actual data or oracle in real-world scenarios)
    let price_at_start = game_state.entry_price;
    //let current_price = price_at_start + (price_at_start / 10); // Simulate current price
    let current_price = game_state.last_price; // Simulate current price

    // Price thresholds for determining the winner
    let price_increase_threshold = price_at_start + (price_at_start * 5 / 100);
    let price_decrease_threshold = price_at_start - (price_at_start * 5 / 100);

    let winner_token_account: &AccountInfo;

    // Determine the winner based on player choices and price change
    if current_price >= price_increase_threshold {
        // If the price increased by 5%, check Player 1's choice
        if game_state.player1_choice {
            winner_token_account = fund_token_account_player1; // Player 1 wins (bet on increase)
            msg!("Player 1 wins with an increase bet.");
        } else {
            winner_token_account = fund_token_account_player2; // Player 2 wins (bet on decrease)
            msg!("Player 2 wins with a decrease bet.");
        }
    } else if current_price <= price_decrease_threshold {
        // If the price decreased by 5%, check Player 2's choice
        if !game_state.player1_choice {
            winner_token_account = fund_token_account_player1; // Player 1 wins (bet on decrease)
            msg!("Player 1 wins with a decrease bet.");
        } else {
            winner_token_account = fund_token_account_player2; // Player 2 wins (bet on increase)
            msg!("Player 2 wins with an increase bet.");
        }
    } else {
        msg!("There is not a winner");
        return Ok(());
        //return Err(ProgramError::InvalidAccountData); // No winner yet (price has not changed enough)
    }

    msg!("Winner account {:?}", winner_token_account.key);

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

    // Mark the game as inactive
    game_state.game_active = false;
    // game_state.winner = *winner_token_account.key;

    let token_account_data = TokenAccount::unpack(&winner_token_account.try_borrow_data()?)?;
    let token_account_authority = token_account_data.owner;

    msg!(
        "Token account authority (owner): {:?}",
        token_account_authority
    );

    // Set the winner as the authority of the winning token account
    game_state.winner = token_account_authority;

    // Serialize and store the updated game state in the escrow account
    let game_state_data = game_state.try_to_vec()?; // Convert GameState to a byte vector
    escrow_account
        .try_borrow_mut_data()?
        .copy_from_slice(&game_state_data); // Write data back into the escrow account

    msg!("Game settled successfully. Winner has been paid.");

    Ok(())
}
