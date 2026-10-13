# Tab

IDE-style autocomplete for your existing macOS terminal, fully offline.

Tab is a standalone fork of
[aws/amazon-q-developer-cli-autocomplete](https://github.com/aws/amazon-q-developer-cli-autocomplete)
(formerly Fig). It keeps only the graphical terminal autocomplete, which Kiro CLI is
[sunsetting](https://github.com/kirodotdev/Kiro/issues/11728), and removes everything that
needs an AWS backend:

- No login, telemetry, update checks, chat, translate or AI inline completions.
  All AWS service clients have been removed, and Tab makes no network requests.
- Completion specs are built from [withfig/autocomplete](https://github.com/withfig/autocomplete)
  (MIT) at a pinned commit and bundled into the app, together with their icons.
- macOS only. You build it yourself and sign it with a local certificate, so no Apple Developer
  account is needed.

Tab is not affiliated with or endorsed by Amazon Web Services.

## Requirements

- macOS with Xcode command line tools and [Homebrew](https://brew.sh)
- [rustup](https://rustup.rs). The toolchain is pinned in `rust-toolchain.toml` (Rust 1.88);
  formatting uses `nightly-2025-11-01`
- Node 22, Python 3.11, pnpm and protobuf. `scripts/setup.sh` installs them with Homebrew
  and [mise](https://mise.jdx.dev)
- The Tauri 1 CLI, which needs an older Rust to build:

  ```sh
  rustup toolchain install 1.79.0
  cargo +1.79.0 install tauri-cli@1.6.0 --locked
  ```

Uninstall or disable Kiro CLI, Amazon Q or Fig first. Their shell hooks conflict with Tab's.

## Build and install

```sh
scripts/setup.sh                # once: installs toolchains
scripts/create-signing-cert.sh  # once: local code signing certificate
scripts/build-local.sh          # builds and signs build/Tab.app
scripts/install-local.sh        # installs to /Applications and launches it
```

On first launch, onboarding installs the shell integrations (a pre and post block in
`~/.zshrc`, `~/.bashrc` and so on) and asks for **Accessibility** permission in
System Settings → Privacy & Security → Accessibility. Then open a new terminal window.

`build-local.sh` passes its arguments to the build script, for example
`scripts/build-local.sh --skip-tests --skip-lints` for a faster rebuild.

### Updating

```sh
git pull
scripts/build-local.sh
scripts/install-local.sh
```

Use `install-local.sh` rather than copying the app over the installed one: macOS kills a
signed binary that is modified in place, and the script also refreshes the `tabterm` copies
in `~/.local/bin`.

Accessibility permission is tied to the app's signature. With the local certificate it
carries over between rebuilds. Without it, the app is ad-hoc signed and you have to remove
and re-add Tab under Accessibility after each rebuild.

## Usage

Type a command and Tab suggests subcommands, options and arguments. Default keys:

| Key | Action |
| --- | --- |
| ↑ / ↓ | Move through suggestions |
| Enter | Insert the selected suggestion |
| Tab | Insert the common prefix |
| Esc | Hide the popup |
| Ctrl+R | Toggle history mode |

Change them in `tab dashboard` under Keybindings.

```sh
tab doctor      # check the installation
tab settings    # view or change settings
tab dashboard   # open the settings dashboard
tab --help-all  # all commands
```

### Custom specs

Specs in `~/Library/Application Support/tab/autocomplete/specs/` override the bundled ones.
Use the same layout as the withfig build output: an `index.json` listing the specs, plus
`<name>.js` files. To bundle newer withfig specs, change `SPECS_COMMIT` in
`build-scripts/build.py` and rebuild.

### Troubleshooting

- Run `tab doctor` in a new terminal window. It must run inside a Tab session, so it reports
  that `tabterm` is not running in terminals opened before installing.
- No popup: check that Tab is enabled under Accessibility. If it is listed but autocomplete
  still doesn't work, remove it with **–** and add `/Applications/Tab.app` again.
- Logs are in `$TMPDIR/tablog` (for example `tab_desktop.log`).

## Uninstall

```sh
tab uninstall
security delete-identity -c "Tab Local Code Signing"   # optional: remove the signing certificate
```

`tab uninstall` removes the app, the shell integrations, the input method and Tab's data in
`~/Library/Application Support/tab`.

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
pnpm install && pnpm test --run && pnpm lint
```

Pre-commit hooks run rustfmt, clippy, prettier, ruff and typos on staged files. Three Rust
tests need `fish` and `nu` installed.

## Project layout

- [`crates/tab_cli`](crates/tab_cli/): the `tab` CLI
- [`crates/tabterm`](crates/tabterm/): `tabterm`, the pseudoterminal that reads the terminal
  edit buffer
- [`crates/tab_desktop`](crates/tab_desktop/): the desktop app
  ([`tao`](https://docs.rs/tao/latest/tao/)/[`wry`](https://docs.rs/wry/latest/wry/)) that
  shows the autocomplete window
- [`crates/tab_input_method`](crates/tab_input_method/): the input method that gets the cursor
  position
- [`crates/tab_integrations`](crates/tab_integrations/): shell, SSH, input method and VS Code
  integrations
- [`packages/autocomplete-app`](packages/autocomplete-app/): the autocomplete React app
- [`packages/dashboard-app`](packages/dashboard-app/): the settings dashboard
- [`build-scripts/`](build-scripts/): Python build scripts
- [`proto/`](proto/): protocol buffer messages for IPC
- [`tests/`](tests/): integration tests

## Licensing

Dual licensed under [MIT](LICENSE.MIT) and [Apache 2.0](LICENSE.APACHE), the same as
the original project. The changes from upstream are in this repository's git history.

"Amazon Web Services", "Amazon Q" and related marks are trademarks of AWS.
