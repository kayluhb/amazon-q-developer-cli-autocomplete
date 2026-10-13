#!/usr/bin/env bash

set -e

OS="$(uname -s)"

install_macos_deps() {
  echo "Detected macOS"
  xcode-select --install || true
  brew install mise pnpm protobuf zsh bash fish shellcheck jq
}

install_rust() {
  echo "Installing Rust toolchain..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  # This script runs in bash, whatever the login shell is
  . "$HOME/.cargo/env"

  rustup default stable
  # Pinned: newer nightlies reformat the existing code
  rustup toolchain install nightly-2025-11-01 --component rustfmt
  cargo install typos-cli

  if [[ "$OS" == "Darwin" ]]; then
    rustup target add x86_64-apple-darwin
    rustup target add aarch64-apple-darwin
  fi
}
add_mise_to_shell() {
  echo "Adding mise integration to shell..."

  SHELL_NAME=$(basename "$SHELL")

  case "$SHELL_NAME" in
    zsh)
      ZSHRC="${ZDOTDIR:-$HOME}/.zshrc"
      grep -qxF 'eval "$(mise activate zsh)"' "$ZSHRC" || echo 'eval "$(mise activate zsh)"' >> "$ZSHRC"
      ;;
    bash)
      BASHRC="$HOME/.bashrc"
      grep -qxF 'eval "$(mise activate bash)"' "$BASHRC" || echo 'eval "$(mise activate bash)"' >> "$BASHRC"
      ;;
    fish)
      FISH_CONFIG="$HOME/.config/fish/config.fish"
      mkdir -p "$(dirname "$FISH_CONFIG")"
      grep -qxF 'mise activate fish | source' "$FISH_CONFIG" || echo 'mise activate fish | source' >> "$FISH_CONFIG"
      ;;
    *)
      echo "⚠️  Unknown shell '$SHELL_NAME'. Please add mise manually to your shell config."
      ;;
  esac
}

setup_mise() {
  echo "Installing Python and Node with mise..."
  add_mise_to_shell
  mise trust
  mise install
}

setup_precommit() {
  echo "Installing pre-commit hooks..."
  pnpm install --ignore-scripts
}

echo "Setting up project dependencies..."

if [[ "$OS" == "Darwin" ]]; then
  install_macos_deps
else
  echo "Unsupported OS: $OS (only macOS is supported)"
  exit 1
fi

install_rust
setup_mise
setup_precommit

echo "✅ Setup complete! Follow the instructions in the README to get started."
