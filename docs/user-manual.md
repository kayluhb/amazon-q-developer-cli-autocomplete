# Tab user manual

Tab adds IDE-style autocomplete to your existing macOS terminal. It runs fully offline. To build
and install it, see the [README](../README.md#build-and-install).

## Setting up

On first launch, onboarding does two things:

1. It installs the shell integrations, a pre and post block in `~/.zshrc`, `~/.bashrc` and so on.
2. It asks for **Accessibility** permission in System Settings → Privacy & Security →
   Accessibility.

Then open a new terminal window. Terminal windows that were open before you installed Tab do not
get autocomplete.

## Using autocomplete

Type a command and Tab suggests subcommands, options and arguments. The default keys are:

| Key    | Action                         |
| ------ | ------------------------------ |
| ↑ / ↓  | Move through suggestions       |
| Enter  | Insert the selected suggestion |
| Tab    | Insert the common prefix       |
| Esc    | Hide the popup                 |
| Ctrl+R | Toggle history mode            |

Change the keys in `tab dashboard` under Keybindings.

To turn autocomplete off everywhere, run `tab settings autocomplete.disable true`.

## Terminals

Tab works in Terminal.app, iTerm 2, Hyper and the VS Code terminal. Each one has a card in
`tab dashboard` under Integrations.

A card's **Enabled** label means only that you have not turned Tab off for that terminal. It does
not check that the terminal is set up. For the popup to appear, all of these must also be true:

- Tab has Accessibility permission.
- Autocomplete is not turned off with `autocomplete.disable`.
- For terminals that need the input method, the input method is installed and enabled. See
  below.

### Input method

Some terminals do not expose the cursor position through Accessibility, so Tab reads it with an
input method instead. These are Alacritty, Kitty, WezTerm, Ghostty, Zed, Rio, Nova and the
JetBrains IDEs.

```sh
tab integrations install input-method
tab integrations status input-method
```

### VS Code

Tab installs a small extension into VS Code, VS Code Insiders and VSCodium.

```sh
tab integrations install vscode
tab integrations status vscode
```

## SSH

The SSH integration adds an `Include` block to the bottom of `~/.ssh/config`. When you connect to
a host, Tab forwards a socket to it so that autocomplete can work in the remote shell.

The remote host must also run Tab's shell integrations. Tab builds for macOS only, so in practice
the remote host must be a Mac with Tab installed.

To set it up:

1. On the remote host, make sure `/etc/ssh/sshd_config` has these lines:

   ```
   AcceptEnv TAB_SET_PARENT
   AllowStreamLocalForwarding yes
   ```

2. Restart sshd on the remote host. On macOS, turn Remote Login off and on again in System
   Settings → General → Sharing.
3. Disconnect from the remote host.
4. On your local machine, run `tab integrations install ssh`.
5. Reconnect, then run `tab doctor` on the remote host.

## Settings

```sh
tab settings                 # view or change settings
tab settings <key>           # show one setting
tab settings <key> <value>   # change one setting
tab dashboard                # open the settings dashboard
```

## Custom completion specs

Specs in `~/Library/Application Support/tab/autocomplete/specs/` override the bundled ones. Use the
same layout as the withfig build output: an `index.json` that lists the specs, plus `<name>.js`
files.

## Troubleshooting

- Run `tab doctor` in a new terminal window. It must run inside a Tab session. In a terminal that
  was open before you installed Tab, it reports that `tabterm` is not running.
- No popup: check that Tab is turned on under Accessibility. If Tab is listed but autocomplete
  still does not work, remove it with **–** and add `/Applications/Tab.app` again.
- Logs are in `$TMPDIR/tablog`, for example `tab_desktop.log`.
- To report a bug, open an issue at
  [github.com/kayluhb/tab/issues](https://github.com/kayluhb/tab/issues).

## Uninstalling

```sh
tab uninstall
```

This removes the app, the shell integrations, the input method and Tab's data in
`~/Library/Application Support/tab`.
