#!/bin/bash
set -euo pipefail

# Cluster
solana config set --url https://api.devnet.solana.com >/dev/null

# Routes
PROGRAM_SO=./program/target/so/escrow_program.so
PROGRAM_KEYPAIR=./program/target/so/escrow_program-keypair.json

# Fee payer / upgrade authority
DEVNET_FEE_PAYER="$HOME/.config/solana/id.json"
UPGRADE_AUTHORITY="$HOME/.config/solana/id.json"

# Make sure that exist fee payer
if [ ! -f "$DEVNET_FEE_PAYER" ]; then
  echo "Doesn't exist $DEVNET_FEE_PAYER"
  exit 1
fi

# Set the key pair by defult (id.json)
solana config set --keypair "$DEVNET_FEE_PAYER" >/dev/null

# 0) Program keypair (if it doesn't exist)
if [ ! -f "$PROGRAM_KEYPAIR" ]; then
  echo "Generating program keypair: $PROGRAM_KEYPAIR"
  mkdir -p "$(dirname "$PROGRAM_KEYPAIR")"
  solana-keygen new --no-bip39-passphrase --outfile "$PROGRAM_KEYPAIR" >/dev/null
fi

# 1) Compiling
echo "Compiling...SBF..."
cargo build-sbf --manifest-path=./program/Cargo.toml --sbf-out-dir=./program/target/so

# 2) Verify .so
if [ ! -f "$PROGRAM_SO" ]; then
  echo "Was not found .so in: $PROGRAM_SO"
  exit 1
fi

# 3) Program ID
PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR")
echo "Program ID: $PROGRAM_ID"

# 4) Initial balance
FEE_PAYER_PK=$(solana-keygen pubkey "$DEVNET_FEE_PAYER")
get_balance() { solana balance "$FEE_PAYER_PK" | awk '{print $1}'; }
INITIAL_BALANCE=$(get_balance)

# 5) Deploy o upgrade
if solana program show "$PROGRAM_ID" &>/dev/null; then
  echo "Upgrading existing program: $PROGRAM_ID"

  CURRENT_SIZE=$(solana program show "$PROGRAM_ID" | grep 'Data Length' | awk '{print $3}')
  NEW_SIZE=$(stat -c%s "$PROGRAM_SO")

  if [ "$NEW_SIZE" -gt "$CURRENT_SIZE" ]; then
    ADDITIONAL_BYTES=$((NEW_SIZE - CURRENT_SIZE))
    echo "Extending program account by $ADDITIONAL_BYTES bytes..."
    solana program extend "$PROGRAM_ID" "$ADDITIONAL_BYTES" \
      --fee-payer "$DEVNET_FEE_PAYER"
  else
    echo "Is not necessary to extend the program account"
  fi

  set +e
  solana program deploy "$PROGRAM_SO" \
    --program-id "$PROGRAM_KEYPAIR" \
    --fee-payer "$DEVNET_FEE_PAYER" \
    --upgrade-authority "$UPGRADE_AUTHORITY"
  DEPLOY_RC=$?
  set -e

  if [ $DEPLOY_RC -ne 0 ]; then
    echo "Deploy failure. retrying..."
    solana program deploy "$PROGRAM_SO" \
      --program-id "$PROGRAM_KEYPAIR" \
      --fee-payer "$DEVNET_FEE_PAYER" \
      --upgrade-authority "$UPGRADE_AUTHORITY"
  fi
else
  echo "Initial deploy of the program: $PROGRAM_ID"
  solana program deploy "$PROGRAM_SO" \
    --program-id "$PROGRAM_KEYPAIR" \
    --fee-payer "$DEVNET_FEE_PAYER" \
    --upgrade-authority "$UPGRADE_AUTHORITY"
fi

# Add variable to the .env file
if grep -q "^DEPLOYED_PROGRAM_ADDRESS=" .env; then
  sed -i "s/^DEPLOYED_PROGRAM_ADDRESS=.*/DEPLOYED_PROGRAM_ADDRESS=$PROGRAM_ID/" .env
else
  echo "DEPLOYED_PROGRAM_ADDRESS=$PROGRAM_ID" >> .env
fi


# 6) Balance
FINAL_BALANCE=$(get_balance)
SOL_USED=$(awk "BEGIN {print $INITIAL_BALANCE - $FINAL_BALANCE}")
echo "Total SOL used: $SOL_USED SOL"
