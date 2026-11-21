#!/bin/bash
# deploy-contract.sh - Deploy AI Agent Marketplace contract to Stellar

set -euo pipefail

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Mainnet guardrail flag may appear anywhere in the argument list
MAINNET_ACK=false
ARGS=()
for arg in "$@"; do
    if [ "$arg" = "--i-understand-this-is-mainnet" ]; then
        MAINNET_ACK=true
    else
        ARGS+=("$arg")
    fi
done
set -- "${ARGS[@]+"${ARGS[@]}"}"

# Configuration
MODE="deploy"
if [ "${1:-}" = "upgrade" ] || [ "${1:-}" = "--upgrade" ]; then
    MODE="upgrade"
    shift
fi
NETWORK="${1:-testnet}"
SOURCE_ACCOUNT="${2:-deployer}"
CONTRACT_ALIAS="ai-agent-marketplace"
WASM_PATH="contracts/ai-agent-marketplace/target/wasm32-unknown-unknown/release/ai_agent_marketplace.wasm"
# Source account used for upgrades (multisig admin)
ADMIN_SOURCE="${ADMIN_SOURCE:-admin-multisig}"
# SHA-256 of the CI-built WASM artifact (required on mainnet)
EXPECTED_WASM_HASH="${EXPECTED_WASM_HASH:-}"

# Save a log of this deployment
LOG_DIR="deployments"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/deploy-$NETWORK-$(date -u +%Y%m%dT%H%M%SZ).log"
exec > >(tee -a "$LOG_FILE") 2>&1

echo -e "${GREEN}🚀 AI Agent Marketplace Contract ${MODE^}${NC}"
echo "Network: $NETWORK"
echo "Source account: $SOURCE_ACCOUNT"
echo ""

# Mainnet safety: require an explicit multisig admin account
if [ "$NETWORK" = "mainnet" ] && [ "$SOURCE_ACCOUNT" = "deployer" ]; then
    echo -e "${RED}❌ Mainnet deploys must use the multisig admin account.${NC}"
    echo "Usage: $0 mainnet <multisig-admin-account>"
    exit 1
fi

# Mainnet safety: require an explicit acknowledgement flag and the CI artifact hash
if [ "$NETWORK" = "mainnet" ]; then
    if [ "$MAINNET_ACK" != true ]; then
        echo -e "${RED}❌ Mainnet deploys require the --i-understand-this-is-mainnet flag.${NC}"
        echo "Usage: EXPECTED_WASM_HASH=<ci-sha256> $0 mainnet <multisig-admin-account> --i-understand-this-is-mainnet"
        exit 1
    fi
    if [ -z "$EXPECTED_WASM_HASH" ]; then
        echo -e "${RED}❌ Set EXPECTED_WASM_HASH to the SHA-256 of the CI-built WASM artifact.${NC}"
        exit 1
    fi
fi

# Check prerequisites
check_command() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${RED}❌ Required command '$1' not found. Please install it first.${NC}"
        exit 1
    fi
}

check_command cargo
check_command stellar
check_command wasm-opt

# Build contract
echo -e "${YELLOW}📦 Building contract...${NC}"
cd contracts/ai-agent-marketplace
cargo build --target wasm32-unknown-unknown --release

# Optimize WASM
echo -e "${YELLOW}⚡ Optimizing WASM...${NC}"
wasm-opt -Oz \
  target/wasm32-unknown-unknown/release/ai_agent_marketplace.wasm \
  -o target/wasm32-unknown-unknown/release/ai_agent_marketplace.opt.wasm

WASM_PATH="target/wasm32-unknown-unknown/release/ai_agent_marketplace.opt.wasm"
cd ../..

# Check WASM size
SIZE=$(wc -c < "$WASM_PATH")
echo -e "${GREEN}✅ WASM size: $((SIZE / 1024)) KB${NC}"

if [ "$SIZE" -gt 1000000 ]; then
    echo -e "${YELLOW}⚠️  Warning: WASM > 1MB, may exceed deployment limits${NC}"
fi

# Compute local WASM hash for post-deploy verification
LOCAL_WASM_HASH=$(sha256sum "$WASM_PATH" | awk '{print $1}')
echo -e "${GREEN}🔑 Local WASM SHA-256: $LOCAL_WASM_HASH${NC}"

# Print the resolved source account
SOURCE_ADDRESS=$(stellar keys address "$SOURCE_ACCOUNT" 2>/dev/null || echo "$SOURCE_ACCOUNT")
echo -e "${GREEN}👤 Source account: $SOURCE_ACCOUNT ($SOURCE_ADDRESS)${NC}"

# Verify the local build matches the CI-built artifact
if [ -n "$EXPECTED_WASM_HASH" ]; then
    if [ "$LOCAL_WASM_HASH" != "$EXPECTED_WASM_HASH" ]; then
        echo -e "${RED}❌ WASM hash does not match the CI-built artifact!${NC}"
        echo "  Local: $LOCAL_WASM_HASH"
        echo "  CI:    $EXPECTED_WASM_HASH"
        exit 1
    fi
    echo -e "${GREEN}✅ WASM hash matches the CI-built artifact${NC}"
fi

# Mainnet safety: typed confirmation before anything is sent
if [ "$NETWORK" = "mainnet" ]; then
    echo ""
    echo -e "${RED}⚠️  You are about to deploy to MAINNET.${NC}"
    echo "  Source account: $SOURCE_ACCOUNT ($SOURCE_ADDRESS)"
    echo "  WASM SHA-256:   $LOCAL_WASM_HASH"
    read -r -p "Type 'deploy to mainnet' to continue: " CONFIRM
    if [ "$CONFIRM" != "deploy to mainnet" ]; then
        echo -e "${RED}❌ Confirmation did not match. Aborting.${NC}"
        exit 1
    fi
fi

# Deploy
echo -e "${YELLOW}📤 Deploying to $NETWORK...${NC}"
CONTRACT_ID=$(stellar contract deploy \
  --wasm "$WASM_PATH" \
  --source-account "$SOURCE_ACCOUNT" \
  --network "$NETWORK" \
  --alias "$CONTRACT_ALIAS-$NETWORK" \
  2>&1 | tail -1)

echo -e "${GREEN}✅ Contract deployed: $CONTRACT_ID${NC}"

# Verify deployed WASM hash matches the local artifact
echo -e "${YELLOW}🔍 Verifying deployed WASM hash...${NC}"
DEPLOYED_WASM_HASH=$(stellar contract info wasm-hash \
  --id "$CONTRACT_ID" \
  --network "$NETWORK" \
  2>/dev/null | tail -1)

if [ -n "$DEPLOYED_WASM_HASH" ] && [ "$DEPLOYED_WASM_HASH" != "$LOCAL_WASM_HASH" ]; then
    echo -e "${RED}❌ WASM hash mismatch!${NC}"
    echo "  Local:    $LOCAL_WASM_HASH"
    echo "  Deployed: $DEPLOYED_WASM_HASH"
    exit 1
fi
echo -e "${GREEN}✅ WASM hash verified${NC}"

# Save contract ID
echo "$CONTRACT_ID" > ".contract-id-$NETWORK"
echo -e "${GREEN}📝 Contract ID saved to .contract-id-$NETWORK${NC}"

# Initialize immediately in the same session (issue #102).
# initialize() requires auth from the admin, so the admin must sign this
# transaction. Running it right after deploy closes the front-running window
# where an attacker could otherwise call initialize() first and choose the
# admin, fee recipient, resolver and token.
if [ "$MODE" = "deploy" ]; then
    echo ""
    echo -e "${YELLOW}🔐 Initializing contract (same session, admin must sign)...${NC}"
    echo "  Admin source: $SOURCE_ACCOUNT ($SOURCE_ADDRESS)"
    if ! ./scripts/init-contract.sh "$NETWORK" "$SOURCE_ACCOUNT"; then
        echo -e "${RED}❌ Initialization failed.${NC}"
        echo -e "${RED}⚠️  The contract is deployed but NOT initialized.${NC}"
        echo -e "${RED}⚠️  Run ./scripts/init-contract.sh $NETWORK $SOURCE_ACCOUNT IMMEDIATELY.${NC}"
        echo -e "${RED}⚠️  Until then anyone can front-run initialize() and seize admin control.${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Contract initialized${NC}"
fi

echo ""
echo -e "${GREEN}🎉 Deployment complete!${NC}"
echo -e "${GREEN}📝 Deployment log saved to $LOG_FILE${NC}"
echo ""
echo -e "${YELLOW}📋 Post-deploy verification checklist (record results for the rehearsal):${NC}"
echo "  [ ] Contract ID: $CONTRACT_ID"
echo "  [ ] Admin set to multisig account: $SOURCE_ACCOUNT"
echo "  [ ] WASM hash matches local artifact: $LOCAL_WASM_HASH"
echo "  [ ] initialize() executed in the SAME session as deploy (no gap)"
echo "  [ ] initialize() transaction signed by the admin account"
echo "  [ ] Caps configured and verified on-chain"
echo "  [ ] Pause drill completed (admin can pause and unpause)"
echo "  [ ] Upgrade drill completed (admin can upgrade to new WASM)"
echo "  [ ] Reconciliation of balances/state against expected values"
echo ""
echo "Next steps:"
echo "1. Update frontend .env.local with contract ID"
echo "2. Deploy frontend: ./scripts/deploy-frontend.sh"
echo "3. Schedule the TTL keeper to keep escrow/job entries alive:"
echo "   ./scripts/keeper.sh $NETWORK"
echo "   (run periodically via cron or a systemd timer, e.g. every 12 hours)"
