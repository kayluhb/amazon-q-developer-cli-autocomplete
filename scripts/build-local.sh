#!/usr/bin/env bash
# Build the app locally and ad-hoc sign it. No Apple Developer ID needed.
#
# Usage: scripts/build-local.sh [extra build-scripts/main.py build args]
#   e.g. scripts/build-local.sh --skip-tests --skip-lints

set -euo pipefail

cd "$(dirname "$0")/.."

missing=()
for tool in cargo rustup pnpm protoc cargo-tauri python3.11; do
  command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
done
if ((${#missing[@]})); then
  echo "Missing tools: ${missing[*]}" >&2
  echo "Run scripts/setup.sh; cargo-tauri needs: cargo +1.79.0 install tauri-cli@1.6.0 --locked" >&2
  exit 1
fi

[ -d .venv ] || python3.11 -m venv .venv
.venv/bin/pip install -q -r build-scripts/requirements.txt

.venv/bin/python build-scripts/main.py build "$@"

app_name="$(cd build-scripts && ../.venv/bin/python -c 'import const; print(const.APP_NAME)')"
app="build/${app_name}.app"

# Accessibility permission is tied to the code signature, so after each
# rebuild macOS may ask for it again.
codesign --force --deep --sign - "$app"
codesign --verify --deep --strict "$app"

echo
echo "Built and ad-hoc signed: $app"
echo "Install: rm -rf \"/Applications/${app_name}.app\" && cp -R \"$app\" /Applications/"
