#!/bin/bash

# Function to display a progress bar
function progress_bar {
    local duration=$1
    local steps=50
    local increment=$((duration / steps))
    for ((i = 0; i <= steps; i++)); do
        echo -ne "[$i/$steps] "
        for ((j = 0; j < i; j++)); do
            echo -n "="
        done
        for ((j = i; j < steps; j++)); do
            echo -n " "
        done
        echo -ne "]\r"
        sleep $increment
    done
    echo -ne "\n"
}

# Check if Rust is installed
if ! command -v rustc &>/dev/null; then
    echo "Rust is not installed. Please install Rust before running this script."
    exit 1
fi

# Display progress bar for Rust installation check
echo "Checking if Rust is installed..."
progress_bar 3

# Get the host value from the output of `rustc -vV`
rustc_output=$(rustc -vV)
host=$(echo "$rustc_output" | awk -F'\n' '/host:/{print $1}' | cut -d' ' -f 2)

# Create a temporary directory and change into it
temp_dir=$(mktemp -d)
cd "$temp_dir"

# Display progress bar for temporary directory creation
echo "Creating a temporary directory..."
progress_bar 3

# Clone the Git repository
git clone https://github.com/mAmineChniti/sticks > /dev/null 2>&1

# Display progress bar for repository cloning
echo "Cloning the Git repository..."
progress_bar 5

# Change into the cloned directory
cd sticks

# Check if build-essential is already installed
if ! dpkg -l | grep -q "build-essential"; then
    # Install build-essential if not already installed (for Debian/Ubuntu)
    echo "Installing build-essential..."
    sudo apt update > /dev/null 2>&1
    sudo apt install build-essential -y > /dev/null 2>&1
    progress_bar 10
fi

# Build the project with Cargo in release mode
echo "Building the project with Cargo..."
cargo build --release > /dev/null 2>&1
progress_bar 10

# Install cargo-deb if not already installed
if ! command -v cargo-deb &>/dev/null; then
    echo "Installing cargo-deb..."
    cargo install cargo-deb > /dev/null 2>&1
    progress_bar 10
fi

# Detect the OS name
if [ -f /etc/os-release ]; then
    . /etc/os-release
    os_name=$ID
else
    os_name=$(uname -s | tr '[:upper:]' '[:lower:]')
fi

# Build a package using cargo-deb for the detected OS
echo "Building a package for $os_name using cargo-deb..."
cargo deb --target "$host" > /dev/null 2>&1
progress_bar 10

# Check if the package is actually generated
if [ -f "./target/${host}/${os_name}"/*.deb ] || [ -f "./target/${host}"/debian/*.deb ]; then
    # Install the generated package using the appropriate package manager
    echo "Installing the generated package..."
    case $os_name in
        debian | raspbian)
            if [ -f "./target/${host}/${os_name}"/*.deb ]; then
                sudo apt install "./target/${host}/${os_name}"/*.deb -y > /dev/null 2>&1
            elif [ -f "./target/${host}"/debian/*.deb ]; then
                sudo apt install "./target/${host}"/debian/*.deb -y > /dev/null 2>&1
            else
                echo "No .deb package found for $os_name."
            fi
            ;;
        ubuntu)
            if [ -f "./target/${host}/${os_name}"/*.deb ]; then
                sudo apt install "./target/${host}/${os_name}"/*.deb -y > /dev/null 2>&1
            elif [ -f "./target/${host}"/debian/*.deb ]; then
                sudo apt install "./target/${host}"/debian/*.deb -y > /dev/null 2>&1
            else
                echo "No .deb package found for $os_name."
            fi
            ;;
        fedora)
            sudo dnf install "./target/${host}/${os_name}"/*.rpm -y > /dev/null 2>&1
            ;;
        centos | rhel)
            sudo yum install "./target/${host}/${os_name}"/*.rpm -y > /dev/null 2>&1
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
