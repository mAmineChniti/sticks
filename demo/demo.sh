#!/usr/bin/env bash
# sticks demo script — the "new user" flow from the README.
#
# Record it with asciinema:
#   asciinema rec -c ./demo/demo.sh demo.cast
#
# Or render to an SVG/GIF locally:
#   agg demo.cast demo.gif
set -euo pipefail

# Point STICKS at a built binary when running from a checkout:
#   STICKS=../target/debug/sticks ./demo/demo.sh
STICKS="${STICKS:-sticks}"

cd "$(mktemp -d)"

echo '=== 1. Create a C++ project ==='
$STICKS cpp demo --build cmake
cd demo

echo
echo '=== 2. Build, run, test, clean ==='
$STICKS build
$STICKS run
$STICKS test || true   # no tests configured yet — error is expected and clear
$STICKS clean

echo
echo '=== 3. Add a dependency (dry-run first) ==='
$STICKS add --dry-run fmt@10.2.1
$STICKS add fmt@10.2.1
$STICKS remove fmt@10.2.1

echo
echo '=== 4. Eject: back to a plain project ==='
$STICKS eject
ls -a
