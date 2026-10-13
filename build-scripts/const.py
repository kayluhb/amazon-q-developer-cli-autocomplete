import pathlib


APP_NAME = "Tab"
CLI_BINARY_NAME = "tab"
CLI_BINARY_NAME_MINIMAL = "tab-minimal"
PTY_BINARY_NAME = "tabterm"
DESKTOP_BINARY_NAME = "tab-desktop"
URL_SCHEMA = "tab"
TAURI_PRODUCT_NAME = "tab_desktop"

# macos specific
MACOS_BUNDLE_ID = "com.kayluhb.tab"
DMG_NAME = APP_NAME

# Linux specific

# cargo packages
CLI_PACKAGE_NAME = "tab_cli"
PTY_PACKAGE_NAME = "tabterm"
DESKTOP_PACKAGE_NAME = "tab_desktop"
DESKTOP_FUZZ_PACKAGE_NAME = "tab_desktop-fuzz"

DESKTOP_PACKAGE_PATH = pathlib.Path("crates", "tab_desktop")

# AMZN Mobile LLC
APPLE_TEAM_ID = "94KV3E626L"
