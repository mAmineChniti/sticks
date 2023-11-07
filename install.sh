#!/bin/bash

# Check if Rust is installed
if ! command -v rustc &>/dev/null; then
    echo "Rust is not installed. Please install Rust before running this script."
    exit 1
fi

# Get the host value from the output of `rustc -vV`
rustc_output=$(rustc -vV)
host=$(echo "$rustc_output" | awk -F'\n' '/host:/{print $1}' | cut -d' ' -f 2)

# Create a temporary directory and change into it
temp_dir=$(mktemp -d)
cd "$temp_dir"

# Clone the Git repository
git clone https://github.com/mAmineChniti/sticks.git

# Change into the cloned directory
cd sticks

# Check if build-essential is already installed
if ! dpkg -l | grep -q "build-essential"; then
    # Install build-essential if not already installed (for Debian/Ubuntu)
    sudo apt update
    sudo apt install build-essential
fi

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
cargo deb --target "$host"

# Check if the package is actually generated
if [ -f "./target/${host}/${os_name}"/*.deb ] || [ -f "./target/${host}/${os_name}"/*.rpm ]; then
    # Install the generated package using the appropriate package manager
    case $os_name in
        debian | ubuntu | raspbian)
            sudo apt install "./target/${host}/${os_name}"/*.deb
            ;;
        fedora)
            sudo dnf install "./target/${host}/${os_name}"/*.rpm
            ;;
        centos | rhel)
            sudo yum install "./target/${host}/${os_name}"/*.rpm
            ;;
        *)
            echo "Unsupported OS: $os_name. Please install the package manually."
            ;;
    esac
else
    echo "Package not generated. Please check the build process."
fi

# Clean up by removing the temporary directory
cd
rm -rf "$temp_dir"

# Optionally, you can print a message to confirm the installation
echo "sticks is now installed."
