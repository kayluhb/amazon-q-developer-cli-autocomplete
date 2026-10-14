# Offline inline suggestions (ghost text)

Date: 2026-10-14
Status: approved design, pending implementation plan

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
                      specs    → `tab _ inline-suggestion --buffer "$BUFFER"`
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
  Keep its MIT licence header. Rename `{{CLI_BINARY_NAME}}` placeholders the same way as the other Tab shell scripts.
- Keep the `history` and `match_prev_cmd` strategies. Add a `specs` strategy:
  ```zsh
  _tab_autosuggest_strategy_specs() {
  	typeset -g suggestion="$(command -v tab >/dev/null 2>&1 && tab _ inline-suggestion --buffer "${BUFFER}" 2>/dev/null)"
  }
  ```
  (The actual script uses the binary-name placeholders.)
- Remove `strategies/inline.zsh` (AI) and `strategies/completion.zsh` (zpty).
- Concatenate the files at build time in `crates/tab_integrations/build.rs`, as upstream did. Expose the result as
  `tab_integrations::shell::inline_plugin::ZSH_SCRIPT`.
- Keep upstream's default highlight style `fg=8`. `post.zsh` already reports
  `TAB_AUTOSUGGEST_HIGHLIGHT_STYLE` to `tabterm` via `TabAutosuggestionColor`, and `tabterm` leaves cells with
  that colour out of the edit buffer (`crates/alacritty_terminal/src/term/mod.rs:1467`). The popup therefore
  never treats ghost text as typed input.
- Check for a conflict when the plugin starts. If `_zsh_autosuggest_start` is already defined (the user loads
  zsh-autosuggestions), Tab's plugin does not bind its widgets.
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
  - then load `ZSH_SCRIPT` with a source guard, so a second `tab init` in the same shell doesn't load it twice.
- If the setting is missing or invalid, treat it as `off`.

### 3. Dashboard (`packages/dashboard-app/src/data/autocomplete.ts`)

- New section "Inline suggestions" with one `select` property:
  - `id`: `inline.source`
  - default: `off`
  - options: Off, History, Specs, History then specs
- Description: "Grey suggestions on the command line. zsh only. Applies to new terminal windows. → accepts."

### 4. Spec suffix in the popup (`packages/autocomplete-app/src/state/inlineSuggestion.ts`)

The build ships `packages/autocomplete-app` (`build-scripts/build.py:146`). Mirror every change into
`packages/autocomplete`, because its `src/` is currently an identical copy.

`inlineSuffix(item, searchTerm, buffer, cursorIndex, visibleState, settings): string` is a pure function. It
returns `""` when any of these holds:
- `cursorIndex !== buffer.length`
- `visibleState === HIDDEN_UNTIL_KEYPRESS` (the suggestions are stale)
- `item` is missing, is a history suggestion, or has `execute` set
- the full insertion (from `getFullInsertion`, so escaping matches the popup's) contains `{cursor}` anywhere but the end
- the full insertion does not start with the query term (from `getQueryTermForSuggestion`). This rules out fuzzy matches.

Otherwise it returns `insertion.slice(queryTerm.length)`, plus `" "` when `item.shouldAddSpace` and
`autocomplete.insertSpaceAutomatically` are both true. It strips a trailing `{cursor}`.

Call site: after the `updateSuggestions` middleware stores new suggestions. The popup computes the suffix
from `suggestions[0]` (the top suggestion, not the selected one) and calls `Shell.setInlineSuggestion`
with `{sessionId, buffer, suffix}`, but only when the triple has changed since the last send. It sends an empty
suffix when nothing qualifies, so `tabterm` never keeps serving an outdated suggestion.

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
- `proto/remote.proto`: a `Clientbound.Request` oneof entry carrying the `figterm.proto` message.
- The multiplexer (`proto/mux.proto`) is not supported. Over a remote or mux connection, inline specs simply return nothing.

### 6. tabterm cache and lookup (`crates/tabterm/src/`)

- New `inline_suggestion.rs` with `InlineSuggestionCache`:
  - holds `Option<{buffer, suffix}>` and a `tokio::sync::watch` (or `Notify`) to wake any waiting lookups
  - `set(buffer, suffix)` overwrites the entry
  - `lookup(buffer, timeout) -> Option<String>` returns the suffix right away if the cached buffer matches. Otherwise
    it waits until either a matching `set` arrives or the timeout (150 ms) runs out.
- `message.rs`:
  - handle `SetInlineSuggestion` in `process_remote_message`/`process_figterm_request`
  - replace the current silent drop of `InlineShellCompletion` (`message.rs:295`) with a spawned lookup task
    that always replies with `InlineShellCompletionResponse { insert_text }`, which may be `None`
  - keep `InlineShellCompletionAccept`, `InlineShellCompletionSetEnabled` and `Telemtety` dropped
- `send_figterm_response_hostbound`: `InlineShellCompletion` stays local-only. It is never sent over remote.

### 7. CLI (`crates/tab_cli/src/cli/internal/`)

New hidden subcommand `tab _ inline-suggestion --buffer <BUFFER>`, based on
`b654a1be:crates/q_cli/src/cli/internal/inline_shell_completion.rs`:
1. Read `TABTERM_SESSION_ID`. If it's missing, exit 0 with no output.
2. Connect to `figterm_socket_path(session_id)`. If that fails, exit 0 with no output.
3. `send_recv_message_timeout(InlineShellCompletionRequest { buffer }, 300ms)`. The outer timeout is longer than
   `tabterm`'s 150 ms wait.
4. On `insert_text: Some(t)` with `t` not empty, print `{buffer}{t}` with no trailing newline. In every
   other case, print nothing.

stdout carries only the suggestion. Errors go to the log only.

### 8. Doctor (`crates/tab_cli/src/cli/doctor/`)

When `inline.source` is not `off` and the shell is zsh, add a check that warns if zsh-autosuggestions is
also loaded. Detect it with `functions _zsh_autosuggest_start` in an interactive `zsh -i -c` (or the closest
pattern the existing doctor checks use). Warning text: "zsh-autosuggestions is loaded; Tab inline
suggestions are inactive."

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
- **insta snapshots** of `tab init zsh post` for each `inline.source` value, including missing and invalid values.
- **Script checks**: the joined plugin keeps its licence text, defines `_tab_autosuggest_strategy_specs`,
  adds the precmd hook, and contains no `inline_shell_completion` strategy. `zsh -n` passes on it.
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
- **Duplicate TS packages:** changes have to be mirrored between `autocomplete` and `autocomplete-app`. A
  separate task should merge them.
