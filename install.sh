#!/bin/bash

# Create a temporary directory and change into it
temp_dir=$(mktemp -d)
cd "$temp_dir"

# Clone the Git repository
git clone https://github.com/mAmineChniti/sticks.git

# Change into the cloned directory
cd sticks

# Build the project with Cargo in release mode
cargo build --release

# Install cargo-deb if not already installed
if ! command -v cargo-deb &>/dev/null; then
    cargo install cargo-deb
fi

# Build a Debian package using cargo-deb
cargo deb

# Install the generated Debian package using apt
sudo apt install ./target/debian/sticks*.deb

# Clean up by removing the temporary directory
cd
rm -rf "$temp_dir"

# Optionally, you can print a message to confirm the installation
echo "sticks is now installed."