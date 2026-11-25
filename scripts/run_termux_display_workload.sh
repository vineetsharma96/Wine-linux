#!/bin/sh
# Run in the user's Termux shell while the X server is already running.
set -eu
cd /sdcard/Download
deadline=$(($(date +%s) + 60))
while [ ! -f androidpc-display-go ]; do
    [ "$(date +%s)" -lt "$deadline" ] || exit 1
    sleep 0.2
done
trap 'touch androidpc-display-done' EXIT
export DISPLAY="${DISPLAY:-:0}"
xrandr --current
DISPLAY_BENCH_FRAMES=$(cat androidpc-display-go)
export DISPLAY_BENCH_FRAMES
python3 androidpc-display-workload.py
