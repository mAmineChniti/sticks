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
# set DEMO_PACE=0 for a fast run. Colors are emitted on TTYs (including
# the asciinema pty) so the recording looks right in the GIF.
set -euo pipefail

STICKS="${STICKS:-sticks}"
PACE="${DEMO_PACE:-0.6}"
if [ -t 1 ]; then
	BOLD=$'\033[1m'
	CYAN=$'\033[1;36m'
	YELLOW=$'\033[1;33m'
	DIM=$'\033[2m'
	RESET=$'\033[0m'
else
	BOLD=''; CYAN=''; YELLOW=''; DIM=''; RESET=''
fi

beat() { [ "$PACE" != "0" ] && sleep "$PACE" || true; }

# Echo the command in yellow, then run it — viewers see what's happening.
# $STICKS is always argv[1]; display it as `sticks` regardless of path.
run() {
	printf '%s$ sticks %s%s\n' "$YELLOW" "${*:2}" "$RESET"
	"$@"
}

header() {
	printf '\n%s=== %s ===%s\n' "$CYAN" "$1" "$RESET"
	beat
}

cd "$(mktemp -d)"

header '1. Create a C++ project'
run $STICKS cpp demo --build cmake
cd demo
beat

header '2. Build, run, test, clean'
run $STICKS build
beat
run $STICKS run
beat
run $STICKS test || true   # no tests configured yet — error is expected and clear
beat
run $STICKS clean
beat

header '3. Add a dependency (dry-run first)'
run $STICKS add --dry-run fmt@10.2.1
beat
run $STICKS add fmt@10.2.1
beat
run $STICKS remove fmt@10.2.1
beat

header '4. Eject: back to a plain project'
run $STICKS eject
ls -a
