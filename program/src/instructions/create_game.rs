use crate::{instructions::fetch_price::fetch_price, state::game_state::GameState};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    pubkey::Pubkey,
    system_instruction,
    sysvar::rent::Rent,
    sysvar::Sysvar,
};
use spl_token::instruction::transfer as spl_transfer;

pub fn create_game(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {

    msg!("Entering the create_game instruction");

    let accounts_iter = &mut accounts.iter();

    let payer = next_account_info(accounts_iter)?; // Player 1 (payer)
    let escrow_account = next_account_info(accounts_iter)?; // Escrow account for game state
    let escrow_token_account = next_account_info(accounts_iter)?; // Escrow USDC token account
    let payer_token_account = next_account_info(accounts_iter)?; // Payer's USDC token account
    let token_program = next_account_info(accounts_iter)?; // Token program for SPL tokens
    let oracle_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?; // System program

    // Ensure that we are transferring the correct amount of USDC to the escrow token account (1000 USDC = 1,000,000 micro USDC)
    let usdc_amount: u64 = 1_000_000; // USDC has 6 decimals, so 1000 USDC is represented as 1,000,000 in smallest units

    let player1_choice = instruction_data[1] != 0; // If the second byte is 1, player1_choice is true, otherwise it's false

    //
    let entry_price_got = u64::from_le_bytes(instruction_data[2..10].try_into().unwrap());
    let last_price = entry_price_got;
    msg!("entry_price_got {:?}", entry_price_got);
    //

    // Transfer 1000 USDC from payer's token account to the escrow token account
    invoke(
        &spl_transfer(
            token_program.key,        // SPL token program
            payer_token_account.key,  // Source account (payer's USDC token account)
            escrow_token_account.key, // Destination account (escrow USDC token account)
            payer.key,                // Authority (payer's account)
            &[],                      // No additional signers
            usdc_amount,              // Amount of USDC to transfer
        )?,
        &[
            payer.clone(),
            payer_token_account.clone(),
            escrow_token_account.clone(),
            token_program.clone(),
        ],
    )?;

    // Create the escrow account (for holding the game state)
    let rent = Rent::get()?;
    let game_state_size = GameState::default().try_to_vec()?.len(); // Size of the serialized game state
    let required_lamports_for_escrow = rent.minimum_balance(game_state_size);
    invoke(
        &system_instruction::create_account(
            payer.key,
            escrow_account.key,           // Create the escrow account
            required_lamports_for_escrow, // Rent exemption for holding the game state
            game_state_size as u64,       // Size of the game state
            program_id,                   // The program that owns this account (your program)
        ),
        &[
            payer.clone(),
            escrow_account.clone(),
            system_program.clone(),
        ],
    )?;

    // Initialize the game state and store it in the escrow account
    let mut game_state = GameState::default();
    game_state.player1 = *payer.key;
    game_state.game_active = true;

    game_state.player1_choice = player1_choice;
    game_state.player2_choice = !player1_choice;

    // Serialize the game state and store it in the escrow account
    let game_state_data = game_state.try_to_vec()?; // Convert GameState to a byte vector
    escrow_account
        .try_borrow_mut_data()?
        .copy_from_slice(&game_state_data); // Write to escrow account

    msg!("Game created successfully with escrow and token accounts.");

    let mut updated_game_state = GameState::try_from_slice(&escrow_account.try_borrow_data()?)?;
    // Check if `last_price_got` is invalid (e.g., zero or a placeholder value), then fetch from the oracle
    if entry_price_got == 0 || instruction_data.len() < 10 {
        msg!("entry_price_got is invalid, fetching price from the oracle");

        // Fetch the price from the oracle and update game state (stored in escrow account)
        fetch_price(
            program_id,
            &[oracle_account.clone(), escrow_account.clone()],
        )?;

        // Re-fetch the updated game state from the escrow account
        updated_game_state = GameState::try_from_slice(&escrow_account.try_borrow_data()?)?;

        updated_game_state.entry_price = updated_game_state.last_price;

        // After fetch_price, the last_price should now be updated in the game state
        msg!(
            "Price fetched from oracle: {}",
            updated_game_state.last_price
        );
    } else {
        updated_game_state.entry_price = entry_price_got; // Set the entry price as 1000 USDC
        updated_game_state.last_price = last_price;
        msg!("Using manually provided entry_price: {}", entry_price_got);
    }

    let updated_game_state_data = updated_game_state.try_to_vec()?; // Convert GameState to a byte vector
    escrow_account
        .try_borrow_mut_data()?
        .copy_from_slice(&updated_game_state_data); // Store the serialized data into the account's data

    Ok(())
}
