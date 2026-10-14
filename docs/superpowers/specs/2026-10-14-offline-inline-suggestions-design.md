# Offline inline suggestions (ghost text)

Date: 2026-10-14
Status: approved design, revised after code review against the tree (2026-10-14), implementing

## Goal

Bring back grey "ghost text" suggestions on the zsh command line, without any network access.
Kiro CLI and Amazon Q had this feature, but their suggestions came from an AI backend. Tab removed it
in `bc9174be`. This design brings it back with two offline sources: shell history and the bundled
completion specs.

## Decisions

| Topic | Decision |
| --- | --- |
| Shells | zsh only. Fish has built-in autosuggestions. Bash has no line-editor support for this. |
| Sources | One setting, `inline.source`: `off`, `history`, `specs`, `history-specs` |
| Default | `off` (opt in from the dashboard) |
| When a change applies | New terminal windows. `tab init` reads the setting when the shell starts. |
| Specs source | The popup's top suggestion, converted to a suffix and cached in `tabterm` |
| Delivery | All three sources ship together |
| Out of scope | Bash/fish/nushell, AI suggestions, the upstream one-time "try inline" prompt, the `zpty` `completion` strategy, merging `packages/autocomplete` and `packages/autocomplete-app` |

## Architecture

```
 zsh keystroke ──▶ plugin (vendored zsh-autosuggestions, async mode)
                    strategies, from inline.source:
                      history  → pure zsh
                      specs    → `print -rn -- "$1" | tab _ inline-suggestion`
                                    │  local socket (TABTERM_SESSION_ID)
                                    ▼
                                 tabterm: cache {buffer, suffix}
                                    ▲  waits ≤150ms for an entry whose buffer matches
                                    │  remote.proto: SetInlineSuggestion
                                 desktop app (forwards by session id)
                                    ▲  fig.proto: SetInlineSuggestionRequest
 popup webview: after updateSuggestions → suffix from suggestions[0]
```

The cache lives in `tabterm`, not in the desktop app. That way each session has its own cache, the
cache goes away when the session ends, and the CLI → `tabterm` request and response types
(`InlineShellCompletionRequest`/`Response` in `proto/figterm.proto`) already exist. A cache in the
desktop app would need a new CLI → desktop message type instead.

## Components

### 1. zsh plugin (`crates/tab_integrations/src/shell/inline/`)

- Restore the zsh-autosuggestions fork from `b654a1be:crates/fig_integrations/src/shell/inline_shell_completion/`.
  Keep its MIT licence header. Write the function and variable prefixes out as `_tab_autosuggest_` and
  `TAB_AUTOSUGGEST_` (as `post.zsh` already does for `TAB_AUTOSUGGEST_HIGHLIGHT_STYLE`). Keep only the
  `{{CLI_BINARY_NAME}}` placeholder, which `get_fig_integration_source` already substitutes for the other scripts.
- Keep the `history` and `match_prev_cmd` strategies. Add a `specs` strategy:
  ```zsh
  _tab_autosuggest_strategy_specs() {
  	[[ -n "${TABTERM_SESSION_ID}" ]] || return
  	typeset -g suggestion="$(command -v tab >/dev/null 2>&1 && print -rn -- "$1" | tab _ inline-suggestion 2>/dev/null)"
  }
  ```
  (The actual script uses the binary-name placeholders.)
  - The buffer goes over stdin, not argv. Other local users can read argv with `ps`, so a password typed on the
    command line would leak one prefix at a time.
  - The strategy uses `$1`, as the other strategies do. It does not start a process when `tabterm` is not running.
- Remove `strategies/inline.zsh` (AI) and `strategies/completion.zsh` (zpty), and the
  `COMPLETIONS_PTY_NAME` config that only the zpty strategy used.
- Remove the `{{CLI_BINARY_NAME}} _ inline-shell-completion-accept` calls from the `accept` and `execute` widgets.
  That subcommand was AI telemetry and no longer exists, so each accept would spawn a failing process.
- Default `TAB_AUTOSUGGEST_STRATEGY` to `(match_prev_cmd history)` instead of the AI strategy.
- Concatenate the files with `concat!(include_str!(…))` in `tab_integrations::shell::inline_plugin`, in upstream's
  order, and substitute `{{CLI_BINARY_NAME}}` at runtime. No `build.rs`: the crate has none, and the other shell
  scripts are already handled this way. Expose `tab_integrations::shell::inline_plugin::zsh_script() -> String`.
- Keep upstream's default highlight style `fg=8`. `post.zsh` already reports
  `TAB_AUTOSUGGEST_HIGHLIGHT_STYLE` to `tabterm` on each prompt via `FigAutosuggestionColor`, and `tabterm` leaves cells
  with that colour out of the edit buffer (`crates/alacritty_terminal/src/term/mod.rs:1467`, which prefers this colour
  over `ZshAutosuggestionColor`). The popup therefore never treats ghost text as typed input.
- Keep upstream's conflict guard (`guard_start.zsh`/`guard_end.zsh`). If `_zsh_autosuggest_accept` is already defined
  (the user loads zsh-autosuggestions earlier in `.zshrc`), the guard exports `TAB_USING_ZSH_AUTOSUGGESTIONS=1` and
  skips the whole plugin. `tab doctor` reads that variable (§8).
- Keys are the plugin defaults:
  - accept the whole suggestion: `forward-char` (→) and `end-of-line` (End)
  - accept a word: `forward-word` (Option+→)
  - The popup does not bind → by default, so the two don't clash.

### 2. Shell init (`crates/tab_cli/src/cli/init.rs`)

- Plugin loading follows the upstream rules from `b654a1be:crates/q_cli/src/cli/init.rs:155-174`. In `shell_init`,
  for `When::Post`, `Shell::Zsh`, and an rc file that is not `zprofile`, with `inline.source` not `off`:
  - set `TAB_AUTOSUGGEST_STRATEGY` from the setting:
    - `history` → `(match_prev_cmd history)`
    - `specs` → `(specs)`
    - `history-specs` → `(match_prev_cmd history specs)`
  - then load `zsh_script()` with a source guard (`TAB_INLINE_PLUGIN_SOURCED`), so a second `tab init` in the
    same shell doesn't load it twice.
- If the setting is missing or invalid, treat it as `off`.
- Put the mapping in a pure function, `inline_plugin_source(source: Option<&str>) -> Option<String>`, so it can be
  unit tested without a settings file.
- Skip the plugin when `TAB_INIT_SNAPSHOT_TEST` is set, as upstream did. The `tab init zsh post` snapshot is linted
  with `shellcheck -s bash`, and the plugin is zsh-only syntax.

### 3. Dashboard (`packages/dashboard-app/src/data/autocomplete.ts`)

- New section "Inline suggestions" with one `select` property:
  - `id`: `inline.source`
  - default: `off`
  - options: `off`, `history`, `specs`, `history-specs` (the dashboard `select` shows option values as they are;
    it has no separate labels)
- Description: "Grey suggestions on the command line from history and/or completion specs. zsh only. Applies to
  new terminal windows. Press → to accept."

### 4. Spec suffix in the popup (`packages/autocomplete-app/src/state/inlineSuggestion.ts`)

The build ships `packages/autocomplete-app` (`build-scripts/build.py:146`). Mirror every change into
`packages/autocomplete`, because its `src/` is currently an identical copy.

`inlineSuffix(item, state): string` is a pure function. `state` is the part of the store it reads
(`parserResult`, `figState`, `visibleState`, `fuzzySearchEnabled`, `settings`). It returns `""` when any of these holds:
- `cursorIndex !== buffer.length`
- `visibleState === HIDDEN_UNTIL_KEYPRESS` (the suggestions are stale)
- `item` is missing, or its type is `history`, `auto-execute`, `special` or `shortcut` (suggestions have no
  `execute` field; these types run, don't insert text, or delete more than the query term)
- the full insertion (from `getFullInsertion`, so escaping matches the popup's) contains `{cursor}` anywhere but the
  end, or contains a control character (`\n` would execute)
- the full insertion does not start with the query term (from `getQueryTermForSuggestion`). This rules out fuzzy
  matches and case-insensitive matches.
- the buffer does not end with the query term. This rules out escaping or quoting differences between what was
  typed and what the parser saw (for example `cat "my fi`).

Otherwise it returns `insertion.slice(queryTerm.length)`, plus `" "` when `item.shouldAddSpace` and
`autocomplete.insertSpaceAutomatically` are both true. It strips a trailing `{cursor}`, and then it adds no space,
because the popup leaves the cursor before that space.

`sendInlineSuggestion(state)` does the sending. It does nothing unless the session id is known and the
`inline.source` setting (`SETTINGS.INLINE_SOURCE`) is `specs` or `history-specs`, so the popup adds no IPC traffic
when spec ghost text is off.

Call site: in the `updateSuggestions` middleware, after every `set` (both the early-return path and the
recompute path). It sends nothing while `parsedCommand` (the command that `parserResult` came from, recorded by
`setParserResult`) is not the current `command`. The buffer and command change before the async parse finishes,
so without this check the popup would send suggestions from the old parse with the new buffer. The popup computes the suffix from `suggestions[0]` of the new state (the top suggestion, not the
selected one) and calls `Shell.setInlineSuggestion`
with `{sessionId, buffer, suffix}`, but only when the triple has changed since the last send. It sends an empty
suffix when nothing qualifies, so `tabterm` never keeps serving an outdated suggestion. Send errors (for example
no `tabterm` session) are caught and logged at debug level.

### 5. Message path: popup → desktop → tabterm

- `proto/fig.proto`: `SetInlineSuggestionRequest { string buffer; string suffix; optional string terminal_session_id; }`,
  added to the `ClientOriginatedMessage` oneof.
- `packages/api-bindings`: regenerate `requests.ts`. Add `setInlineSuggestion` to `shell.ts`.
- `crates/tab_desktop_api/src/handler.rs`: a trait method and a dispatch arm.
- `crates/tab_desktop/src/request/{mod.rs,figterm.rs}`: parse the session id and send a `FigtermCommand`.
- `crates/tab_remote_ipc/src/figterm.rs`: a `FigtermCommand::SetInlineSuggestion` variant.
- `crates/tab_remote_ipc/src/remote.rs`: a `handle_commands` arm that needs no response.
- `proto/figterm.proto`: `SetInlineSuggestionRequest { string buffer; string suffix; }`, added to the
  `FigtermRequestMessage` oneof (this is the `tabterm`-side message; the fig.proto one carries the session id).
- `proto/remote.proto`: a `Clientbound.Request` oneof entry carrying the `figterm.proto` message (field 108).
- The multiplexer (`proto/mux.proto`) is not supported. Over a remote or mux connection, inline specs simply return nothing.

### 6. tabterm cache and lookup (`crates/tabterm/src/`)

- New `inline_suggestion.rs` with `InlineSuggestionCache`, held in a process-wide static like the other `tabterm`
  state in `main.rs`:
  - holds `Option<{buffer, suffix}>` and a `tokio::sync::watch` (or `Notify`) to wake any waiting lookups
  - `set(buffer, suffix)` overwrites the entry
  - `lookup(buffer, timeout) -> Option<String>` returns the suffix right away if the cached buffer matches and the
    suffix is not empty. Otherwise it waits until such a `set` arrives or the timeout (150 ms) runs out. An empty
    suffix does not end the wait, because the popup sends an empty suffix first and a real one when its
    generators finish.
- `message.rs`:
  - handle `SetInlineSuggestion` in `process_figterm_request` (shared by the local and remote paths), and route it
    from `process_remote_message`
  - replace the current silent drop of `InlineShellCompletion` (`message.rs:295`) with a spawned lookup task
    that always replies with `InlineShellCompletionResponse { insert_text }`, which may be `None`
  - keep `InlineShellCompletionAccept`, `InlineShellCompletionSetEnabled` and `Telemtety` dropped
- `send_figterm_response_hostbound`: `InlineShellCompletion` stays local-only. It is never sent over remote.

### 7. CLI (`crates/tab_cli/src/cli/internal/`)

New hidden subcommand `tab _ inline-suggestion`, which reads the buffer from stdin. It is based on
`b654a1be:crates/q_cli/src/cli/internal/inline_shell_completion.rs`:
1. Read `TABTERM_SESSION_ID`. If it's missing, exit 0 with no output.
2. Connect to `figterm_socket_path(session_id)`. If that fails, exit 0 with no output.
3. `send_recv_message_timeout(InlineShellCompletionRequest { buffer }, 300ms)`. The outer timeout is longer than
   `tabterm`'s 150 ms wait.
4. On `insert_text: Some(t)` with `t` not empty, print `{buffer}{t}` with no trailing newline. In every
   other case, print nothing.

stdout carries only the suggestion. Errors go to the log only.

### 8. Doctor (`crates/tab_cli/src/cli/doctor/mod.rs`)

An `InlineCheck` is already there, left over from upstream. It reads `inline.enabled` (default `true`) and
suggests `tab inline enable`/`disable`, which no longer exist. Rewrite it:
- Run it only when the shell is zsh and `inline.source` is not `off`. Otherwise `NoCheck`.
- Keep detection through `Env::q_using_zsh_autosuggestions()` (`TAB_USING_ZSH_AUTOSUGGESTIONS`, exported by the
  plugin guard). This needs no extra `zsh -i` process.
- Return a warning, not an error: "zsh-autosuggestions is loaded; Tab inline suggestions are inactive." with the
  fixes "remove zsh-autosuggestions from ~/.zshrc" or "set Inline suggestions to off in the dashboard".

## Error handling

Ghost text must never break typing or show an error. Every failure ends the same way: no suggestion.

- Not under `tabterm`, socket missing, timeout, cache mismatch, or a protocol error → empty output, exit 0.
- Generators slower than 150 ms → no spec ghost text for that keystroke. A later keystroke can still pick it up.
- The plugin runs in async mode (zsh ≥ 5.0.8), so the `tab` process it starts on each keystroke never blocks input.

## Testing

- **vitest** for `inlineSuffix`:
  - prefix match
  - fuzzy match (no prefix)
  - `{cursor}` in the middle and at the end
  - `execute`
  - `shouldAddSpace` with the setting on and off
  - a file name with a space (escaped)
  - a history item
  - cursor not at the end of the line
  - `HIDDEN_UNTIL_KEYPRESS`
- **Rust unit tests** for `InlineSuggestionCache`:
  - an immediate hit
  - a mismatch then a timeout
  - a wait resolved by a later `set`
  - a newer `set` for a different buffer
- **Rust unit tests** for `inline_plugin_source`: each `inline.source` value, plus missing and invalid values.
  The existing `tab init` snapshots stay unchanged (§2).
- **Script checks** (unit tests in `tab_integrations`): the joined plugin keeps its licence text, defines
  `_tab_autosuggest_strategy_specs`, adds the precmd hook, has no `{{` placeholders left, and contains no
  `inline_shell_completion` or `inline-shell-completion` text. `zsh -n` passes on it.
- **Manual checklist**:
  - Terminal.app, iTerm2, Ghostty and tmux, each with all four modes
  - zsh-autosuggestions loaded: Tab's plugin stays inactive and `tab doctor` warns
  - with the popup open, ghost text never shows up in the popup's search term
  - no noticeable typing lag

## Risks

- **Process spawn cost:** one `tab` process per keystroke in the `specs` modes. Async mode hides this delay, but
  measure it. If it hurts, a follow-up could have zsh talk to the socket directly (`zsocket`) instead.
- **Ghost text colour detection:** this depends on the suggestion colour being distinct. A prompt or theme that
  also uses `fg=8` in the command line region could confuse `tabterm`. zsh-autosuggestions users have the same issue today.
- **Buffer equality:** `tabterm` matches the zsh `$BUFFER` sent by the CLI against the edit buffer the popup saw,
  which `tabterm` parses from the screen. Anything that makes the two differ (multi-line buffers, right prompts,
  unusual themes) gives no spec ghost text. That is safe, but measure how often it happens.
- **Duplicate TS packages:** changes have to be mirrored between `autocomplete` and `autocomplete-app`. A
  separate task should merge them.
