#!/bin/bash
# Helper script to create a dmenu symlink for fsel

# Check if fsel is installed
if ! command -v fsel &> /dev/null; then
    echo "Error: fsel is not installed or not in PATH"
    exit 1
fi

# Get the fsel binary location
FSEL_PATH=$(which fsel)

# Get the directory where fsel is installed
INSTALL_DIR=$(dirname "$FSEL_PATH")

# Create symlink
DMENU_PATH="$INSTALL_DIR/dmenu"

if [ -e "$DMENU_PATH" ]; then
    echo "Warning: $DMENU_PATH already exists"
    read -p "Overwrite? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
    rm "$DMENU_PATH"
fi

ln -s "$FSEL_PATH" "$DMENU_PATH"

if [ $? -eq 0 ]; then
    echo "Successfully created dmenu symlink at $DMENU_PATH"
    echo "You can now use 'dmenu' as a drop-in replacement"
else
    echo "Error: Failed to create symlink"
    exit 1
fi
