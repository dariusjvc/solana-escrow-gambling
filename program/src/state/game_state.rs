use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::{Pubkey};

#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct GameState {
    pub player1: Pubkey,// 32 bytes
    pub player2: Pubkey,// 32 bytes
    pub player1_choice: bool, // Player 1's bet: true for increase, false for decrease
    pub player2_choice: bool, // Player 2's bet: true for increase, false for decrease
    pub entry_price: u64, 
    pub last_price: u64, // 8 bytes
    pub game_active: bool,// 1 byte (0 or 1 to represent true/false)
    pub winner: Pubkey,// Pubkey of the winner
}
