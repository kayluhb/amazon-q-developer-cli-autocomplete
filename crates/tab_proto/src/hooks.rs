use std::collections::HashMap;

use thiserror::Error;

use crate::local::*;
use crate::util::get_shell;

type Result<T, E = HookError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum HookError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    GetShell(#[from] crate::util::GetShellError),
}

fn hook_enum_to_hook(hook: hook::Hook) -> Hook {
    Hook { hook: Some(hook) }
}

pub fn hook_to_message(hook: Hook) -> LocalMessage {
    LocalMessage {
        r#type: Some(local_message::Type::Hook(hook)),
    }
}

pub fn generate_shell_context(
    pid: impl Into<i32>,
    tty: impl Into<String>,
    session_id: Option<impl Into<String>>,
) -> Result<ShellContext> {
    let cwd = std::env::current_dir()?;
    let shell = get_shell()?;
    Ok(ShellContext {
        pid: Some(pid.into()),
        ttys: Some(tty.into()),
        session_id: session_id.map(|s| s.into()),
        process_name: Some(shell),
        current_working_directory: Some(cwd.to_string_lossy().into()),
        ..Default::default()
    })
}

// Builders for the hook payloads that both the local and the remote protocols send.
// `remote_hooks` uses these too, so the two protocols build the same structs.

pub(crate) fn edit_buffer_hook(
    context: impl Into<Option<ShellContext>>,
    text: impl Into<String>,
    cursor: i64,
    histno: i64,
    coords: impl Into<Option<TerminalCursorCoordinates>>,
) -> EditBufferHook {
    EditBufferHook {
        context: context.into(),
        terminal_cursor_coordinates: coords.into(),
        text: text.into(),
        cursor,
        histno,
    }
}

pub(crate) fn prompt_hook(context: impl Into<Option<ShellContext>>) -> PromptHook {
    PromptHook {
        context: context.into(),
    }
}

pub(crate) fn preexec_hook(context: impl Into<Option<ShellContext>>) -> PreExecHook {
    PreExecHook {
        context: context.into(),
        command: None,
    }
}

pub(crate) fn intercepted_key_hook(
    context: impl Into<Option<ShellContext>>,
    action: impl Into<String>,
    key: impl Into<String>,
) -> InterceptedKeyHook {
    InterceptedKeyHook {
        context: context.into(),
        action: action.into(),
        key: key.into(),
    }
}

/// Construct a edit buffer hook
pub fn new_edit_buffer_hook(
    context: impl Into<Option<ShellContext>>,
    text: impl Into<String>,
    cursor: i64,
    histno: i64,
    coords: impl Into<Option<TerminalCursorCoordinates>>,
) -> Hook {
    hook_enum_to_hook(hook::Hook::EditBuffer(edit_buffer_hook(
        context, text, cursor, histno, coords,
    )))
}

/// Construct a new hook
pub fn new_init_hook(context: impl Into<Option<ShellContext>>) -> Result<Hook> {
    let env_map: HashMap<_, _> = std::env::vars().collect();

    Ok(hook_enum_to_hook(hook::Hook::Init(InitHook {
        context: context.into(),
        called_direct: false,
        bundle: "".into(), // GetCurrentTerminal()?.PotentialBundleId()?
        env: env_map,
    })))
}

/// Construct a new prompt hook
pub fn new_prompt_hook(context: impl Into<Option<ShellContext>>) -> Hook {
    hook_enum_to_hook(hook::Hook::Prompt(prompt_hook(context)))
}

pub fn new_preexec_hook(context: impl Into<Option<ShellContext>>) -> Hook {
    hook_enum_to_hook(hook::Hook::PreExec(preexec_hook(context)))
}

pub fn new_keyboard_focus_changed_hook(
    app_identifier: impl Into<String>,
    focused_session_id: impl Into<String>,
) -> Hook {
    hook_enum_to_hook(hook::Hook::KeyboardFocusChanged(KeyboardFocusChangedHook {
        app_identifier: app_identifier.into(),
        focused_session_id: focused_session_id.into(),
    }))
}

pub fn new_ssh_hook(
    context: impl Into<Option<ShellContext>>,
    control_path: impl Into<String>,
    remote_dest: impl Into<String>,
) -> Result<Hook> {
    Ok(hook_enum_to_hook(hook::Hook::OpenedSshConnection(
        OpenedSshConnectionHook {
            context: context.into(),
            control_path: control_path.into(),
            remote_hostname: remote_dest.into(),
        },
    )))
}

pub fn new_integration_ready_hook(identifier: impl Into<String>) -> Hook {
    hook_enum_to_hook(hook::Hook::IntegrationReady(IntegrationReadyHook {
        identifier: identifier.into(),
    }))
}

pub fn new_hide_hook() -> Hook {
    hook_enum_to_hook(hook::Hook::Hide(HideHook {}))
}

pub fn new_event_hook(
    event_name: impl Into<String>,
    payload: impl Into<Option<String>>,
    apps: impl Into<Vec<String>>,
) -> Hook {
    hook_enum_to_hook(hook::Hook::Event(EventHook {
        event_name: event_name.into(),
        payload: payload.into(),
        apps: apps.into(),
    }))
}

pub fn new_file_changed_hook(
    file_changed: file_changed_hook::FileChanged,
    filepath: impl Into<Option<String>>,
) -> Hook {
    hook_enum_to_hook(hook::Hook::FileChanged(FileChangedHook {
        file_changed: file_changed.into(),
        filepath: filepath.into(),
    }))
}

pub fn new_callback_hook(handler_id: impl Into<String>, filepath: impl Into<String>, exit_code: i64) -> Hook {
    hook_enum_to_hook(hook::Hook::Callback(CallbackHook {
        handler_id: handler_id.into(),
        filepath: filepath.into(),
        exit_code: exit_code.to_string(),
    }))
}

pub fn new_intercepted_key_hook(
    context: impl Into<Option<ShellContext>>,
    action: impl Into<String>,
    key: impl Into<String>,
) -> Hook {
    hook_enum_to_hook(hook::Hook::InterceptedKey(intercepted_key_hook(context, action, key)))
}

pub fn new_caret_position_hook(x: f64, y: f64, width: f64, height: f64, origin: caret_position_hook::Origin) -> Hook {
    hook_enum_to_hook(hook::Hook::CaretPosition(CaretPositionHook {
        x,
        y,
        width,
        height,
        origin: Some(origin as i32),
    }))
}

pub fn new_clear_autocomplete_cache(clis: Vec<String>) -> Hook {
    hook_enum_to_hook(hook::Hook::ClearAutocompleteCache(ClearAutocompleteCacheHook { clis }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_hook_builders_set_fields() {
        let context = generate_shell_context(1, "tty", Some("session_id")).unwrap();

        let intercepted_key = intercepted_key_hook(context.clone(), "action", "key");
        assert_eq!(intercepted_key.action, "action");
        assert_eq!(intercepted_key.key, "key");
        assert_eq!(intercepted_key.context, Some(context.clone()));

        let edit_buffer = edit_buffer_hook(context.clone(), "text", 2, 3, None);
        assert_eq!(edit_buffer.text, "text");
        assert_eq!(edit_buffer.cursor, 2);
        assert_eq!(edit_buffer.histno, 3);

        assert_eq!(preexec_hook(context.clone()).command, None);
        assert_eq!(prompt_hook(context.clone()).context, Some(context));
    }

    #[test]
    fn new_hooks() {
        let context = generate_shell_context(1, "tty", Some("session_id")).unwrap();

        let _ = new_edit_buffer_hook(context.clone(), "text", 0, 0, None);
        let _ = new_init_hook(context.clone()).unwrap();
        let _ = new_prompt_hook(context.clone());
        let _ = new_preexec_hook(context.clone());
        let _ = new_keyboard_focus_changed_hook("app_identifier", "focused_session_id");
        let _ = new_ssh_hook(context.clone(), "control_path", "remote_dest").unwrap();
        let _ = new_integration_ready_hook("identifier");
        let _ = new_hide_hook();
        let _ = new_event_hook("event_name", Some("payload".into()), vec!["app".into()]);
        let _ = new_file_changed_hook(file_changed_hook::FileChanged::Settings, Some("filepath".into()));
        let _ = new_callback_hook("handler_id", "filepath", 0);
        let _ = new_intercepted_key_hook(context, "action", "key");
        let _ = new_caret_position_hook(0.0, 0.0, 0.0, 0.0, caret_position_hook::Origin::BottomLeft);
        let _ = new_clear_autocomplete_cache(vec!["cli".into()]);
    }
}
