#!/bin/sh
# Bugs 安装脚本 — 兼容 bash / zsh / fish 等所有 POSIX Shell
set -e

VERSION="${BUGS_VERSION:-latest}"
BIN_DIR="${HOME}/.bugs/bin"

echo "🧠 Bugs v$VERSION 安装中..."

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)   ARCH="linux-x86_64" ;;
  Darwin-x86_64|Darwin-arm64) ARCH="macos" ;;
  *) echo "❌ 不支持的平台"; exit 1 ;;
esac

mkdir -p "$BIN_DIR"
cd "$BIN_DIR"

# 下载主二进制
curl -fsSL "https://github.com/bugs-ai/bugs/releases/download/v${VERSION}/bugs-${ARCH}" -o bugs 2>/dev/null || {
  echo "  ⚠️ 下载失败，从源码编译..."
  command -v cargo >/dev/null || { curl https://sh.rustup.rs -sSf | sh -s -- -y; source "$HOME/.cargo/env"; }
  cd /tmp && git clone https://github.com/bugs-ai/bugs && cd Bugs
  cargo build --release -p bugs -p bugs-api -p bugs-tui
  cp target/release/{bugs,bugs-api,bugs-tui} "$BIN_DIR/"
}
chmod +x "$BIN_DIR"/*

# 添加到 PATH
grep -q "$BIN_DIR" ~/.bashrc 2>/dev/null || echo "export PATH=\"$BIN_DIR:\$PATH\"" >> ~/.bashrc
grep -q "$BIN_DIR" ~/.zshrc 2>/dev/null || echo "export PATH=\"$BIN_DIR:\$PATH\"" >> ~/.zshrc

echo ""
echo "✅ 安装完成"
echo ""
echo "  启动:  bugs tui"
echo "  守护:  bugs start"
echo "  Web:   bugs web"
echo "  帮助:  bugs --help"
