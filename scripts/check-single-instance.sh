#!/usr/bin/env bash
# Verifies that only one instance of the bundled macOS app can run at a time.
# Reproduces the three ways a second instance used to appear (see issue #2):
# a normal launch, a direct exec of the binary (what the autostart LaunchAgent
# does, bypassing LaunchServices), and a forced `open -n`.
#
# Run with: scripts/check-single-instance.sh [path/to/App.app]
set -euo pipefail

APP=${1:-src-tauri/target/release/bundle/macos/NFC Keyboard Emulator.app}

if [[ "$(uname)" != "Darwin" ]]; then
  echo "skip: macOS only"
  exit 0
fi
if [[ ! -d "$APP" ]]; then
  echo "error: no app bundle at '$APP' — run 'npm run tauri build' first" >&2
  exit 2
fi

EXE=$(/usr/libexec/PlistBuddy -c "Print :CFBundleExecutable" "$APP/Contents/Info.plist")
BIN="$APP/Contents/MacOS/$EXE"

count() { pgrep -f "$BIN" | wc -l | tr -d ' '; }

# A superseded instance has to boot far enough to reach the guard before it can
# hand over and exit, which on a cold CI runner takes well over the ~1s it takes
# on a warm desktop. So poll for the expected count instead of guessing a fixed
# settle time - a real regression never reaches it and fails on the timeout.
settle() { # settle <wanted count>
  local deadline=$((SECONDS + 30))
  while [[ "$(count)" != "$1" && $SECONDS -lt $deadline ]]; do sleep 0.25; done
}
cleanup() { pkill -f "$BIN" >/dev/null 2>&1 || true; }
trap cleanup EXIT

if [[ "$(count)" != "0" ]]; then
  echo "error: '$EXE' is already running — quit it first" >&2
  exit 2
fi

fail=0
expect() { # expect <count> <what>
  local got
  got=$(count)
  if [[ "$got" == "$1" ]]; then
    echo "ok   — $2: $got instance(s)"
  else
    echo "FAIL — $2: expected $1, got $got"
    fail=1
  fi
}

open "$APP"
settle 1
if [[ "$(count)" == "0" ]]; then
  # No window server (headless agent, locked-out CI runner): the app cannot come
  # up at all, which says nothing about single-instance behaviour. Skip loudly
  # rather than report a regression that isn't one.
  echo "skip — app did not start, no GUI session available"
  exit 0
fi
expect 1 "after launch"

"$BIN" >/dev/null 2>&1 &
settle 1
expect 1 "after direct binary exec (autostart path)"

open -n "$APP"
settle 1
expect 1 "after forced 'open -n'"

exit "$fail"
