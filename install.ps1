;# Exit immediately if a command exits with a non-zero status
$ErrorActionPreference = "Stop"

# Function to display a progress bar
function Show-ProgressBar {
    param (
        [int]$Duration
    )
    $steps = 100
    $increment = $Duration / $steps
    for ($i = 0; $i -le $steps; $i++) {
        Write-Host -NoNewline "`r[$i/$steps] "
        Write-Host -NoNewline ('=' * $i) + (' ' * ($steps - $i)) + ']'
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

# Fetch the current version installed from the command output
$localVersionOutput = (sticks -v 2>&1)
if ($localVersionOutput -match "^sticks\s\d+\.\d+\.\d+$") {
    $localVersion = $matches[0].Split(' ')[1]
    # Fetch the version from Cargo.toml in the repository
    $cargoTomlVersion = (Invoke-RestMethod -Uri "https://raw.githubusercontent.com/mAmineChniti/sticks/master/Cargo.toml" | Select-String "version" | ForEach-Object { $_.Line }).Split('"')[1]
    
    if (-not (Compare-Version $cargoTomlVersion $localVersion)) {
        Write-Host "Latest version of sticks is already installed."
        exit 0
    }
    else {
        Write-Host "Newer version of sticks has been found."
    }
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
$host = ($rustcOutput -split "`n" | Where-Object { $_ -like "*host:*" }).Split(' ')[1]

# Display progress bar for creating a temporary directory
Write-Host "Creating a temporary directory..."
Show-ProgressBar -Duration 3000

$tempDir = New-TemporaryFile
Set-Location $tempDir

# Clone the Git repository
Write-Host "Cloning the Git repository..."
Show-ProgressBar -Duration 5000
git clone https://github.com/mAmineChniti/sticks | Out-Null
Set-Location sticks

# Build the project with Cargo
Write-Host "Building the project with Cargo..."
cargo build --release | Out-Null
Show-ProgressBar -Duration 10000

# Install cargo-deb if not already installed
if (-not (Get-Command cargo-deb -ErrorAction SilentlyContinue)) {
    Write-Host "Installing cargo-deb..."
    cargo install cargo-deb | Out-Null
    Show-ProgressBar -Duration 10000
}

# Clean up by removing the temporary directory
Set-Location $env:USERPROFILE
Remove-Item -Recurse -Force $tempDir

Write-Host "$(sticks -v) is now installed."
