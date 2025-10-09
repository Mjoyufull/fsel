#!/usr/bin/env bash
set -e

# fsel installer
REPO="Mjoyufull/fsel"
INSTALL_DIR="$HOME/.local/bin"
BIN_NAME="fsel"

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
    info "Installing fsel"
    UPDATE=0
fi

# Clone or update repository
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

cd "$TEMP_DIR"
info "Downloading source from github.com/$REPO"
git clone --depth 1 "https://github.com/$REPO.git" fsel-src
cd fsel-src

# Build release binary
info "Building fsel (this may take a while)"
cargo build --release --quiet

# Install binary
info "Installing to $INSTALL_DIR"
cp target/release/fsel "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/fsel"

# Install man page
if [ -f "fsel.1" ]; then
    info "Installing man page"
    
    # Try system-wide first (requires sudo)
    if [ -w "/usr/local/share/man/man1" ] 2>/dev/null; then
        cp fsel.1 "/usr/local/share/man/man1/"
        mandb -q 2>/dev/null || true
    elif command -v sudo >/dev/null 2>&1; then
        # Ask for sudo to install system-wide
        if sudo -n true 2>/dev/null || sudo -v; then
            sudo mkdir -p /usr/local/share/man/man1
            sudo cp fsel.1 /usr/local/share/man/man1/
            sudo mandb -q 2>/dev/null || true
        else
            # Fall back to user install
            MAN_DIR="$HOME/.local/share/man/man1"
            mkdir -p "$MAN_DIR"
            cp fsel.1 "$MAN_DIR/"
            
            # Check if MANPATH is set
            if ! echo "$MANPATH" | grep -q "$HOME/.local/share/man"; then
                warn "Man page installed to $MAN_DIR"
                warn "Add this to your shell config to use 'man fsel':"
                warn "  export MANPATH=\"\$HOME/.local/share/man:\$MANPATH\""
            fi
        fi
    else
        # No sudo, install to user directory
        MAN_DIR="$HOME/.local/share/man/man1"
        mkdir -p "$MAN_DIR"
        cp fsel.1 "$MAN_DIR/"
        
        # Check if MANPATH is set
        if ! echo "$MANPATH" | grep -q "$HOME/.local/share/man"; then
            warn "Man page installed to $MAN_DIR"
            warn "Add this to your shell config to use 'man fsel':"
            warn "  export MANPATH=\"\$HOME/.local/share/man:\$MANPATH\""
        fi
    fi
fi

# Check if in PATH
if ! echo "$PATH" | grep -q "$HOME/.local/bin"; then
    warn "$HOME/.local/bin is not in your PATH"
    warn "Add this to your shell config:"
    warn "  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# Verify installation
if "$INSTALL_DIR/fsel" --version >/dev/null 2>&1; then
    VERSION=$("$INSTALL_DIR/fsel" --version)
    if [ $UPDATE -eq 1 ]; then
        info "Updated fsel to $VERSION"
    else
        info "Installed fsel $VERSION"
    fi
    info "Usage: fsel --help"
else
    error "Installation failed"
fi