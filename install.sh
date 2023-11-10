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
local_version=$(sticks -v 2>/dev/null || true)
echo $local_version
if [[ "$local_version" =~ ^sticks\ [0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    local_version=$(echo "$local_version" | cut -d ' ' -f 2)
    # Fetch the version from Cargo.toml in the repository
    cargo_toml_version=$(curl -s https://raw.githubusercontent.com/mAmineChniti/sticks/master/Cargo.toml | grep "version" | cut -d '"' -f 2)
    if version_gt "$local_version" "$cargo_toml_version"; then
        echo "Latest version of sticks is already installed."
        exit 0
    fi
fi

# Display progress bar for Rust installation check
echo "Checking if Rust is installed..."
progress_bar 3

# Check if Rust is installed
if ! command -v rustc &>/dev/null; then
    echo "Rust is not installed. Please install Rust before running this script."
    exit 1
fi


# Get the host value from the output of `rustc -vV`
rustc_output=$(rustc -vV)
host=$(echo "$rustc_output" | awk -F'\n' '/host:/{print $1}' | cut -d' ' -f 2)

# Check if build-essential is already installed
if ! dpkg -l | grep -q "build-essential"; then
    # Install build-essential if not already installed (for Debian/Ubuntu)
    echo "Installing build-essential..."
    sudo apt install build-essential -y > /dev/null 2>&1 || { echo "Error: Unable to install build-essential."; exit 1; }
    progress_bar 10
fi

# Display progress bar for temporary directory creation
echo "Creating a temporary directory..."
progress_bar 3

# Create a temporary directory and change into it
temp_dir=$(mktemp -d)
cd "$temp_dir"

# Display progress bar for repository cloning
echo "Cloning the Git repository..."
progress_bar 5

# Clone the Git repository
git clone https://github.com/mAmineChniti/sticks > /dev/null 2>&1 || { echo "Error: Unable to clone the Git repository."; exit 1; }

# Change into the cloned directory
cd sticks

# Build the project with Cargo in release mode
echo "Building the project with Cargo..."
cargo build --release > /dev/null 2>&1 || { echo "Error: Cargo build failed."; exit 1; }
progress_bar 10

# Install cargo-deb if not already installed
if ! command -v cargo-deb &>/dev/null; then
    echo "Installing cargo-deb..."
    cargo install cargo-deb > /dev/null 2>&1 || { echo "Error: Unable to install cargo-deb."; exit 1; }
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
echo "Building a package for $os_name/$host using cargo-deb..."
cargo deb --target "$host" > /dev/null 2>&1 || { echo "Error: Cargo deb failed."; exit 1; }
progress_bar 10

# Install the generated package using the appropriate package manager
deb_dir="./target/${host}/${os_name}"
debian_dir="./target/${host}/debian"

if [ -f "$deb_dir"/*.deb ] || [ -f "$debian_dir"/*.deb ]; then
    echo "Attempting to install the generated package..."

    case $os_name in
        debian | raspbian | ubuntu)
            if [ -f "$deb_dir"/*.deb ]; then
                sudo apt install "$deb_dir"/*.deb -y > /dev/null 2>&1
            elif [ -f "$debian_dir"/*.deb ]; then
                sudo apt install "$debian_dir"/*.deb -y > /dev/null 2>&1
            else
                echo "No .deb package found for $os_name."
            fi
            ;;
        *)
            echo "Unsupported OS: $os_name. Please install the package manually."
            ;;
    esac
fi

# Clean up by removing the temporary directory
cd
rm -rf "$temp_dir"

# Optionally, you can print a message to confirm the installation
echo "$(sticks -v) is now installed."