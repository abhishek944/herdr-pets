#!/bin/sh
set -eu

PLUGIN_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
STATE_DIR=${HERDR_PLUGIN_STATE_DIR:-"${HOME}/.local/state/herdr/plugins/herdr-pets"}
PID_FILE="$STATE_DIR/renderer.pid"
BINARY_FILE="$STATE_DIR/renderer.binary"
START_FILE="$STATE_DIR/renderer.started"
LOCK_FILE="$STATE_DIR/control.lock"
SESSIONS_DIR="$STATE_DIR/sessions"

mkdir -p "$STATE_DIR" "$SESSIONS_DIR"

# lockf holds a kernel-backed lock for the lifetime of the nested process. It has
# no stale-PID or partially-written lock-file race after a crash.
if [ "${HERDR_PETS_LOCKED:-0}" != 1 ]; then
  HERDR_PETS_LOCKED=1 lockf -t 15 "$LOCK_FILE" sh "$0" "$@"
  exit $?
fi

register_session() {
  socket=${HERDR_SOCKET_PATH:-}
  [ -n "$socket" ] || return 0
  key=$(printf '%s' "$socket" | cksum | awk '{print $1 "-" $2}')
  tmp="$SESSIONS_DIR/$key.tmp.$$"
  printf '%s\n' "$socket" >"$tmp"
  mv "$tmp" "$SESSIONS_DIR/$key"

  for record in "$SESSIONS_DIR"/*; do
    [ -f "$record" ] || continue
    registered=$(cat "$record" 2>/dev/null || true)
    if [ -z "$registered" ] || [ ! -e "$registered" ]; then
      rm -f "$record"
    fi
  done
}

read_pid() {
  [ -f "$PID_FILE" ] || return 1
  pid=$(cat "$PID_FILE" 2>/dev/null || true)
  case "$pid" in
    ''|*[!0-9]*) return 1 ;;
  esac
  printf '%s\n' "$pid"
}

process_executable() {
  LC_ALL=C TZ=UTC ps -ww -p "$1" -o comm= 2>/dev/null | sed 's/^[[:space:]]*//;s/[[:space:]]*$//'
}

process_started() {
  LC_ALL=C TZ=UTC ps -ww -p "$1" -o lstart= 2>/dev/null | sed 's/^[[:space:]]*//;s/[[:space:]]*$//'
}

process_matches_expected() {
  pid=$1
  expected_binary=$2
  expected_start=$3
  [ "$(process_executable "$pid")" = "$expected_binary" ] || return 1
  [ "$(process_started "$pid")" = "$expected_start" ] || return 1
}

pid_matches_identity() {
  pid=$1
  [ -f "$BINARY_FILE" ] && [ -f "$START_FILE" ] || return 1
  expected_binary=$(cat "$BINARY_FILE" 2>/dev/null || true)
  expected_start=$(cat "$START_FILE" 2>/dev/null || true)
  [ -n "$expected_binary" ] && [ -n "$expected_start" ] || return 1
  [ "$(process_executable "$pid")" = "$expected_binary" ] || return 1
  [ "$(process_started "$pid")" = "$expected_start" ] || return 1
}

running_pid() {
  pid=$(read_pid) || return 1
  kill -0 "$pid" 2>/dev/null || return 1
  pid_matches_identity "$pid" || return 1
  printf '%s\n' "$pid"
}

clear_process_state() {
  rm -f "$PID_FILE" "$BINARY_FILE" "$START_FILE"
}

absolute_executable() {
  candidate=$1
  [ -x "$candidate" ] || return 1
  directory=$(CDPATH= cd -- "$(dirname -- "$candidate")" && pwd -P)
  printf '%s/%s\n' "$directory" "$(basename -- "$candidate")"
}

resolve_binary() {
  if [ -n "${HERDR_PETS_BINARY:-}" ]; then
    absolute_executable "$HERDR_PETS_BINARY" && return 0
  fi

  system=$(uname -s)
  machine=$(uname -m)
  case "$system:$machine" in
    Darwin:arm64) packaged="$PLUGIN_ROOT/bin/macos-arm64/herdr-pets" ;;
    Darwin:x86_64) packaged="$PLUGIN_ROOT/bin/macos-x64/herdr-pets" ;;
    Linux:x86_64) packaged="$PLUGIN_ROOT/bin/linux-x64/herdr-pets" ;;
    Linux:aarch64) packaged="$PLUGIN_ROOT/bin/linux-arm64/herdr-pets" ;;
    *) packaged="$PLUGIN_ROOT/bin/unsupported/herdr-pets" ;;
  esac

  for candidate in \
    "$packaged" \
    "$PLUGIN_ROOT/src-tauri/target/release/herdr-pets" \
    "$PLUGIN_ROOT/src-tauri/target/release/bundle/macos/Herdr Pets.app/Contents/MacOS/herdr-pets"
  do
    if absolute_executable "$candidate"; then
      return 0
    fi
  done
  return 1
}

start_renderer() {
  if pid=$(running_pid); then
    echo "herdr-pets: running (pid $pid)"
    return 0
  fi
  clear_process_state

  binary=$(resolve_binary) || {
    echo "herdr-pets: renderer binary is missing" >&2
    return 1
  }

  # The renderer is intentionally quiet; discarding output prevents a faulty
  # WebView process from filling the plugin state directory indefinitely.
  HERDR_PETS_SESSION_REGISTRY="$SESSIONS_DIR" nohup "$binary" >/dev/null 2>&1 &
  pid=$!

  attempts=0
  started=
  while kill -0 "$pid" 2>/dev/null && [ "$attempts" -lt 10 ]; do
    if [ "$(process_executable "$pid")" = "$binary" ]; then
      started=$(process_started "$pid")
      [ -n "$started" ] && break
    fi
    attempts=$((attempts + 1))
    sleep 0.05
  done
  if [ -z "$started" ]; then
    echo "herdr-pets: could not verify renderer startup" >&2
    return 1
  fi

  sleep 0.5
  if ! process_matches_expected "$pid" "$binary" "$started"; then
    # Never signal a PID after its verified identity has changed.
    echo "herdr-pets: renderer identity changed during startup" >&2
    return 1
  fi

  printf '%s\n' "$binary" >"$BINARY_FILE.tmp"
  printf '%s\n' "$started" >"$START_FILE.tmp"
  printf '%s\n' "$pid" >"$PID_FILE.tmp"
  mv "$BINARY_FILE.tmp" "$BINARY_FILE"
  mv "$START_FILE.tmp" "$START_FILE"
  mv "$PID_FILE.tmp" "$PID_FILE"

  if ! running_pid >/dev/null; then
    clear_process_state
    echo "herdr-pets: renderer identity changed during startup" >&2
    return 1
  fi
  echo "herdr-pets: started (pid $pid)"
}

stop_renderer() {
  if ! pid=$(running_pid); then
    clear_process_state
    echo "herdr-pets: stopped"
    return 0
  fi

  if pid_matches_identity "$pid"; then
    kill "$pid" 2>/dev/null || true
  fi
  attempts=0
  while pid_matches_identity "$pid" && kill -0 "$pid" 2>/dev/null && [ "$attempts" -lt 50 ]; do
    attempts=$((attempts + 1))
    sleep 0.1
  done
  if pid_matches_identity "$pid" && kill -0 "$pid" 2>/dev/null; then
    kill -9 "$pid" 2>/dev/null || true
  fi
  clear_process_state
  echo "herdr-pets: stopped"
}

show_status() {
  if pid=$(running_pid); then
    echo "herdr-pets: running (pid $pid)"
  else
    clear_process_state
    echo "herdr-pets: stopped"
  fi
}

register_session
case "${1:-}" in
  on|start) start_renderer ;;
  off|stop) stop_renderer ;;
  status) show_status ;;
  *) echo "usage: $0 on|off|status" >&2; exit 2 ;;
esac
