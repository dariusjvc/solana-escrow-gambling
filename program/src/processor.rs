use crate::instructions::{
    create_game::create_game, fetch_price::fetch_price, join_game::join_game,
    settle_game::settle_game, withdraw_funds::withdraw_funds, close_game::close_game,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

pub struct Processor;

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = instruction_data[0];

        msg!("Instruction data length: {}", instruction_data.len());
        msg!(" Instruction: {:?}", instruction);

        match instruction {
            0 => create_game(program_id, accounts, instruction_data),// Create the game
            1 => fetch_price(program_id, accounts),// fetch_price
            2 => join_game(program_id, accounts, instruction_data),// Player joins
            3 => settle_game(program_id, accounts, instruction_data),// Settle the game and distribute winnings
            4 => withdraw_funds(program_id, accounts),//
            5 => close_game(program_id, accounts),//  
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
