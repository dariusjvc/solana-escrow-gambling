#!/bin/bash
set -euo pipefail

# =========================
# Config
# =========================
CLUSTER="${CLUSTER:-https://api.devnet.solana.com}"
WALLETS_DIR="./wallets"
mkdir -p "$WALLETS_DIR"

echo
echo "=> Usando cluster: $CLUSTER"
solana config set --url "$CLUSTER" >/dev/null
solana config get

# =========================
# PAYER: usa tu id.json existente
# =========================
PAYER_KEYPAIR="$HOME/.config/solana/id.json"
if [ ! -f "$PAYER_KEYPAIR" ]; then
  echo "❌ No existe $PAYER_KEYPAIR. Crea la wallet con: solana-keygen new"
  exit 1
fi
PAYER_PK=$(solana-keygen pubkey "$PAYER_KEYPAIR")
echo "PAYER_PK:    $PAYER_PK"

# =========================
# PLAYER2 keypair (se crea si no existe)
# =========================
PLAYER2_KEYPAIR="$WALLETS_DIR/player2.json"
if [ ! -f "$PLAYER2_KEYPAIR" ]; then
  echo "🔐 Generando keypair: player2.json"
  solana-keygen new --no-bip39-passphrase --outfile "$PLAYER2_KEYPAIR" >/dev/null
fi
PLAYER2_PK=$(solana-keygen pubkey "$PLAYER2_KEYPAIR")
echo "PLAYER2_PK:  $PLAYER2_PK"

# =========================
# ESCROW authority keypair (se crea si no existe)
# =========================
ESCROW_KEYPAIR="$WALLETS_DIR/escrow.json"
if [ ! -f "$ESCROW_KEYPAIR" ]; then
  echo "🔐 Generando keypair: escrow.json"
  solana-keygen new --no-bip39-passphrase --outfile "$ESCROW_KEYPAIR" >/dev/null
fi
ESCROW_AUTH=$(solana-keygen pubkey "$ESCROW_KEYPAIR")
echo "ESCROW_AUTH: $ESCROW_AUTH"

# =========================
# Crear MINT (6 decimales)
# =========================
echo "🪙 Creando mint (decimals=6)…"
CREATE_OUT=$(spl-token create-token --decimals 6 --fee-payer "$PAYER_KEYPAIR")
# Extraer el mint (primer base58 que no sea el ProgramId)
MINT=$(echo "$CREATE_OUT" | grep -Eo '([1-9A-HJ-NP-Za-km-z]{32,44})' \
  | grep -v '^TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA$' | head -n1)
if [[ -z "$MINT" ]]; then
  echo "❌ No pude extraer el MINT:"
  echo "$CREATE_OUT"
  exit 1
fi
echo "MINT:        $MINT"

# =========================
# Helpers
# =========================
calc_ata() {
  local mint="$1"
  local owner="$2"
  # La última línea del --verbose es la dirección del ATA
  spl-token address --token "$mint" --owner "$owner" --verbose \
    | grep -Eo '([1-9A-HJ-NP-Za-km-z]{32,44})' | tail -n1
}

create_ata() {
  local mint="$1"
  local owner="$2"
  local fee_payer="$3"

  # Crea la ATA para "owner" pagando con "fee_payer"
  # Si ya existe, el comando devuelve error: lo ignoramos con "|| true"
  spl-token create-account "$mint" \
    --owner "$owner" \
    --fee-payer "$fee_payer" >/dev/null 2>&1 || true
}

mint_to() {
  local mint="$1"
  local raw_amount="$2"
  local recipient_ata="$3"
  local fee_payer="$4"

  spl-token mint "$mint" "$raw_amount" "$recipient_ata" \
    --fee-payer "$fee_payer" >/dev/null
}

# =========================
# Calcular ATAs determinísticas
# =========================
echo "📫 Calculando ATAs…"
PAYER_ATA=$(calc_ata "$MINT" "$PAYER_PK")
PLAYER2_ATA=$(calc_ata "$MINT" "$PLAYER2_PK")
ESCROW_ATA=$(calc_ata "$MINT" "$ESCROW_AUTH")
echo "PAYER_ATA:   $PAYER_ATA"
echo "PLAYER2_ATA: $PLAYER2_ATA"
echo "ESCROW_ATA:  $ESCROW_ATA"

# =========================
# Crear ATAs (payer como fee-payer en todos)
# =========================
echo "📫 Creando ATAs…"
create_ata "$MINT" "$PAYER_PK"   "$PAYER_KEYPAIR"
create_ata "$MINT" "$PLAYER2_PK" "$PAYER_KEYPAIR"
create_ata "$MINT" "$ESCROW_AUTH" "$PAYER_KEYPAIR"

# =========================
# Mint tokens
# =========================
# UI amounts → RAW con 6 decimales
MINT_AMOUNT_UI_PAYER=${MINT_AMOUNT_UI_PAYER:-100000}  # 100k
MINT_AMOUNT_UI_P2=${MINT_AMOUNT_UI_P2:-2000}         # 2k

RAW_PAYER=$(python3 - <<PY
print(int("$MINT_AMOUNT_UI_PAYER")*(10**6))
PY
)

RAW_P2=$(python3 - <<PY
print(int("$MINT_AMOUNT_UI_P2")*(10**6))
PY
)

echo "🪙 Mint a payer:   $MINT_AMOUNT_UI_PAYER (raw=$RAW_PAYER)"
mint_to "$MINT" "$RAW_PAYER" "$PAYER_ATA" "$PAYER_KEYPAIR"

echo "🪙 Mint a player2: $MINT_AMOUNT_UI_P2 (raw=$RAW_P2)"
mint_to "$MINT" "$RAW_P2" "$PLAYER2_ATA" "$PAYER_KEYPAIR"

# =========================
# Escribir .env para test.ts
# =========================
cat > .env <<EOL
# Generado por test.sh
CLUSTER=$CLUSTER

# Wallets
PATH_TO_YOUR_SOLANA_PAYER_JSON=$PAYER_KEYPAIR
PATH_TO_YOUR_SOLANA_PLAYER2_JSON=$PLAYER2_KEYPAIR
PATH_TO_YOUR_SOLANA_GAME_JSON=$ESCROW_KEYPAIR

# Program (rellena si hace falta)
DEPLOYED_PROGRAM_ADDRESS=${DEPLOYED_PROGRAM_ADDRESS:-}

# Mint & ATAs
MINT=$MINT
PAYER_TOKEN_ACCOUNT=$PAYER_ATA
PLAYER2_TOKEN_ACCOUNT=$PLAYER2_ATA
SCROW_TOKEN_ACCOUNT=$ESCROW_ATA

# Pyth (ETH/USDC devnet)
PYTH_PRICE_ETH_USDC=EdVCmQ9FSPcVe5YySXDPCRmc8aDQLKJ9xvYBMZPie1Vw
EOL

echo "✅ Listo. Variables guardadas en .env"
echo "   - MINT:                 $MINT"
echo "   - PAYER_TOKEN_ACCOUNT:  $PAYER_ATA"
echo "   - PLAYER2_TOKEN_ACCOUNT:$PLAYER2_ATA"
echo "   - SCROW_TOKEN_ACCOUNT:  $ESCROW_ATA"
