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

# Detect the OS name
if [ -f /etc/os-release ]; then
    . /etc/os-release
    os_name=$ID
else
    os_name=$(uname -s | tr '[:upper:]' '[:lower:]')
fi

# Build a package using cargo-deb for the detected OS
cargo deb --target "$os_name"

# Install the generated package using the appropriate package manager
case $os_name in
    debian | ubuntu | raspbian)
        sudo apt install "./target/${os_name}/*.deb"
        ;;
    fedora)
        sudo dnf install "./target/${os_name}/*.rpm"
        ;;
    centos | rhel)
        sudo yum install "./target/${os_name}/*.rpm"
        ;;
    *)
        echo "Unsupported OS: $os_name. Please install the package manually."
        ;;
esac

# Clean up by removing the temporary directory
cd
rm -rf "$temp_dir"

# Optionally, you can print a message to confirm the installation
echo "sticks is now installed."
