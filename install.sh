#!/bin/bash

set -e

# Function to display a progress bar
function progress_bar {
    local duration=$1
    local steps=100
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

# Function to compare version numbers
function version_gt {
    local ver1=$1
    local ver2=$2

    IFS=. read -ra ver1 <<< "$ver1"
    IFS=. read -ra ver2 <<< "$ver2"

    for ((i = 0; i < ${#ver1[@]}; i++)); do
        if [ -z "${ver2[i]}" ]; then
            # If ver2 has fewer components, consider it smaller
            return 0
        elif [ "${ver1[i]}" -gt "${ver2[i]}" ]; then
            return 1
        elif [ "${ver1[i]}" -lt "${ver2[i]}" ]; then
            return 0
        fi
    done

    return 0
}

# Get the version from the output of sticks -v
local_version=$(sticks -v | cut -d ' ' -f 2)

# Fetch the version from Cargo.toml in the repository
cargo_toml_version=$(curl -s https://raw.githubusercontent.com/mAmineChniti/sticks/master/Cargo.toml | grep "version" | cut -d '"' -f 2)

# Check if the local version is the latest
if version_gt "$local_version" "$cargo_toml_version"; then
    echo "Latest version of sticks is already installed."
    exit 0
fi

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
git clone https://github.com/mAmineChniti/sticks > /dev/null 2>&1 || { echo "Error: Unable to clone the Git repository."; exit 1; }

# Display progress bar for repository cloning
echo "Cloning the Git repository..."
progress_bar 5

# Change into the cloned directory
cd sticks

# Check if build-essential is already installed
if ! dpkg -l | grep -q "build-essential"; then
    # Install build-essential if not already installed (for Debian/Ubuntu)
    echo "Installing build-essential..."
    sudo apt install build-essential -y || { echo "Error: Unable to install build-essential."; exit 1; }
    progress_bar 10
fi

# Build the project with Cargo in release mode
echo "Building the project with Cargo..."
cargo build --release || { echo "Error: Cargo build failed."; exit 1; }
progress_bar 10

# Install cargo-deb if not already installed
if ! command -v cargo-deb &>/dev/null; then
    echo "Installing cargo-deb..."
    cargo install cargo-deb || { echo "Error: Unable to install cargo-deb."; exit 1; }
    progress_bar 10
fi

# Build a package using cargo-deb for the detected OS
echo "Building a package for $host using cargo-deb..."
cargo deb --target "$host" || { echo "Error: Cargo deb failed."; exit 1; }
progress_bar 10

# Check if the package is actually generated
if [ -f "./target/${host}/${host}"/*.deb ] || [ -f "./target/${host}"/debian/*.deb ]; then
    # Install the generated package using the appropriate package manager
    echo "Installing the generated package..."
    case $host in
        x86_64-unknown-linux-gnu)
            os_name="debian"
            ;;
        *)
            echo "Unsupported OS: $host. Please install the package manually."
            ;;
    esac

    if [ -f "./target/${host}/${host}"/*.deb ]; then
        sudo apt install "./target/${host}/${host}"/*.deb -y || { echo "Error: Unable to install the generated package."; exit 1; }
    elif [ -f "./target/${host}"/debian/*.deb ]; then
        sudo apt install "./target/${host}"/debian/*.deb -y || { echo "Error: Unable to install the generated package."; exit 1; }
    else
        echo "No .deb package found for $host."
    fi
else
    echo "Package not generated. Please check the build process."
fi

# Clean up by removing the temporary directory
cd
rm -rf "$temp_dir"

# Optionally, you can print a message to confirm the installation
echo "sticks is now installed."
