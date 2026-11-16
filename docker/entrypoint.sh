#!/usr/bin/env bash
# Launch Lekhani Latex inside the container.
#
# Two display modes:
#
#   1. X11 passthrough — start with DISPLAY set and the host's
#      /tmp/.X11-unix socket mounted; the window appears on that display and
#      no VNC stack is started.
#   2. Browser (default) — Xvfb + openbox + x11vnc + noVNC run inside the
#      container. Point a browser at http://localhost:${NOVNC_PORT}.
#
# Tunables (env): SCREEN_SIZE, XVFB_DISPLAY, VNC_PORT, NOVNC_PORT.

set -euo pipefail

# Bare paths (e.g. `docker run IMAGE /workspace/main.tex`) open that document;
# anything else is executed as given (`lekhani-latex`, `bash`, ...).
if [[ "${1:-}" == /* || "${1:-}" == ./* ]]; then
  set -- lekhani-latex "$@"
fi

X_DISPLAY="${DISPLAY:-}"
if [[ -n "$X_DISPLAY" && -e "/tmp/.X11-unix/X${X_DISPLAY#:}" ]]; then
  echo "lekhani: using host X server at $X_DISPLAY" >&2
  exec "$@"
fi

XVFB_DISPLAY="${XVFB_DISPLAY:-99}"
SCREEN_SIZE="${SCREEN_SIZE:-1600x900x24}"
VNC_PORT="${VNC_PORT:-5900}"
NOVNC_PORT="${NOVNC_PORT:-6080}"

export DISPLAY=":${XVFB_DISPLAY}"
export XDG_RUNTIME_DIR="/tmp/runtime-$(id -u)"
mkdir -p "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"

Xvfb ":${XVFB_DISPLAY}" -screen 0 "$SCREEN_SIZE" -nolisten tcp >/dev/null 2>&1 &

for _ in $(seq 1 50); do
  [[ -e "/tmp/.X11-unix/X${XVFB_DISPLAY}" ]] && break
  sleep 0.1
done
if [[ ! -e "/tmp/.X11-unix/X${XVFB_DISPLAY}" ]]; then
  echo "lekhani: Xvfb did not come up on :${XVFB_DISPLAY}" >&2
  exit 1
fi

# Tiny window manager so the editor window gets decorations and stacking.
openbox >/dev/null 2>&1 &

# x11vnc binds loopback only; websockify (noVNC) is the public entry point.
x11vnc -display ":${XVFB_DISPLAY}" -localhost -forever -shared -nopw \
  -rfbport "$VNC_PORT" -quiet -bg >/dev/null 2>&1

websockify --web=/usr/share/novnc "$NOVNC_PORT" "localhost:${VNC_PORT}" \
  >/dev/null 2>&1 &

# Private D-Bus session + XDG portal so rfd file dialogs work headless.
eval "$(dbus-launch --sh-syntax)"
xdg-desktop-portal >/dev/null 2>&1 &

cat >&2 <<EOF

  Lekhani Latex is starting. Open it in a browser:
    http://localhost:${NOVNC_PORT}/vnc.html?autoconnect=true&resize=scale

EOF

exec "$@"
