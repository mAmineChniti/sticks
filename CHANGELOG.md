# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] - 2026-11-12

### Added

- `sticks build [--release]`, `sticks run [-- args]`, `sticks test`,
  `sticks clean` — drive the project's real build system (Make or CMake).
- `CMakePresets.json` generation for CMake projects (`debug`/`release`
  configure, build and test presets; Ninja generator when available).
- `sticks eject` — moves sticks-specific files to `.sticks.bak` backups,
  leaving a plain working project.
- Global `--dry-run` (unified diff preview) and `--force` flags.
- Automatic `.sticks.bak` backups before overwriting or removing files.
- `sticks add --system` / `sticks remove --system` for OS packages.
- Version constraints: `sticks add fmt@10.2.1` (vcpkg `version>=`,
  Conan `name/version`).
- `--cc`/`--cxx` compiler overrides on `c`, `cpp` and `init`.
- Toolchain auto-detection (gcc, clang, MSVC `cl`) for Makefile projects.
- `sticks completions <shell>` and `sticks man`.
- Checksum-verified `sticks update` (SHA256SUMS published per release).
- CI matrix: tests run on Ubuntu, macOS and Windows.
- Dedicated CI job performing real vcpkg and Conan installs.

### Changed

- Generated CMake no longer forces GCC-style warning flags on MSVC.
- `convert` keeps the old build file as `<name>.sticks.bak` instead of
  deleting it.
- `sticks update` now performs verified in-place updates for binaries
  installed outside package managers (previously refused outright).
- Removing a package manager moves the manifest to `.sticks.bak` instead
  of deleting it.

### Fixed

- Makefile generation no longer depends on GNU-only shell idioms beyond
  `find`/`sort`; compiler variables are overridable (`CC ?=`/`CXX ?=`).
