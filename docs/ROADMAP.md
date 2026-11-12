# Roadmap

Status as of the community-tooling push (post-0.4.1).

## Done

- Cross-platform binaries (Linux/macOS/Windows × x86_64/aarch64, armv7 Linux)
  and CI on all three OSes.
- `sticks build|run|test|clean` driving Make or CMake (Ninja when available),
  plus generated `CMakePresets.json`.
- Manifest-based dependency management for Conan and vcpkg, including
  `name@version` constraints; `--system` for OS packages.
- Safe edits: `--dry-run`, `--force`, automatic `.sticks.bak` backups,
  idempotent writes, `sticks eject`.
- Checksum-verified `sticks update`.
- Shell completions, man page, packaging templates.

## Next

- Real-install CI coverage for more Conan/vcpkg ports and triplets
  (Windows/macOS package-manager jobs).
- `sticks doctor`: probe toolchain, cmake, ninja, conan, vcpkg and report
  what a project needs.
- Signature verification for releases (minisign/cosign) on top of SHA256.
- `sticks add` version resolution hints (query vcpkg/Conan registries when
  online, degrade gracefully offline).
- Homebrew/winget/Nix packaging accepted upstream.

## Maybe

- `sticks migrate` helpers for importing existing autotools/Meson projects.
- Plugin hooks for custom generators.
