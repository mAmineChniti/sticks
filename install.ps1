# Exit immediately if a command exits with a non-zero status
$ErrorActionPreference = "Stop"

# Function to display a progress bar
function Show-ProgressBar {
    param (
        [int]$Duration
    )
    $steps = 100
    $increment = $Duration / $steps
    for ($i = 0; $i -le $steps; $i++) {
        Write-Host -NoNewline "`rProgress: [$i/$steps] " + ('=' * $i) + (' ' * ($steps - $i)) + "]"
        Start-Sleep -Milliseconds $increment
    }
    Write-Host "`n"
}

# Function to compare version numbers
function Compare-Version {
    param (
        [string]$ver1,
        [string]$ver2
    )

    $ver1Parts = $ver1.Split('.')
    $ver2Parts = $ver2.Split('.')

    for ($i = 0; $i -lt $ver1Parts.Length; $i++) {
        if ($ver2Parts.Length -le $i) {
            return $true
        }
        if ([int]$ver1Parts[$i] -gt [int]$ver2Parts[$i]) {
            return $true
        }
        elseif ([int]$ver1Parts[$i] -lt [int]$ver2Parts[$i]) {
            return $false
        }
    }
    return $false
}

# Function to fetch the latest version from Cargo.toml
function Get-LatestVersion {
    try {
        $cargoTomlContent = Invoke-RestMethod -Uri "https://raw.githubusercontent.com/mAmineChniti/sticks/master/Cargo.toml"
        $versionLine = $cargoTomlContent | Select-String 'version\s*=\s*".+"'
        if ($versionLine) {
            return ($versionLine -split '"' )[1]
        }
        else {
            throw "Version not found in Cargo.toml."
        }
    }
    catch {
        Write-Error "Failed to fetch latest version from Cargo.toml: $_"
        exit 1
    }
}

# Function to get the installed version of sticks
function Get-InstalledVersion {
    try {
        $localVersionOutput = sticks -v
        if ($localVersionOutput -match "^sticks\s(\d+\.\d+\.\d+)$") {
            return $matches[1]
        }
        else {
            Write-Warning "Unable to parse sticks version. Assuming it's not installed."
            return $null
        }
    }
    catch {
        return $null
    }
}

# Main Script Execution Starts Here

# Get the installed version of sticks
$installedVersion = Get-InstalledVersion

# Fetch the latest version from the repository
$latestVersion = Get-LatestVersion

if ($installedVersion) {
    if (-not (Compare-Version $latestVersion $installedVersion)) {
        Write-Host "You already have the latest version of sticks ($installedVersion) installed."
        exit 0
    }
    else {
        Write-Host "A newer version of sticks ($latestVersion) is available. Installed version: $installedVersion."
    }
}
else {
    Write-Host "sticks is not installed. Proceeding with installation."
}

# Display progress bar for Rust installation check
Write-Host "Checking if Rust is installed..."
Show-ProgressBar -Duration 3000

# Check if Rust is installed
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Host "Rust is not installed. Please install Rust from https://www.rust-lang.org/ before running this script."
    exit 1
}

# Get the host value from rustc output
$rustcOutput = rustc -vV
$rustHostLine = $rustcOutput | Where-Object { $_ -like "host:*" }
$rustHost = ($rustHostLine -split ' ')[1]

# Create a temporary directory
$tempDir = Join-Path -Path ([System.IO.Path]::GetTempPath()) -ChildPath ("sticks_install_" + (Get-Random))
New-Item -ItemType Directory -Path $tempDir | Out-Null
Set-Location $tempDir

# Clone the Git repository
Write-Host "Cloning the Git repository..."
Show-ProgressBar -Duration 5000
git clone https://github.com/mAmineChniti/sticks . | Out-Null

# Build the project with Cargo
Write-Host "Building the project with Cargo..."
Show-ProgressBar -Duration 10000
cargo build --release | Out-Null

# Install the project using Cargo
Write-Host "Installing sticks using Cargo..."
Show-ProgressBar -Duration 5000
cargo install --path . --force | Out-Null

# Clean up by removing the temporary directory
Set-Location $env:USERPROFILE
Remove-Item -Recurse -Force $tempDir

Write-Host "sticks version $latestVersion has been installed successfully."
