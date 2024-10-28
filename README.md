# Solana Escrow Gambling

## Overview

escrow-program is a smart contract built on Solana that facilitates a simple gambling game between two users. The game revolves around betting on the ETH/USDC price movement, where players bet whether the price will increase or decrease by 5%. The contract holds the entry fees in escrow and sends the funds to the winner once a price change condition is met.

## Task Overview

The goal of this contract is to facilitate a game where two users compete by betting on the ETH/USDC price. Users can bet on either an increase or decrease of the ETH price by 5%. Once one of these thresholds is reached, the winning player can call the `closeGame` function, and the escrow contract will verify the win and send the entry fees to the winner.

## Minimum Viable Product (MVP) Functionality

- **Entry into game for Player 1**:
  - $1000 USDC entrance fee.
  - Player selects either an increase or decrease of the ETH price.

- **Entry into game for Player 2**:
  - $1000 USDC entrance fee.
  - Must choose the opposite of Player 1’s choice.
  - Entry allowed only if the price has not fluctuated by more than 1% since Player 1 entered.

- **Withdrawing of entry**:
  - Only allowed for Player 1 if Player 2 has not entered the game yet.
  - Once Player 2 enters, no withdrawals are permitted.

- **Closing the game**:
  - The winner, once determined by a 5% price movement in their favor, calls the `closeGame` function to receive the entry fees.

## Clonning
```
git clone https://github.com/dariusjvc/solana-escrow-gambling.git
cd solana-escrow-gambling
```
## Dependencies

After cloning the repository, run the following command to install the necessary dependencies:
```
npm install
```

## Program Deployment

To deploy the program, simply run:
```
./cicd.sh
```
After compiling and deploying the program, it will return a `Program Id` similar to:

Program Id: 2fzcvS5kvody5nhc252GiD2nUZCDsCYV61Gx2TvNRewp

This Program ID should be added to the test script (`tests/test.ts`) by setting the variable `DEPLOYED_PROGRAM_ADDRESS`.


## Tests

### Running the Tests

To execute the tests, use the following command:
```
npm test
```
The program can fetch price data manually injected during tests, or if the value `0` is passed, it will automatically fetch prices from the Pyth Oracle on Solana's devnet. If another value is passed, it uses that value.

### Test Descriptions

1. **Create Game**:
   - Injected parameter: 
   const entry_price = 0; // If 0, the price is fetched from the Pyth Oracle

2. **Oracle Price Test**:
   - No input injected parameters needed

3. **Join Game**:
   - Injected parameter: 
   const last_price = 0; // If 0, the price is fetched from the Pyth Oracle

4. **Withdraw Game**:
   - No input injected parameters needed

5. **Settle Game**:
   - Injected parameter: 
   const last_price = 0; // If 0, the price is fetched from the Pyth Oracle

## Notes

- **Node Version**: Ensure you're using Node v18.20.4 or higher.
- **Solana CLI Version**: Ensure you're using Solana CLI version 1.18.23 or higher.


## Conclusion

This program demonstrates a simple escrow-based gambling game on Solana, using Pyth Oracle for real-time price data. Feel free to run the tests, review the contract logic, and adjust parameters as needed for your use case.
