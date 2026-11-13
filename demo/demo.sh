#!/usr/bin/env bash
# sticks demo script — the "new user" flow from the README.
#
# Record it with asciinema:
#   asciinema rec -c ./demo/demo.sh demo.cast
#
# Or render to an SVG/GIF locally:
#   agg demo.cast demo.gif
#
# DEMO_PACE controls the pause between beats in seconds (default 0.6);
# set DEMO_PACE=0 for a fast run.
set -euo pipefail

STICKS="${STICKS:-sticks}"
PACE="${DEMO_PACE:-0.6}"
beat() { [ "$PACE" != "0" ] && sleep "$PACE" || true; }

cd "$(mktemp -d)"

echo '=== 1. Create a C++ project ==='
beat
$STICKS cpp demo --build cmake
cd demo
beat

echo
echo '=== 2. Build, run, test, clean ==='
beat
$STICKS build
beat
$STICKS run
beat
$STICKS test || true   # no tests configured yet — error is expected and clear
beat
$STICKS clean
beat

echo
echo '=== 3. Add a dependency (dry-run first) ==='
beat
$STICKS add --dry-run fmt@10.2.1
beat
$STICKS add fmt@10.2.1
beat
$STICKS remove fmt@10.2.1
beat

echo
echo '=== 4. Eject: back to a plain project ==='
beat
$STICKS eject
ls -a
