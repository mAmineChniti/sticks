<p align="center">
<a href="https://crates.io/crates/sticks" rel="noopener noreferrer">
<img src="sticks.png" alt="sticks Logo" height="150" width="150"/>
</a>
</p>
<h1 align="center">sticks</h1>

<p align="center">
  <em>A modern, lightweight CLI tool for managing C and C++ projects</em>
</p>

<p align="center">
  <a href="https://crates.io/crates/sticks"><img alt="Crates.io" src="https://img.shields.io/crates/v/sticks"/></a>
  <a href="https://github.com/mAmineChniti/sticks/actions/workflows/coverage.yml"><img alt="Code Coverage" src="https://img.shields.io/badge/coverage-100%25-brightgreen"/></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue"/></a>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#usage">Usage</a> •
  <a href="#updating">Updating</a> •
  <a href="#contributing">Contributing</a>
</p>

---

## Features

- 🎯 **Interactive Mode** - Just run `sticks` for guided project setup with arrow key navigation
- 🚀 **Quick Project Setup** - Create new C/C++ projects with a single command
- 📁 **Multiple Build Systems** - Support for both Makefile and CMake
- 🔨 **Smart Structure** - Auto-generates organized project structure with source files and build configs
- 📦 **Dependency Management** - Easily add/remove dependencies in your Makefile
- 🔧 **Multi-Source Support** - Add multiple source files with automatic build integration
- 📝 **Auto-Generated Config** - Creates .gitignore, .editorconfig, Clang-format config, VSCode settings
- 🔄 **Self-Updating** - Built-in update mechanism that downloads from GitHub releases
- 🎯 **Zero Runtime Dependencies** - Just needs GCC; no Rust/Cargo required after installation
- ✅ **Quality Assured** - Comprehensive test suite with 39 automated tests (100% coverage)
- 🔐 **CI/CD Pipeline** - Automated testing, building, and releases on every change

## Installation

Choose the installation method that works best for you:

### 📦 Package Managers (Recommended)

<details>
<summary><b>Arch Linux (AUR)</b></summary>

**Package Name:** `sticks-aur`

```bash
# Using an AUR helper (recommended)
yay -S sticks-aur
# or
paru -S sticks-aur

# Or manually clone from AUR
git clone https://aur.archlinux.org/sticks-aur.git
cd sticks-aur
makepkg -si
```

See [sticks-aur repository](https://github.com/mAmineChniti/sticks-aur) for packaging details.

</details>

<details>
<summary><b>Debian/Ubuntu</b></summary>

```bash
# Download the latest .deb package
wget https://github.com/mAmineChniti/sticks/releases/latest/download/sticks_0.3.3-1_amd64.deb
sudo dpkg -i sticks_*.deb
```

</details>

### 🚀 Pre-built Binaries

```bash
wget https://github.com/mAmineChniti/sticks/releases/latest/download/sticks-linux-x86_64
chmod +x sticks-linux-x86_64
sudo mv sticks-linux-x86_64 /usr/local/bin/sticks
```

### 🦀 From Cargo

```bash
cargo install sticks
```

Requires Rust toolchain from [rustup.rs](https://rustup.rs/).

### 🔨 Build from Source

```bash
# Clone the repository
git clone --recurse-submodules https://github.com/mAmineChniti/sticks.git
cd sticks

# Build release binary
cargo build --release

# Install (choose one)
sudo cp target/release/sticks /usr/local/bin/  # System-wide
# or
cp target/release/sticks ~/.local/bin/         # User only
```

## Quick Start

### Interactive Mode (Easiest!)

Just run `sticks` with no arguments for an interactive guided experience:

```bash
sticks
```

Follow the prompts to:
1. Enter your project name
2. Choose language (C or C++)
3. Select build system (Makefile or CMake)
4. Your project is created!

Use arrow keys to navigate, Enter to select.

### Command Line

```bash
# Create a new C++ project with Makefile (default)
sticks cpp my-project
cd my-project

# Or with CMake build system
sticks cpp my-project --build cmake
cd my-project

# Add a dependency
sticks add libcurl

# Add more source files
sticks src utils network

# Build and run
make
./my-project
```

## Usage

### Interactive Mode

Run `sticks` without any arguments to enter interactive mode:

```bash
sticks
```

This launches a guided setup where you can:
- Enter project name
- Select language (C or C++) using arrow keys
- Choose build system (Makefile or CMake) using arrow keys
- Confirm with Enter to create your project

### Command Shortcuts

Common commands have short aliases for faster typing:

```bash
sticks i              # sticks init
sticks s myfile       # sticks src myfile
sticks a libcurl      # sticks add libcurl
sticks r libcurl      # sticks remove libcurl
sticks u              # sticks update
```

### Creating Projects

**Create a new project in a subdirectory:**

```bash
sticks c my-c-project       # New C project with Makefile
sticks cpp my-cpp-project   # New C++ project with Makefile
```

**Create with CMake build system:**

```bash
sticks c my-project --build cmake       # C project with CMake
sticks cpp my-project --build cmake     # C++ project with CMake
```

**Initialize in current directory:**

```bash
sticks init c               # Initialize C project here
sticks init cpp --build cmake  # Initialize C++ project with CMake
```

### Managing Dependencies

**Add dependencies:**

```bash
sticks add libcurl              # Single dependency
sticks add openssl libpq zlib   # Multiple dependencies
```

Automatically updates your Makefile's `install-deps` target.

**Remove dependencies:**

```bash
sticks remove libcurl           # Remove single dependency
sticks remove openssl libpq     # Remove multiple dependencies
```

Cleans up the `install-deps` rule automatically when empty.

### Adding Source Files

```bash
sticks src utils               # Adds src/utils.cpp (or .c) and header
sticks src network database    # Add multiple source files
```

Sticks will:

- Create source files in `src/`
- Create corresponding headers
- Update build file (Makefile or CMakeLists.txt) automatically

### Generated Configuration Files

When you create a project, Sticks automatically generates:

- **Build System Files:** `Makefile` or `CMakeLists.txt` (your choice)
- **Git:** `.gitignore`, `.gitattributes` (pre-configured for C/C++)
- **Code Style:** `.editorconfig`, `.clang-format` (consistent formatting)
- **IDE:** VSCode `.vscode/settings.json`, `launch.json`, `tasks.json`
- **Documentation:** `README.md` (project-specific template)

This gives you a professional, production-ready project structure out of the box!

### Getting Help

```bash
sticks --help           # Show all commands
sticks <command> --help # Help for specific command
sticks --version        # Show version
```

## Updating

Sticks can update itself without requiring Rust/Cargo:

```bash
sticks update
```

This downloads the latest binary from GitHub releases and replaces your installation.

**Alternative update methods:**

```bash
# Arch Linux (using AUR package manager)
# Package name: sticks-aur
sudo pacman -Syu  # Update package database first
yay -Syu sticks-aur
# or
paru -Syu sticks-aur

# Debian/Ubuntu (download new .deb)
wget https://github.com/mAmineChniti/sticks/releases/latest/download/sticks_0.3.0-1_amd64.deb
sudo dpkg -i sticks_*.deb

# Cargo installation
cargo install sticks --force
```

## Uninstallation

```bash
# Cargo installation
cargo uninstall sticks

# Arch Linux (AUR package name: sticks-aur)
yay -R sticks-aur
# or
paru -R sticks-aur

# Debian/Ubuntu
sudo apt remove sticks

# Manual installation
sudo rm /usr/local/bin/sticks
# or
rm ~/.local/bin/sticks
```

## Project Structure

A typical sticks-managed project looks like:

```
my-project/
├── src/
│   ├── main.cpp        # Entry point
│   ├── utils.cpp       # Additional sources
│   └── network.cpp
├── include/
│   ├── utils.h         # Headers
│   └── network.h
├── build/              # Build artifacts (gitignored)
│   ├── debug/
│   └── release/
└── Makefile            # Auto-generated, customizable
```

## Technical Details

- **Language:** Rust 2021 edition
- **Dependencies:** clap 4, anyhow (build-time only)
- **Dev Dependencies:** serial_test (for isolated test execution)
- **Runtime Requirements:** GCC (for compiling your C/C++ projects)
- **Supported Architectures:** x86_64
- **Supported Platforms:** Linux (Arch, Debian, Ubuntu, others)
- **Test Coverage:** 18 comprehensive tests covering all core functionality
- **CI/CD:** Automated testing, building, and releases via GitHub Actions

## Contributing

We welcome contributions! Here's how to get involved:

1. **Report Issues:** Found a bug? [Open an issue](https://github.com/mAmineChniti/sticks/issues)
2. **Submit PRs:** Fork the repo and submit pull requests
3. **Improve Docs:** Help us make documentation better

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

## Roadmap

- [X] Implement dependency removal with `sticks remove`
- [X] Auto-cleanup of empty `install-deps` rule
- [X] Modularized codebase
- [X] Batch dependency operations
- [X] CI/CD pipeline with automated releases
- [X] Self-update mechanism without Cargo dependency
- [X] Multi-architecture support (x86_64)
- [X] Comprehensive test suite with automated testing
- [X] Quality gates in CI/CD (tests run before releases)
- [X] CMake support alongside Makefile
- [X] Auto-generated .gitignore, .editorconfig, .clang-format
- [X] VSCode integration (settings, launch config, tasks)
- [X] Auto-generated README templates
- [ ] Integration tests for end-to-end workflows
- [ ] Template system for custom project structures
- [ ] Package manager integration (conan, vcpkg)
- [ ] Plugin system for extending functionality
- [X] Code coverage reporting

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contact

**Maintainer:** mAmineChniti  
**Email:** [emin.chniti@esprit.tn](mailto:emin.chniti@esprit.tn)  
**Repository:** [github.com/mAmineChniti/sticks](https://github.com/mAmineChniti/sticks)  
**AUR Package:** [github.com/mAmineChniti/sticks-aur](https://github.com/mAmineChniti/sticks-aur)

---

<p align="center">Made with ❤️ for the C/C++ community</p>
