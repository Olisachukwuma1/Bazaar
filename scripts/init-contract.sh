#!/bin/bash
# Initialize the AI Agent Marketplace contract
# Usage: ./scripts/init-contract.sh [testnet|mainnet|futurenet]

set -euo pipefail

NETWORK="${1:-testnet}"
CONTRACT_ALIAS="ai-agent-marketplace"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Check prerequisites
if ! command -v stellar &> /dev/null; then
    echo -e "${RED}❌ Required command 'stellar' not found. Install the Stellar CLI first.${NC}"
    exit 1
fi

# Load contract ID
CONTRACT_FILE=".contract-id-$NETWORK"
if [ ! -f "$CONTRACT_FILE" ]; then
    echo -e "${RED}❌ Contract ID file not found: $CONTRACT_FILE${NC}"
    echo "Run deploy-contract.sh first"
    exit 1
fi

CONTRACT_ID=$(cat "$CONTRACT_FILE")
echo -e "${YELLOW}🔧 Initializing contract $CONTRACT_ID on $NETWORK...${NC}"

# Network-specific config
case "$NETWORK" in
    testnet)
        RPC_URL="https://soroban-testnet.stellar.org"
        NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
        USDC_CONTRACT="CBIELSO6KZLFBUSGZ47QRYCYMHK4D2H5IMKBORJGUJVAVKZOQK4D7IAJ"
        ;;
    mainnet)
        RPC_URL="https://soroban-mainnet.stellar.org"
        NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
        USDC_CONTRACT="CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75"
        ;;
    futurenet)
        RPC_URL="https://rpc-futurenet.stellar.org"
        NETWORK_PASSPHRASE="Test SDF Future Network ; October 2022"
        USDC_CONTRACT="CBIELSO6KZLFBUSGZ47QRYCYMHK4D2H5IMKBORJGUJVAVKZOQK4D7IAJ"
        ;;
    *)
        echo -e "${RED}❌ Unknown network: $NETWORK${NC}"
        exit 1
        ;;
esac

# Resolve admin address: prefer explicit ADMIN_ADDRESS (e.g. multisig), else deployer key
# NOTE: initialize() now requires admin.require_auth(). The invocation below is signed
# by the admin's source account (ADMIN_SOURCE). Because initialize must be authorized by
# the admin, deploy + initialize should be run atomically (e.g. via deploy-contract.sh
# or a single script) to avoid a front-runner initializing with their own admin.
if [ -n "${ADMIN_ADDRESS:-}" ]; then
    ADMIN="$ADMIN_ADDRESS"
    ADMIN_SOURCE="${ADMIN_SOURCE:-deployer}"
    echo -e "${YELLOW}👤 Using admin from ADMIN_ADDRESS: $ADMIN${NC}"
    echo -e "${YELLOW}🔑 Signing initialize() with source account: $ADMIN_SOURCE${NC}"
else
    ADMIN_SOURCE="${ADMIN_SOURCE:-deployer}"
    ADMIN=$(stellar keys address "$ADMIN_SOURCE")
    echo -e "${YELLOW}👤 Using key '$ADMIN_SOURCE' as admin: $ADMIN${NC}"
fi

# Conservative caps: tighter on mainnet, looser on test networks
case "$NETWORK" in
    mainnet)
        PLATFORM_FEE_BPS=100
        MIN_JOB_BUDGET=10000000
        MAX_JOB_DEADLINE_DAYS=30
        DISPUTE_WINDOW_DAYS=7
        PLATFORM_FEE_BPS_DISPUTE=200
        ESCROW_EXPIRY_DAYS=30
        REQUIRE_SKILL_VERIFICATION=true
        ;;
    *)
        PLATFORM_FEE_BPS=100
        MIN_JOB_BUDGET=10000000
        MAX_JOB_DEADLINE_DAYS=30
        DISPUTE_WINDOW_DAYS=7
        PLATFORM_FEE_BPS_DISPUTE=200
        ESCROW_EXPIRY_DAYS=30
        REQUIRE_SKILL_VERIFICATION=false
        ;;
esac

# Validate config values before invoking (mirrors contract validate_config rules)
MIN_AGENT_REPUTATION=0
MIN_AGENT_REPUTATION_TO_BID=0

validate_positive() {
    local name="$1" value="$2"
    if ! [[ "$value" =~ ^[0-9]+$ ]] || [ "$value" -le 0 ]; then
        echo -e "${RED}❌ $name must be a positive integer (got: $value)${NC}"
        exit 1
    fi
}

validate_bps() {
    local name="$1" value="$2"
    if ! [[ "$value" =~ ^[0-9]+$ ]] || [ "$value" -gt 10000 ]; then
        echo -e "${RED}❌ $name must be an integer in 0..=10000 (got: $value)${NC}"
        exit 1
    fi
}

validate_reputation() {
    local name="$1" value="$2"
    if ! [[ "$value" =~ ^[0-9]+$ ]] || [ "$value" -gt 100 ]; then
        echo -e "${RED}❌ $name must be an integer in 0..=100 (got: $value)${NC}"
        exit 1
    fi
}

validate_bps "PLATFORM_FEE_BPS" "$PLATFORM_FEE_BPS"
validate_bps "PLATFORM_FEE_BPS_DISPUTE" "$PLATFORM_FEE_BPS_DISPUTE"
validate_positive "MIN_JOB_BUDGET" "$MIN_JOB_BUDGET"
validate_positive "MAX_JOB_DEADLINE_DAYS" "$MAX_JOB_DEADLINE_DAYS"
validate_positive "DISPUTE_WINDOW_DAYS" "$DISPUTE_WINDOW_DAYS"
validate_positive "ESCROW_EXPIRY_DAYS" "$ESCROW_EXPIRY_DAYS"
validate_reputation "MIN_AGENT_REPUTATION" "$MIN_AGENT_REPUTATION"
validate_reputation "MIN_AGENT_REPUTATION_TO_BID" "$MIN_AGENT_REPUTATION_TO_BID"

# #160: On mainnet, refuse non-canonical USDC token contracts.
MAINNET_USDC="CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75"
if [ "$NETWORK" = "mainnet" ]; then
    if [ "$USDC_CONTRACT" != "$MAINNET_USDC" ]; then
        echo -e "${RED}❌ Mainnet init requires canonical Circle USDC: $MAINNET_USDC${NC}"
        exit 1
    fi
    echo -e "${YELLOW}🔎 Verifying USDC token metadata (decimals/symbol)...${NC}"
    META=$(stellar contract invoke --id "$USDC_CONTRACT" --network mainnet --rpc-url "$RPC_URL" --network-passphrase "$NETWORK_PASSPHRASE" -- decimals 2>/dev/null || echo "")
    if [ -n "$META" ] && [ "$META" != "7" ]; then
        echo -e "${RED}❌ USDC decimals must be 7 on mainnet${NC}"
        exit 1
    fi
fi

# Initialize (signed by the admin source account so admin.require_auth() passes)
echo -e "${YELLOW}⚙️  Calling initialize()...${NC}"
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --source-account "$ADMIN_SOURCE" \
  --network "$NETWORK" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- \
  initialize \
  --admin "$ADMIN" \
  --usdc_token "$USDC_CONTRACT" \
  --platform_fee_bps "$PLATFORM_FEE_BPS" \
  --min_job_budget "$MIN_JOB_BUDGET" \
  --max_job_deadline_days "$MAX_JOB_DEADLINE_DAYS" \
  --dispute_window_days "$DISPUTE_WINDOW_DAYS" \
  --min_agent_reputation "$MIN_AGENT_REPUTATION" \
  --platform_fee_recipient "$ADMIN" \
  --dispute_resolver "$ADMIN" \
  --min_agent_reputation_to_bid "$MIN_AGENT_REPUTATION_TO_BID" \
  --platform_fee_bps_dispute "$PLATFORM_FEE_BPS_DISPUTE" \
  --escrow_expiry_days "$ESCROW_EXPIRY_DAYS" \
  --skill_verifiers '[]' \
  --require_skill_verification "$REQUIRE_SKILL_VERIFICATION"

echo -e "${GREEN}✅ Contract initialized successfully!${NC}"
echo ""
echo "Configuration (network: $NETWORK):"
echo "  Contract ID: $CONTRACT_ID"
echo "  Admin: $ADMIN"
echo "  Admin Source: $ADMIN_SOURCE"
echo "  USDC: $USDC_CONTRACT"
echo "  Platform Fee: $PLATFORM_FEE_BPS bps"
echo "  Min Job Budget: $MIN_JOB_BUDGET (stroops)"
echo "  Max Deadline: $MAX_JOB_DEADLINE_DAYS days"
echo "  Dispute Window: $DISPUTE_WINDOW_DAYS days"
echo "  Dispute Fee: $PLATFORM_FEE_BPS_DISPUTE bps"
echo "  Escrow Expiry: $ESCROW_EXPIRY_DAYS days"
echo "  Require Skill Verification: $REQUIRE_SKILL_VERIFICATION"
