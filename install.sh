#!/usr/bin/env sh
set -eu

ESC="\033"
GREEN="$ESC[32m"
YELLOW="$ESC[33m"
BLUE="$ESC[34m"
BOLD="$ESC[1m"
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

spinner() {
  # $1 = PID
  i=0
  sp="|/-\\"
  while kill -0 "$1" 2>/dev/null; do
    printf "\r%s %s" "${sp:i%4:1}" "$2"
    i=$((i+1))
    sleep 0.1
  done
  printf "\r" # clear
}

printf "%b\n" "$BOLD Installing xsql — SQL dialect converter$RESET"

TMPDIR=$(mktemp -d 2>/dev/null || mktemp -d -t xsql)
cleanup() { rm -rf "$TMPDIR"; }
trap cleanup EXIT

info "Cloning repository to temporary directory..."
if ! git clone --depth=1 https://github.com/Dawaman43/xsql.git "$TMPDIR" >/dev/null 2>&1; then
  warn "git clone failed; fetching archive..."
  if ! curl -fsSL https://github.com/Dawaman43/xsql/archive/refs/heads/main.tar.gz | tar xz -C "$TMPDIR" --strip-components=1 >/dev/null 2>&1; then
    warn "Failed to download repository. Aborting."
    exit 1
  fi
fi

# Ensure cargo/rustup exists
if ! command -v cargo >/dev/null 2>&1; then
  warn "cargo not found. Installing rustup + cargo (non-interactive)..."
  if command -v curl >/dev/null 2>&1; then
    (curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y) >/dev/null 2>&1 &
    RPID=$!
    spinner "$RPID" "Installing Rust toolchain..."
    wait "$RPID" || {
      warn "rustup install failed. Check your network and try again.";
      exit 1
    }
    [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
  else
    printf "%b\n" "$YELLOW cargo is required but curl is not available. Install Rust manually: https://rustup.rs/$RESET"
    exit 1
  fi
fi

info "Building and installing xsql (this may take a minute)..."
# run cargo install quietly and show spinner
LOG="$TMPDIR/build.log"
(cd "$TMPDIR" && cargo install --path crates/xsql-cli --force >"$LOG" 2>&1) &
CPID=$!
spinner "$CPID" "Building xsql..."
wait "$CPID" || {
  warn "Build/install failed. Showing last 200 lines of log:";
  tail -n 200 "$LOG" >&2 || true
  exit 1
}

# Ensure PATH contains cargo bin permanently
if ! echo "$PATH" | grep -q "$HOME/.cargo/bin"; then
  if [ -f "$HOME/.zshrc" ]; then
    rc="$HOME/.zshrc"
  elif [ -f "$HOME/.bashrc" ]; then
    rc="$HOME/.bashrc"
  else
    rc="$HOME/.profile"
  fi
  info "Adding $HOME/.cargo/bin to your PATH in $rc"
  if ! grep -q "$HOME/.cargo/bin" "$rc" 2>/dev/null; then
    printf "\n# xsql installed: add cargo bin to PATH\nexport PATH=\"$HOME/.cargo/bin:\$PATH\"\n" >> "$rc"
  fi
  ok "xsql installed successfully"
  printf "%b\n" "To use xsql in your current shell, run:\n  ${BOLD}${GREEN}export PATH=\"$HOME/.cargo/bin:\$PATH\"${RESET}\nOr source your rc: ${BOLD}${GREEN}source $rc${RESET}\n"
else
  ok "xsql installed successfully (already on PATH)"
  printf "%b\n" "Run: ${BOLD}${GREEN}xsql --help${RESET}\n"
fi

exit 0
