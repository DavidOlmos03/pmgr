#!/bin/bash

set -e

echo "Building pmgr..."
cargo build --release

echo "Installing pmgr to /usr/local/bin..."
sudo cp target/release/pmgr /usr/local/bin/

echo ""
echo "✓ Installation complete!"
echo ""
echo "Try running: pmgr --help"
echo ""
echo "Quick start:"
echo "  pmgr install          # Browse and install packages"
echo "  pmgr remove           # Browse and remove packages"
echo "  pmgr list -i          # Browse installed packages"
echo "  pmgr search firefox   # Search for packages"
