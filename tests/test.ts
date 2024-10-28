import {
    Connection,
    Keypair,
    PublicKey,
    sendAndConfirmTransaction,
    SystemProgram,
    Transaction,
    TransactionInstruction,
} from '@solana/web3.js';

import * as borsh from "borsh";
import { Buffer } from "buffer";

const path = require('path');

let PATH_TO_YOUR_SOLANA_PAYER_JSON = path.resolve(__dirname, '../wallets/payer.json');
let PATH_TO_YOUR_SOLANA_PLAYER2_JSON = path.resolve(__dirname, '../wallets/player2.json');
let PATH_TO_YOUR_SOLANA_GAME_JSON = path.resolve(__dirname, '../wallets/escrow.json');


let DEPLOYED_PROGRAM_ADDRESS = "FoqMdvAE1PEGyTMGBKrvhwAojWEH1n9soLLFBM7uvokE"
let PAYER_TOKEN_ACCOUNT = "ESLp21gWPUmPMsTo4wJAPSVdY6hiBU4PPn8KuvqxgBKD"
let PLAYER2_TOKEN_ACCOUNT = "6VCjhfe3njxWT4YqTKpQ7T6wkFHdn7QKpksfqa4wqiKj"
let SCROW_TOKEN_ACCOUNT = "2E6TLrmeuzMitoRviPK5Fd4kiZPXj7MteZLxAppPDif6"
const TOKEN_PROGRAM_ID = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

// Get oracles for mainnet here: https://www.pyth.network/developers/price-feed-ids#solana-stable
// JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB (for Solana Miannet)
const usdcPriceAccount = new PublicKey("EdVCmQ9FSPcVe5YySXDPCRmc8aDQLKJ9xvYBMZPie1Vw"); // ETH/USDC Price Feed Account (Devnet)

function createKeypairFromFile(path: string): Keypair {
    return Keypair.fromSecretKey(
        Buffer.from(JSON.parse(require('fs').readFileSync(path, "utf-8")))
    )
};

describe("Tests of the escrow_program in the Solana devnet:", () => {

    const connection = new Connection(`https://api.devnet.solana.com`, 'confirmed'); // For solana devnet
    //const connection = new Connection(`http://localhost:8899`, 'confirmed'); // For solana test validator
    //const connection = new Connection(`https://api.mainnet-beta.solana.com`, 'confirmed'); // For solana mainnet

    const payer = createKeypairFromFile(PATH_TO_YOUR_SOLANA_PAYER_JSON);
    const player2 = createKeypairFromFile(PATH_TO_YOUR_SOLANA_PLAYER2_JSON);
    const escrowTokenAccountAuthority = createKeypairFromFile(PATH_TO_YOUR_SOLANA_GAME_JSON);
    const gameAccount = Keypair.generate();
    const escrowTokenAccount = new PublicKey(SCROW_TOKEN_ACCOUNT);
    const payerTokenAccount = new PublicKey(PAYER_TOKEN_ACCOUNT);
    const player2TokenAccount = new PublicKey(PLAYER2_TOKEN_ACCOUNT);
    const PROGRAM_ID: PublicKey = new PublicKey(
        DEPLOYED_PROGRAM_ADDRESS
    );

    class GameState {
        player1: Uint8Array;
        player2: Uint8Array;
        player1_choice: boolean;
        player2_choice: boolean;
        entry_price: bigint;
        last_price: bigint;
        game_active: boolean;
        winner: Uint8Array;

        constructor(fields: { player1: Uint8Array, player2: Uint8Array, player1_choice: boolean, player2_choice: boolean, entry_price: bigint, last_price: bigint, game_active: boolean, winner: Uint8Array } | undefined = undefined) {
            if (fields) {
                this.player1 = fields.player1;
                this.player2 = fields.player2;
                this.player1_choice = fields.player1_choice,
                    this.player2_choice = fields.player2_choice,
                    this.entry_price = fields.entry_price;
                this.last_price = fields.last_price;
                this.game_active = fields.game_active;
                this.winner = fields.winner;
            }
        }
    }

    const GameStateSchema = new Map([
        [GameState, { kind: 'struct', fields: [['player1', [32]], ['player2', [32]], ['player1_choice', 'u8'], ['player2_choice', 'u8'], ['entry_price', 'u64'], ['last_price', 'u64'], ['game_active', 'u8'], ['winner', [32]]] }]
    ]);

    function serializeGameState(gameState: GameState): Buffer {
        return Buffer.from(borsh.serialize(GameStateSchema, gameState));
    }

    function deserializeGameState(buffer: Buffer): GameState {
        return borsh.deserialize(GameStateSchema, GameState, buffer);
    }

    it("Create game", async () => {
        logSeparator();

        const instruction_code = Buffer.from([0]);  // instruction_code as a single byte

        const player1_choice = true;  // true- -> 'increase', false -> 'decrease'

        //const entry_price = 0;
        const entry_price = 2500;

        // The price given by the oracle has 8 decimal places
        const entry_price_in_micro_usdc = Math.round(entry_price * 100_000_000);

        // Serialize the entry_price as u64 (8 bytes)
        const entry_price_buffer = Buffer.alloc(8);
        entry_price_buffer.writeBigUInt64LE(BigInt(entry_price_in_micro_usdc));


        const player1_choice_buffer = Buffer.from([player1_choice ? 1 : 0]);  // Convert boolean to 1 or 0

        // Concatenate the instruction_code, player1_choice, and entry_price buffers
        const data = Buffer.concat([instruction_code, player1_choice_buffer, entry_price_buffer]);

        const instruction = new TransactionInstruction({
            keys: [
                { pubkey: payer.publicKey, isSigner: true, isWritable: true },  // Player 1 (payer)
                { pubkey: gameAccount.publicKey, isSigner: true, isWritable: true },  // Escrow account for game state
                { pubkey: escrowTokenAccount, isSigner: false, isWritable: true },  // Escrow token account to hold USDC
                { pubkey: payerTokenAccount, isSigner: false, isWritable: true },  // Player 1's USDC token account
                { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },  // Token program to transfer USDC
                { pubkey: usdcPriceAccount, isSigner: false, isWritable: false }, // Pyth oracle
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },  // System program
            ],
            programId: PROGRAM_ID,
            data: data,
        });

        const transaction = new Transaction().add(instruction);
        const { blockhash } = await connection.getLatestBlockhash();
        transaction.recentBlockhash = blockhash;
        try {
            await sendAndConfirmTransaction(connection, transaction, [payer, gameAccount]);
        } catch (error) {
            console.error("Transaction failed with error:", error);
            if (error.logs) {
                console.log("Transaction logs:", error.logs);
            }
        }

        // Fetch and validate the game state in the escrow account
        const accountInfo = await connection.getAccountInfo(gameAccount.publicKey);
        const fundTokenBalance = await connection.getTokenAccountBalance(escrowTokenAccount);



        if (accountInfo === null) {
            console.error("Failed to create game account");
        } else {
            const gameState = deserializeGameState(accountInfo.data);

            // Verify if Player 1's public key is correctly set in the game state
            if (new PublicKey(gameState.player1).equals(payer.publicKey)) {
                console.log("Test passed: Player 1 correctly set");
            } else {
                console.error("Test failed: Player 1 not set correctly");
            }

            // Verify if the game is active
            if (gameState.game_active == true) {
                console.log("Test passed: Game is active");
            } else {
                console.error("Test failed: Game is not active");
            }

            console.log(`Entry price is ${formatPrice(gameState.entry_price)} ETH/USDC`);
            console.log(`Last price is ${formatPrice(gameState.last_price)} ETH/USDC`);
            

        }
        console.log(`Escrow Token Account Balance: ${fundTokenBalance.value.uiAmount} USDC`);
    });

    it('Oracle Price', async () => {
        logSeparator();
        const instruction = new TransactionInstruction({
            keys: [
                { pubkey: usdcPriceAccount, isSigner: false, isWritable: false }, // Oráculo de precios
                { pubkey: gameAccount.publicKey, isSigner: false, isWritable: true },  // Escrow account for game state
            ],
            programId: PROGRAM_ID, // ID del programa en Solana
            data: Buffer.from([1]), // No se necesita data adicional para esta consulta
        });

        const transaction = new Transaction().add(instruction);

        // Envía y confirma la transacción
        try {
            // Get the latest blockhash
            const { blockhash } = await connection.getLatestBlockhash();
            transaction.recentBlockhash = blockhash;

            // Send the transaction and confirm
            try {
                const signature = await sendAndConfirmTransaction(connection, transaction, [payer]);

                // Get the transaction details using the signature
                const transactionDetails = await connection.getTransaction(signature, { commitment: 'confirmed' });

                // Check if there are logs and extract the price value
                if (transactionDetails && transactionDetails.meta && transactionDetails.meta.logMessages) {
                    const logs = transactionDetails.meta.logMessages;

                    // Find the log that contains the price
                    const priceLog = logs.find(log => log.includes('Price of ETH/USDC:'));

                    if (priceLog) {
                        const price = priceLog.split('Price of ETH/USDC: ')[1]; // Extract the price from the log
                        console.log("Test passed: Price successfully retrieved");
                        console.log(`The price is: ${formatPrice(BigInt(price))} ETH/USDC`);
                    } else {
                        console.log('Price log not found');
                    }
                } else {
                    console.error('No logs found in the transaction.');
                }

            } catch (error) {
                console.error("Transaction failed with error:", error);
                if (error.logs) {
                    console.log("Transaction logs:", error.logs);
                }
            }

        } catch (error) {
            console.error('Error querying the oracle price:', error);
            throw new Error(`Transaction failed: ${error.message}`);
        }

        const accountInfo = await connection.getAccountInfo(gameAccount.publicKey);
        const gameState = deserializeGameState(accountInfo.data);

        //console.log(gameState);

    });


    it("Join game", async () => {
        logSeparator();
        const instruction_code = Buffer.from([2]);

        //const last_price = 0;
        const last_price = 2500;

        // The price has 8 decimal places
        const last_price_in_micro_usdc = Math.round(last_price * 100_000_000);

        // Serialize the entry_price as u64 (8 bytes)
        const last_price_buffer = Buffer.alloc(8);
        last_price_buffer.writeBigUInt64LE(BigInt(last_price_in_micro_usdc));

        const data = Buffer.concat([instruction_code, last_price_buffer]);
        // Prepare the instruction for the join_game function
        const instruction = new TransactionInstruction({
            keys: [
                { pubkey: player2.publicKey, isSigner: true, isWritable: true }, // Player 2
                { pubkey: gameAccount.publicKey, isSigner: false, isWritable: true }, // Escrow account for game state
                { pubkey: escrowTokenAccount, isSigner: false, isWritable: true },  // Escrow token account (holds USDC for both players)
                { pubkey: player2TokenAccount, isSigner: false, isWritable: true }, // Player 2's USDC token account
                { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },  // Token program to transfer USDC
                { pubkey: usdcPriceAccount, isSigner: false, isWritable: false }, // Oráculo de precios
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },  // System program
            ],
            programId: PROGRAM_ID,
            data: data,
        });

        const transaction = new Transaction().add(instruction);
        const { blockhash } = await connection.getLatestBlockhash();
        transaction.recentBlockhash = blockhash;

        try {
            // Send and confirm the transaction
            await sendAndConfirmTransaction(
                connection,
                transaction,
                [payer, player2] // Signers: Player 1 and Player 2
            );

        } catch (error) {
            // Catch the error and handle specific cases based on logs
            if (error.logs) {
                const logs = error.logs;

                // Check for the specific log message related to price fluctuation
                if (logs.some(log => log.includes("Impossible to join Player 2, price fluctuation more than 1%"))) {
                    console.log("Test valid: Player 2 could not join due to price fluctuation more than 1%");
                    // Treat this as a valid outcome and not a test failure
                    return;
                } else {
                    console.error("Transaction logs:", logs);
                }
            } else {
                console.error("No logs available for this transaction");
            }

            throw error;
        }

        // Fetch the game account data again to check the state
        const accountInfo = await connection.getAccountInfo(gameAccount.publicKey);
        const gameState = deserializeGameState(accountInfo.data);

        if (new PublicKey(gameState.player2).equals(player2.publicKey)) {
            console.log("Test passed: Player 2 correctly set");
        } else {
            console.error("Test failed: Player 2 not set correctly");
        }

        if (gameState.game_active == true) {
            console.log("Test passed: Game is active");
        } else {
            console.error("Test failed: Game is not active");
        }

        // Fetch the fund token account balance to verify deposit
        const fundTokenBalance = await connection.getTokenAccountBalance(escrowTokenAccount);
        console.log(`Escrow Token Account Balance: ${fundTokenBalance.value.uiAmount} USDC`);

        const player1Balance = await connection.getTokenAccountBalance(payerTokenAccount);
        const player2Balance = await connection.getTokenAccountBalance(player2TokenAccount);

        console.log(`Player 1 Token Balance: ${player1Balance.value.uiAmount} USDC`);
        console.log(`Player 2 Token Balance: ${player2Balance.value.uiAmount} USDC`);

        console.log(`Last price: ${formatPrice(gameState.last_price)} ETH/USDC`);


    });


    it("Withdraw Game", async () => {
        logSeparator();
        const instruction_code = 4;
        const data = Buffer.from([instruction_code]);
        const instruction = new TransactionInstruction({
            keys: [
                { pubkey: payer.publicKey, isSigner: true, isWritable: true },  // Player 1 (payer)
                { pubkey: gameAccount.publicKey, isSigner: false, isWritable: true },  // Escrow account for game state
                { pubkey: escrowTokenAccountAuthority.publicKey, isSigner: true, isWritable: true },
                { pubkey: escrowTokenAccount, isSigner: false, isWritable: true },  // Escrow token account to hold USDC
                { pubkey: payerTokenAccount, isSigner: false, isWritable: true },  // Player 1's USDC token account
                { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },  // Token program to transfer USDC
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },  // System program

            ],
            programId: PROGRAM_ID,  // Your Solana program ID
            data: data,
        });

        const transaction = new Transaction().add(instruction);
        const { blockhash } = await connection.getLatestBlockhash();
        transaction.recentBlockhash = blockhash;
        try {
            await sendAndConfirmTransaction(connection, transaction, [payer, escrowTokenAccountAuthority]);
            console.log("Test passed: Player 1 withdrew funds.");  // Success message
        } catch (error) {
            // Catch the error and check for logs
            if (error.logs) {
                const logs = error.logs;
                // Check if Player 2 already exists and log a custom message
                if (logs.some(log => log.includes("Player 2 already exists"))) {
                    console.error("Impossible to withdraw: Player 2 already exists, withdrawal not allowed");
                } else {
                    console.error("Transaction logs:", logs);
                }
            } else {
                console.error("No logs available for this transaction");
            }
        }

        // Fetch and validate the game state in the escrow account
        const accountInfo = await connection.getAccountInfo(gameAccount.publicKey);


        const gameState = deserializeGameState(accountInfo.data);


        if (gameState.game_active == false) {
            console.log("Test passed: Now the game is inactive");
        } else {
            console.error("Test passed: Game is still active");
        }

        const fundTokenBalance = await connection.getTokenAccountBalance(escrowTokenAccount);

        console.log(`Escrow Token Account Balance: ${fundTokenBalance.value.uiAmount} USDC`);

        const player1Balance = await connection.getTokenAccountBalance(payerTokenAccount);

        console.log(`Player 1 Token Balance: ${player1Balance.value.uiAmount} USDC`);
    }
    );

        it("Settle game", async () => {
            logSeparator();
            const instruction_code = Buffer.from([3]);

            //const last_price = 0;
            const last_price = 3000;

            // The price has 8 decimal places
            const last_price_in_micro_usdc = Math.round(last_price * 100_000_000);

            // Serialize the entry_price as u64 (8 bytes)
            const last_price_buffer = Buffer.alloc(8);
            last_price_buffer.writeBigUInt64LE(BigInt(last_price_in_micro_usdc));

            const data = Buffer.concat([instruction_code, last_price_buffer]);

            // Prepare the instruction for the settle_game function
            const instruction = new TransactionInstruction({
                keys: [
                    { pubkey: payer.publicKey, isSigner: true, isWritable: true },  // Player 1 (payer)
                    { pubkey: player2.publicKey, isSigner: true, isWritable: true }, // Player 2
                    { pubkey: gameAccount.publicKey, isSigner: false, isWritable: true }, // Existing game account (escrow for game state, NOT lamports)
                    { pubkey: escrowTokenAccountAuthority.publicKey, isSigner: true, isWritable: true },  // Token account holding the USDC (Escrow token account)
                    { pubkey: escrowTokenAccount, isSigner: false, isWritable: true },  // Token account holding the USDC (Escrow token account)
                    { pubkey: payerTokenAccount, isSigner: false, isWritable: true },  // Player 1's token account (USDC)
                    { pubkey: player2TokenAccount, isSigner: false, isWritable: true },  // Player 2's token account (USDC)
                    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },  // Token program for SPL tokens
                    { pubkey: usdcPriceAccount, isSigner: false, isWritable: false }, // Pyth Oracle
                    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // System program
                ],
                programId: PROGRAM_ID,
                data: data, // Data to trigger the `settle_game` instruction
            });

            const transaction = new Transaction().add(instruction);
            const { blockhash } = await connection.getLatestBlockhash();
            transaction.recentBlockhash = blockhash;

            try {
                // Send and confirm the transaction
                await sendAndConfirmTransaction(
                    connection,
                    transaction,
                    [payer, player2, escrowTokenAccountAuthority]  // Both players sign the transaction
                );
            } catch (error) {
                // Catch the error and handle the case when the game is inactive or Player 2 is missing
                if (error.logs) {

                    console.log("Transaction logs:", error.logs);
                    const logs = error.logs;

                    // Check if the logs contain the game inactive message
                    if (logs.some(log => log.includes("Impossible to settle game, game is inactive"))) {
                        console.log("Test passed: Settle game failed because the game is inactive");
                    }
                    // Check if the logs contain the message about Player 2 missing
                    else if (logs.some(log => log.includes("Impossible to settle game, there is not a player2"))) {
                        console.log("Test passed: Settle game failed because Player 2 is not present");
                    }
                    else {
                        console.error("Transaction failed with unexpected error:", logs);
                    }
                } else {
                    console.error("No logs available for this transaction");
                }
            }

            // Fetch the game account data again to check the updated state
            const accountInfo = await connection.getAccountInfo(gameAccount.publicKey);
            const gameState = deserializeGameState(accountInfo.data);

            // Check the USDC balances to confirm the winner
            const player1Balance = await connection.getTokenAccountBalance(payerTokenAccount);
            const player2Balance = await connection.getTokenAccountBalance(player2TokenAccount);

            // The winner should receive the 2000 USDC
            const escrowBalance = await connection.getTokenAccountBalance(escrowTokenAccount);

            console.log(`Escrow Token Account Balance: ${escrowBalance.value.uiAmount} USDC`);

            // Convert the winner's public key from the game state to a PublicKey object
            const winnerPubKey = new PublicKey(gameState.winner);

            if (winnerPubKey.equals(PublicKey.default)) {
                console.log("There is no winner");
            } else if (winnerPubKey.equals(payer.publicKey)) {
                console.log("Test passed: Player 1 is the winner");
            } else if (winnerPubKey.equals(player2.publicKey)) {
                console.log("Test passed: Player 2 is the winner");
            } else {
                console.error("Test failed: Winner's public key does not match Player 1 or Player 2");
            }

            if (gameState.game_active == false) {
                console.log("Test passed: Game is inactive");
            } else {
                console.error("Game is still active");
            }

            console.log(`Player 1 Token Balance: ${player1Balance.value.uiAmount} USDC`);
            console.log(`Player 2 Token Balance: ${player2Balance.value.uiAmount} USDC`);
            console.log(`Last price: ${formatPrice(gameState.last_price)} ETH/USDC`);

        });


    it("Close game", async () => {
        logSeparator();
        const instruction_code = Buffer.from([5]);

        const data = Buffer.concat([instruction_code]);

        const instruction = new TransactionInstruction({
            keys: [
                { pubkey: payer.publicKey, isSigner: true, isWritable: true },  // Player 1 (payer)
                { pubkey: player2.publicKey, isSigner: true, isWritable: true }, // Player 2
                { pubkey: gameAccount.publicKey, isSigner: false, isWritable: true }, // Existing game account (escrow for game state, NOT lamports)
                { pubkey: escrowTokenAccountAuthority.publicKey, isSigner: true, isWritable: true },  // Token account holding the USDC (Escrow token account)
                { pubkey: escrowTokenAccount, isSigner: false, isWritable: true },  // Token account holding the USDC (Escrow token account)
                { pubkey: payerTokenAccount, isSigner: false, isWritable: true },  // Player 1's token account (USDC)
                { pubkey: player2TokenAccount, isSigner: false, isWritable: true },  // Player 2's token account (USDC)
                { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },  // Token program for SPL tokens
                { pubkey: usdcPriceAccount, isSigner: false, isWritable: false }, // Pyth Oracle
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // System program
            ],
            programId: PROGRAM_ID,
            data: data, // Data to trigger the `settle_game` instruction
        });

        const transaction = new Transaction().add(instruction);
        const { blockhash } = await connection.getLatestBlockhash();
        transaction.recentBlockhash = blockhash;

        try {
            // Send and confirm the transaction
            await sendAndConfirmTransaction(
                connection,
                transaction,
                [payer, player2, escrowTokenAccountAuthority]  // Both players sign the transaction
            );
        } catch (error) {
            // Catch the error and handle the case when the game is inactive or Player 2 is missing
            if (error.logs) {

                //console.log("Transaction logs:", error.logs);
                const logs = error.logs;
                // Check if Player 2 already exists and log a custom message

                if (logs.some(log => log.includes("Impossible to close game, game is still active"))) {
                    console.error("Impossible to close: The game is still active, not allowed");
                }
                else if (logs.some(log => log.includes("Impossible to close game, there is no winner"))) {
                    console.error("Impossible to close: There is no winner, not allowed");
                } else {
                    console.error("Transaction logs:", logs);
                }

            } else {
                console.error("No logs available for this transaction");
            }
        }

        // Check the USDC balances to confirm the winner
        const player1Balance = await connection.getTokenAccountBalance(payerTokenAccount);
        const player2Balance = await connection.getTokenAccountBalance(player2TokenAccount);

        console.log(`Player 1 Token Balance: ${player1Balance.value.uiAmount} USDC`);
        console.log(`Player 2 Token Balance: ${player2Balance.value.uiAmount} USDC`);

    });

});

function formatPrice(price: bigint, decimals: number = 8): string {
    const priceStr = price.toString();
    const integerPart = priceStr.slice(0, -decimals) || "0";
    const decimalPart = priceStr.slice(-decimals).padStart(decimals, "0");
    return `${integerPart}.${decimalPart}`;
}

function logSeparator() {
    console.log('-------------------------------------------------');
}