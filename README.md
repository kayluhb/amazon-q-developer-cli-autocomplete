# Tab

IDE-style autocomplete for your existing macOS terminal, fully offline.

Tab is a standalone fork of
[aws/amazon-q-developer-cli-autocomplete](https://github.com/aws/amazon-q-developer-cli-autocomplete)
(formerly Fig). It keeps only the graphical terminal autocomplete, which Kiro CLI is
[sunsetting](https://github.com/kirodotdev/Kiro/issues/11728), and removes everything that
needs an AWS backend:

- No login, telemetry, update checks, chat, translate or AI inline completions.
  All AWS service clients have been removed.
- Completion specs are built from [withfig/autocomplete](https://github.com/withfig/autocomplete)
  (MIT) at a pinned commit and bundled into the app, together with their icons.
- macOS only. The app is ad-hoc signed, so you build it yourself.

Tab is not affiliated with or endorsed by Amazon Web Services.

## Build and install

```sh
scripts/setup.sh              # once: installs toolchains
scripts/build-local.sh        # builds build/Tab.app and ad-hoc signs it
cp -R build/Tab.app /Applications/
open /Applications/Tab.app    # onboarding installs the shell integrations
```

Accessibility permission is tied to the app's signature, so you may need to
re-grant it in System Settings after each rebuild.

## Usage

```sh
tab doctor      # check the installation
tab settings    # view or change settings
tab --help-all  # all commands
```

Custom specs in `~/Library/Application Support/tab/autocomplete/specs/` override the
bundled ones. Use the same layout as the withfig build output (`index.json` plus
`<name>.js`).

To uninstall, run `tab uninstall`.

## Project layout

- [`packages/autocomplete`](packages/autocomplete/): the autocomplete React app
- [`packages/dashboard-app`](packages/dashboard-app/): the settings dashboard
- [`crates/tabterm`](crates/tabterm/): `tabterm`, the pseudoterminal that reads the
  terminal edit buffer
- [`crates/tab_cli`](crates/tab_cli/): the `tab` CLI
- [`crates/tab_desktop`](crates/tab_desktop/): the desktop app
  ([`tao`](https://docs.rs/tao/latest/tao/)/[`wry`](https://docs.rs/wry/latest/wry/))
- [`crates/tab_input_method`](crates/tab_input_method/): the input method that gets the
  cursor position
- [`build-scripts/`](build-scripts/): Python build scripts
- [`proto/`](proto/): protocol buffer messages for IPC
- [`tests/`](tests/): integration tests

![architecture](docs/assets/architecture.svg)

## Licensing

Dual licensed under [MIT](LICENSE.MIT) and [Apache 2.0](LICENSE.APACHE), the same as
the original project. The changes from upstream are in this repository's git history.

"Amazon Web Services", "Amazon Q" and related marks are trademarks of AWS.
