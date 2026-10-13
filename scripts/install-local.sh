#!/usr/bin/env bash
# Install (or reinstall) build/Tab.app into /Applications.
#
# Replaces files instead of copying over them: macOS kills a signed binary
# that was modified in place. Also refreshes the tabterm copies in
# ~/.local/bin, which the app only updates when the version changes.

set -euo pipefail

cd "$(dirname "$0")/.."

read -r app_name cli_name pty_name process_name < <(cd build-scripts && ../.venv/bin/python -c \
  'import const; print(const.APP_NAME, const.CLI_BINARY_NAME, const.PTY_BINARY_NAME, const.TAURI_PRODUCT_NAME)')
src="build/${app_name}.app"
dest="/Applications/${app_name}.app"

[ -d "$src" ] || { echo "$src not found, run scripts/build-local.sh first" >&2; exit 1; }

if pgrep -xq "$process_name"; then
  "$dest/Contents/MacOS/$cli_name" quit || true
  for _ in {1..20}; do
    pgrep -xq "$process_name" || break
    sleep 0.5
  done
  if pgrep -xq "$process_name"; then
    echo "$process_name is still running, quit it and try again" >&2
    exit 1
  fi
fi

rm -rf "$dest"
cp -R "$src" "$dest"
codesign --verify --deep --strict "$dest"

for copy in "$HOME"/.local/bin/*" ($pty_name)"; do
  [ -e "$copy" ] || continue
  rm -f "$copy"
  cp "$dest/Contents/MacOS/$pty_name" "$copy"
done

open "$dest"
echo "Installed $dest. Open a new terminal window to use it."
