#!/bin/bash

cargo build-bpf --manifest-path=./program/Cargo.toml --bpf-out-dir=./program/target/so
solana program deploy ./program/target/so/escrow_program.so --fee-payer ./wallets/payer.json 