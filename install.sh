#!/usr/bin/env bash
set -e

# gyr installer
REPO="Mjoyufull/gyr"
INSTALL_DIR="$HOME/.local/bin"
BIN_NAME="gyr"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Check dependencies
check_deps() {
    command -v git >/dev/null 2>&1 || error "git is required"
    command -v cargo >/dev/null 2>&1 || error "cargo is required (install from https://rustup.rs/)"
}

check_deps

# Create install directory
mkdir -p "$INSTALL_DIR"

# Check if already installed
if [ -f "$INSTALL_DIR/$BIN_NAME" ]; then
    info "Updating existing installation"
    UPDATE=1
else
    info "Installing gyr"
    UPDATE=0
fi

# Clone or update repository
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

cd "$TEMP_DIR"
info "Downloading source from github.com/$REPO"
git clone --depth 1 "https://github.com/$REPO.git" gyr-src
cd gyr-src

# Build release binary
info "Building gyr (this may take a while)"
cargo build --release --quiet

# Install binary
info "Installing to $INSTALL_DIR"
cp target/release/gyr "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/gyr"

# Check if in PATH
if ! echo "$PATH" | grep -q "$HOME/.local/bin"; then
    warn "$HOME/.local/bin is not in your PATH"
    warn "Add this to your shell config:"
    warn "  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# Verify installation
if "$INSTALL_DIR/gyr" --version >/dev/null 2>&1; then
    VERSION=$("$INSTALL_DIR/gyr" --version)
    if [ $UPDATE -eq 1 ]; then
        info "Updated gyr to $VERSION"
    else
        info "Installed gyr $VERSION"
    fi
    info "Usage: gyr --help"
else
    error "Installation failed"
fi