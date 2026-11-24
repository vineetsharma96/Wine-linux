#!/bin/sh
# Android PC Environment — XFCE 4 Desktop Session Launcher
# Executed within the rootless PRoot guest userspace

set -e

export DISPLAY="${DISPLAY:-:1}"
export LANG="C.UTF-8"
export LC_ALL="C.UTF-8"
export HOME="/home/user"
export USER="user"
export XDG_CURRENT_DESKTOP="XFCE"
export XDG_SESSION_TYPE="x11"
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_CACHE_HOME="$HOME/.cache"
export PULSE_SERVER="127.0.0.1:4713"

echo "[AndroidPC] Initializing XFCE 4 Desktop Session on $DISPLAY"

# Clean up stale locks
rm -f /tmp/.X1-lock /tmp/.X11-unix/X1 || true

# Start D-Bus session if available
if command -v dbus-launch >/dev/null 2>&1; then
    eval "$(dbus-launch --sh-syntax --exit-with-session)"
    echo "[AndroidPC] D-Bus session bus started (PID: $DBUS_SESSION_BUS_PID)"
fi

# Launch XFCE Desktop
if command -v xfce4-session >/dev/null 2>&1; then
    echo "[AndroidPC] Launching xfce4-session..."
    exec xfce4-session
elif command -v startxfce4 >/dev/null 2>&1; then
    echo "[AndroidPC] Launching startxfce4..."
    exec startxfce4
else
    echo "[AndroidPC] Falling back to basic X11 window manager..."
    exec xterm
fi
