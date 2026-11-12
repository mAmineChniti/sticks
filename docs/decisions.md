# Decisions log

Ambiguities encountered while taking sticks to a community tool, and the
simplest option chosen.

## D1 — Makefile portability vs recursion

`$(wildcard)` cannot recurse into `src/` subdirectories; `find|sort` does.
Chosen: keep `find`/`sort` (present on Unix, Git Bash and MSYS2 — the
environments where Makefiles realistically run on Windows) and make the
compiler variables overridable (`CC ?=`/`CXX ?=`) plus per-extension
pattern rules. Windows users are directed to the CMake path.

## D2 — Toolchain override surface

`--cc`/`--cxx` on `c`/`cpp`/`init`. For Makefiles the detected/override
compiler is written as `CC ?=`/`CXX ?=`; for CMake, `set(CMAKE_*_COMPILER)`
is only emitted for explicit overrides (CMake auto-detects otherwise).

## D3 — `name@version` mapping

`sticks add fmt@10.2.1` maps to `"version>=10.2.1"` in vcpkg.json and
`fmt/10.2.1` in Conan files. Conan specs already containing `/` pass
through unchanged.

## D4 — Package-manager baselines

vcpkg `builtin-baseline` / Conan lockfiles require values sticks cannot
invent offline. Chosen: write correct manifest entries and document that
pinning a baseline is the package manager's own workflow (`vcpkg
install` records the baseline; `conan lock create` produces lockfiles).
Revisit if registry queries are added.

## D5 — `--dry-run`/`--force` scope

Implemented in `file_handler` (the single funnel for file writes) plus
`features.rs` renames/removals. Commands that shell out (`build`, `run`,
`test`, `clean`) don't modify project files and ignore the flags.

## D6 — Backups

Every modifying write to an existing file first copies it to
`<name>.sticks.bak` (overwriting any previous backup). Deletions become
renames to the same backup path. This trades some disk for reversibility;
a `--no-backup` escape hatch can be added if it annoys users.

## D7 — `sticks eject` scope

Removes `.vscode/`, `.clang-format`, `.editorconfig`, `.gitattributes`,
`CMakePresets.json` (to `.sticks.bak`). Build files and package-manager
manifests are already plain standards — kept untouched. Conan users who
relied on the generated preset can regenerate it with `conan install`.

## D8 — Update verification

SHA256 only (hashes published in `SHA256SUMS` by the release workflow).
Signature verification (minisign/cosign) is on the roadmap; HTTPS +
GitHub-release integrity is the current trust anchor.

## D9 — `sticks test` semantics

`sticks test` with no subcommand now runs tests (ctest / `make test`).
`sticks test add|generate` still manage test frameworks. This is a
behavior change from erroring on bare `sticks test`, matching user
expectation from other ecosystems.
