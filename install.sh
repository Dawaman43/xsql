#!/usr/bin/env sh
set -eu

ESC="\033"
GREEN="$ESC[32m"
YELLOW="$ESC[33m"
BLUE="$ESC[34m"
BOLD="$ESC[1m"
RESET="$ESC[0m"

info() { printf "%b\n" "$BLUE$1$RESET"; }
ok() { printf "%b\n" "$GREEN$1$RESET"; }
warn() { printf "%b\n" "$YELLOW$1$RESET"; }

printf "%b\n" "$BOLD Installing xsql — SQL dialect converter$RESET"

if ! command -v cargo >/dev/null 2>&1; then
  warn "cargo not found. Installing rustup + cargo (non-interactive)..."
  if command -v curl >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck disable=SC1090
    [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
  else
    printf "%b\n" "$YELLOW cargo is required but curl is not available. Install Rust manually: https://rustup.rs/$RESET"
    exit 1
  fi
fi

TMPDIR=$(mktemp -d 2>/dev/null || mktemp -d -t xsql)
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

info "Cloning repository to temporary directory..."
git clone --depth=1 https://github.com/Dawaman43/xsql.git "$TMPDIR" >/dev/null 2>&1 || {
  warn "git clone failed; trying curl+tar fallback..."
  curl -fsSL https://github.com/Dawaman43/xsql/archive/refs/heads/main.tar.gz | tar xz -C "$TMPDIR" --strip-components=1
}

info "Building and installing xsql (this may take a minute)..."
if command -v cargo >/dev/null 2>&1; then
  cargo install --path "$TMPDIR/crates/xsql-cli" --force || {
    warn "cargo install failed. See output above for errors.";
    exit 1
  }
else
  warn "cargo not available after rustup install. Aborting.";
  exit 1
fi

if ! echo "$PATH" | grep -q "$HOME/.cargo/bin"; then
  info "Adding $HOME/.cargo/bin to your PATH in ~/.profile"
  printf "\n# xsql installed: add cargo bin to PATH\nexport PATH=\"$HOME/.cargo/bin:\$PATH\"\n" >> "$HOME/.profile"
  # try to update current shell PATH
  export PATH="$HOME/.cargo/bin:$PATH"
fi

ok "xsql installed successfully"
printf "%b\n" "\nRun the tool: ${BOLD}${GREEN}xsql --help${RESET}\n"

exit 0
