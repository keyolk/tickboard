#!/bin/bash
set -euo pipefail

APP_DIR="$(cd "$(dirname "$0")" && pwd)"
ENV_FILE="$APP_DIR/.env"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

print_banner() {
    echo ""
    echo -e "${CYAN}╔══════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║         tickboard Rust Installer         ║${NC}"
    echo -e "${CYAN}║       US & KR Markets Monitor            ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════╝${NC}"
    echo ""
}

ensure_rust() {
    if command -v cargo >/dev/null 2>&1; then
        echo "  → Rust: $(rustc --version)"
        return
    fi

    echo -e "${YELLOW}  Rust toolchain이 필요합니다. rustup으로 설치합니다...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
    echo "  → Rust: $(rustc --version)"
}

print_banner

echo -e "${GREEN}[1/4] Rust toolchain 확인${NC}"
ensure_rust

echo -e "${GREEN}[2/4] AWS Bedrock 설정 (선택사항)${NC}"
echo -e "  BEDROCK_API_KEY를 입력하거나, AWS_PROFILE/AWS_ACCESS_KEY_ID를 이미 설정했다면 Enter를 누르세요."
read -rsp "  Bedrock API Key: " BEDROCK_KEY
echo ""

if [[ -n "$BEDROCK_KEY" ]]; then
    cat > "$ENV_FILE" <<ENVEOF
BEDROCK_API_KEY=${BEDROCK_KEY}
BEDROCK_REGION=us-east-1
BEDROCK_MODEL_ID=us.anthropic.claude-sonnet-4-6
ENVEOF
    chmod 600 "$ENV_FILE"
    echo -e "  ${GREEN}✓ Bedrock API Key 설정 완료${NC}"
elif [[ ! -f "$ENV_FILE" ]]; then
    cat > "$ENV_FILE" <<ENVEOF
# BEDROCK_API_KEY=your-api-key
BEDROCK_REGION=us-east-1
BEDROCK_MODEL_ID=us.anthropic.claude-sonnet-4-6
ENVEOF
    echo -e "  ${YELLOW}⚠ Bedrock API Key 미설정 — AWS CLI/IAM credentials가 없으면 AI 기능이 비활성화됩니다.${NC}"
else
    echo "  → 기존 .env 유지"
fi

echo -e "${GREEN}[3/4] Release build${NC}"
cd "$APP_DIR"
make setup

echo -e "${GREEN}[4/4] Verification${NC}"
make smoke

echo ""
echo -e "${CYAN}╔══════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║          설치 완료!                       ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════╝${NC}"
echo ""
echo -e "  실행: ${GREEN}make run${NC} 또는 ${GREEN}./run.sh${NC}"
echo -e "  검증: ${GREEN}make verify${NC}"
echo -e "  단축키: Q(종료) R(새로고침) Tab(섹션이동) A(AI분석) B/Esc(뒤로)"
echo ""
