#!/bin/sh
# Bugs 安装脚本 — 兼容 bash / zsh / fish / dash
# 用法:
#   curl -fsSL https://raw.githubusercontent.com/Indivisible2025/Bugs/main/scripts/install.sh | sh
#   curl -fsSL https://bugs.neaneu.top/install.sh | sh
#
# 环境变量:
#   BUGS_CHANNEL=stable|beta|dev  选择频道（默认 stable → main 分支）
#   BUGS_VERSION=v1.0.0           指定版本（默认 latest）
set -e

# ── 频道映射 ──────────────────────────────────────────────
CHANNEL="${BUGS_CHANNEL:-stable}"
case "$CHANNEL" in
  stable) BRANCH="main"  ;;
  beta)   BRANCH="beta"  ;;
  dev)    BRANCH="dev"   ;;
  *) echo "❌ 无效频道: $CHANNEL (可选: stable, beta, dev)" >&2; exit 1 ;;
esac

REPO="Indivisible2025/Bugs"
BIN_DIR="${HOME}/.bugs/bin"
VERSION="${BUGS_VERSION:-latest}"

echo "🧠 Bugs 安装中..."
echo "   频道: $CHANNEL → $BRANCH 分支"

# ── 平台检测 ──────────────────────────────────────────────
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)   ARCH="x86_64-unknown-linux-gnu";  OS="linux" ;;
  Darwin-x86_64)  ARCH="x86_64-apple-darwin";       OS="macos" ;;
  Darwin-arm64)   ARCH="aarch64-apple-darwin";      OS="macos" ;;
  *) echo "❌ 不支持的平台: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

mkdir -p "$BIN_DIR"

# ── 方式一：从 GitHub Releases 下载预编译二进制 ──────────
download_release() {
  if [ "$VERSION" = "latest" ]; then
    BASE="https://github.com/${REPO}/releases/latest/download"
  else
    BASE="https://github.com/${REPO}/releases/download/${VERSION}"
  fi

  echo "  → 下载预编译二进制..."
  for bin in bugs bugs-daemon bugs-tui; do
    # 先尝试带架构后缀，再尝试不带（兼容旧格式）
    curl -fsSL "${BASE}/${bin}-${ARCH}" -o "$BIN_DIR/$bin" 2>/dev/null || \
    curl -fsSL "${BASE}/${bin}-${OS}"   -o "$BIN_DIR/$bin" 2>/dev/null || \
    return 1
  done
  return 0
}

# ── 方式二：从源码编译 ────────────────────────────────────
compile_from_source() {
  echo "  → 从源码编译 (分支: $BRANCH)"

  # 确保 Rust 已安装
  if ! command -v cargo >/dev/null 2>&1; then
    echo "  → 安装 Rust 工具链..."
    curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable
    . "$HOME/.cargo/env"
  fi

  TMPDIR="$(mktemp -d)"
  trap 'rm -rf "$TMPDIR"' EXIT
  cd "$TMPDIR"

  git clone --branch "$BRANCH" --depth 1 "https://github.com/${REPO}.git" Bugs
  cd Bugs
  cargo build --release -p bugs -p bugs-api -p bugs-tui
  cp target/release/bugs target/release/bugs-daemon target/release/bugs-tui "$BIN_DIR/"
}

# ── 执行安装 ──────────────────────────────────────────────
download_release || compile_from_source
chmod +x "$BIN_DIR"/*

# ── 添加到 PATH ───────────────────────────────────────────
add_path() {
  local rc="$1" dir="$2"
  if [ -f "$rc" ]; then
    grep -q "$dir" "$rc" 2>/dev/null || {
      case "$rc" in
        *.fish) echo "fish_add_path $dir" >> "$rc" ;;
        *)      echo "export PATH=\"$dir:\$PATH\"" >> "$rc" ;;
      esac
    }
  fi
}
add_path "$HOME/.bashrc" "$BIN_DIR"
add_path "$HOME/.zshrc"  "$BIN_DIR"
add_path "$HOME/.config/fish/config.fish" "$BIN_DIR"

# ── 完成 ──────────────────────────────────────────────────
echo ""
echo "✅ Bugs 安装完成 (${CHANNEL} 频道)"
echo ""
echo "  启动:  bugs tui"
echo "  对话:  bugs"
echo "  守护:  bugs start"
echo "  帮助:  bugs --help"
echo ""
echo "  💡 运行 'exec \$SHELL' 或重新打开终端使 PATH 生效"
