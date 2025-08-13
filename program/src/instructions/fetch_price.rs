use crate::state::game_state::GameState;
use borsh::{BorshDeserialize, BorshSerialize};
use pyth_sdk_solana::{load_price_feed_from_account_info, PriceFeed, PythError};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

/// Function to fetch the ETH/USDC price from a Pyth oracle account
pub fn fetch_price(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {

    msg!("Entering the fetch_price instruction");

    // Assuming the oracle account is the first in the list of accounts passed
    let oracle_account = &accounts[0];
    let escrow_account = &accounts[1]; // Escrow account for game state

    // Load the price feed from the oracle account
    let price_feed_result: Result<PriceFeed, PythError> =
        load_price_feed_from_account_info(oracle_account);

    // Handle the result of loading the price feed
    let price_feed = match price_feed_result {
        Ok(feed) => feed,
        Err(_) => return Err(ProgramError::InvalidAccountData), // Return error if loading the price feed fails
    };

    // Fetch the current price from the price feed
    let price = price_feed
        .get_current_price()
        .ok_or(ProgramError::InvalidAccountData)?;

    msg!("Price of ETH/USDC: {}", price.price);

    // Deserialize the current game state from the escrow account
    let mut game_state = GameState::try_from_slice(&escrow_account.try_borrow_data()?)?;

    //Updating the last price with the price obtained from the oracle
    game_state.last_price = price.price as u64;

    let game_state_data = game_state.try_to_vec()?;

    escrow_account
        .try_borrow_mut_data()?
        .copy_from_slice(&game_state_data); // Store the serialized data into the account's data

    msg!("Price fetched successfully from oracle and stored in game state: {}", game_state.last_price);

    Ok(())
}
