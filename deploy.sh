#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# cx-switch 发布脚本
# 用法: bash scripts/deploy.sh [版本号]
# 示例: bash scripts/deploy.sh v0.2.0   # 指定版本号
#       bash scripts/deploy.sh          # 自动递增 patch 版本号
# ============================================================

C_RESET=$'\033[0m'
C_BOLD=$'\033[1m'
C_GREEN=$'\033[32m'
C_YELLOW=$'\033[33m'
C_CYAN=$'\033[36m'
C_RED=$'\033[31m'

print_success() { printf "%b\n" "${C_BOLD}${C_GREEN}✅ $*${C_RESET}"; }
print_info()    { printf "%b\n" "${C_CYAN}📌 $*${C_RESET}"; }
print_error()   { printf "%b\n" "${C_BOLD}${C_RED}❌ $*${C_RESET}"; }

echo ""

# 计算默认版本号（基于最新 tag 自动 patch +1）
LATEST_TAG="$(git tag --sort=-version:refname | head -n 1 || echo "")"

if [[ -z "${LATEST_TAG}" ]]; then
  DEFAULT_TAG="v0.1.0"
  print_info "首次发布，默认版本号: ${DEFAULT_TAG}"
else
  print_info "当前最新版本: ${LATEST_TAG}"
  VERSION="${LATEST_TAG#v}"
  IFS='.' read -r MAJOR MINOR PATCH <<< "${VERSION}"
  PATCH=$((PATCH + 1))
  DEFAULT_TAG="v${MAJOR}.${MINOR}.${PATCH}"
fi

# 交互式输入版本号，回车使用默认值
printf "${C_CYAN}请输入版本号${C_RESET} [${C_BOLD}${C_GREEN}${DEFAULT_TAG}${C_RESET}]: "
read -r INPUT_TAG

if [[ -z "${INPUT_TAG}" ]]; then
  NEXT_TAG="${DEFAULT_TAG}"
else
  NEXT_TAG="${INPUT_TAG}"
  # 自动补 v 前缀
  if [[ "${NEXT_TAG}" != v* ]]; then
    NEXT_TAG="v${NEXT_TAG}"
  fi
fi

print_info "将要发布版本: ${NEXT_TAG}"

# 检查 tag 是否已存在
if git rev-parse "${NEXT_TAG}" >/dev/null 2>&1; then
  print_error "Tag ${NEXT_TAG} 已存在！请指定其他版本号。"
  exit 1
fi

# 同步版本号到 Cargo.toml
CARGO_VERSION="${NEXT_TAG#v}"
print_info "同步版本号 ${CARGO_VERSION} 到 Cargo.toml..."

if [[ "$(uname -s)" == "Darwin" ]]; then
  # macOS 的 sed 需要 -i ''
  sed -i '' "s/^version = \".*\"/version = \"${CARGO_VERSION}\"/" Cargo.toml
else
  sed -i "s/^version = \".*\"/version = \"${CARGO_VERSION}\"/" Cargo.toml
fi

# 同步 Cargo.lock
cargo generate-lockfile 2>/dev/null || true

# 提交版本号变更（仅在有实际变更时）
git add Cargo.toml Cargo.lock
if git diff --cached --quiet; then
  print_info "版本号未变化，跳过 commit"
else
  git commit -m "chore: 发布 ${NEXT_TAG}，同步版本号到 ${CARGO_VERSION}"
fi

# 在最新 commit 上创建 tag 并推送
git tag "${NEXT_TAG}"
git push origin HEAD "${NEXT_TAG}"

echo ""
print_success "Tag ${NEXT_TAG} 已创建并推送！"
print_info "Cargo.toml 版本号已同步为 ${CARGO_VERSION}"
print_info "GitHub Actions 正在自动构建..."
print_info "查看进度: https://github.com/jay6697117/cx-switch/actions"
echo ""
