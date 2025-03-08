#!/bin/bash

# Paths to key files
PROGRAM_SO=./program/target/so/escrow_program.so
PROGRAM_KEYPAIR=./program/target/so/escrow_program-keypair.json
FEE_PAYER=./wallets/payer.json
UPGRADE_AUTHORITY=./wallets/payer.json

# Compile the program
cargo build-bpf --manifest-path=./program/Cargo.toml --bpf-out-dir=./program/target/so

# Retrieve the program ID from the keypair
PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR")

# Function to get the balance in SOL
get_balance() {
    solana balance "$FEE_PAYER" | awk '{print $1}'
}

# Record initial balance
INITIAL_BALANCE=$(get_balance)

# Check if the program is already deployed
if solana program show "$PROGRAM_ID" &>/dev/null; then
    # Existing deployment: Upgrade the program
    echo "Upgrading existing program with ID: $PROGRAM_ID"

    # Get the current program account size
    CURRENT_SIZE=$(solana program show "$PROGRAM_ID" | grep 'Data Length' | awk '{print $3}')

    # Get the size of the new program binary
    NEW_SIZE=$(stat -c%s "$PROGRAM_SO")

    # Check if extension is needed
    if [ "$NEW_SIZE" -gt "$CURRENT_SIZE" ]; then
        # Calculate additional bytes needed
        ADDITIONAL_BYTES=$((NEW_SIZE - CURRENT_SIZE))
        echo "Extending program account by $ADDITIONAL_BYTES bytes"

        # Extend the program account
        solana program extend "$PROGRAM_ID" "$ADDITIONAL_BYTES"
    else
        echo "No extension needed; deploying new version"
    fi

    # Deploy the new version
    solana program deploy "$PROGRAM_SO" \
        --program-id "$PROGRAM_KEYPAIR" \
        --fee-payer "$FEE_PAYER" \
        --upgrade-authority "$UPGRADE_AUTHORITY"
else
    # First-time deployment
    echo "Deploying new program with ID: $PROGRAM_ID"

    # Deploy the program
    solana program deploy "$PROGRAM_SO" \
        --program-id "$PROGRAM_KEYPAIR" \
        --fee-payer "$FEE_PAYER" \
        --upgrade-authority "$UPGRADE_AUTHORITY"
fi

# Record final balance
FINAL_BALANCE=$(get_balance)

# Calculate and display the total SOL used
SOL_USED=$(awk "BEGIN {print $INITIAL_BALANCE - $FINAL_BALANCE}")
echo "Total SOL used for this operation: $SOL_USED SOL"
