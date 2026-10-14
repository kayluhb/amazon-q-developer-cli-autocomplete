use crate::hooks::{
    edit_buffer_hook,
    intercepted_key_hook,
    preexec_hook,
    prompt_hook,
};
use crate::proto::local::{
    PostExecHook,
    ShellContext,
    TerminalCursorCoordinates,
};
use crate::proto::remote::{
    Hostbound,
    hostbound,
};

fn hook_enum_to_hook(request: hostbound::request::Request) -> hostbound::Request {
    hostbound::Request {
        request: Some(request),
        ..Default::default()
    }
}

pub fn hook_to_message(request: hostbound::Request) -> Hostbound {
    Hostbound {
        packet: Some(hostbound::Packet::Request(request)),
    }
}

/// Construct a edit buffer hook
pub fn new_edit_buffer_hook(
    context: impl Into<Option<ShellContext>>,
    text: impl Into<String>,
    cursor: i64,
    histno: i64,
    coords: impl Into<Option<TerminalCursorCoordinates>>,
) -> hostbound::Request {
    hook_enum_to_hook(hostbound::request::Request::EditBuffer(edit_buffer_hook(
        context, text, cursor, histno, coords,
    )))
}

/// Construct a new prompt hook
pub fn new_prompt_hook(context: impl Into<Option<ShellContext>>) -> hostbound::Request {
    hook_enum_to_hook(hostbound::request::Request::Prompt(prompt_hook(context)))
}

pub fn new_preexec_hook(context: impl Into<Option<ShellContext>>) -> hostbound::Request {
    hook_enum_to_hook(hostbound::request::Request::PreExec(preexec_hook(context)))
}

pub fn new_postexec_hook(
    context: impl Into<Option<ShellContext>>,
    command: Option<String>,
    exit_code: Option<i32>,
) -> hostbound::Request {
    hook_enum_to_hook(hostbound::request::Request::PostExec(PostExecHook {
        context: context.into(),
        command,
        exit_code,
    }))
}

pub fn new_intercepted_key_hook(
    context: impl Into<Option<ShellContext>>,
    action: impl Into<String>,
    key: impl Into<String>,
) -> hostbound::Request {
    hook_enum_to_hook(hostbound::request::Request::InterceptedKey(intercepted_key_hook(
        context, action, key,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_hooks() {
        new_edit_buffer_hook(None, "test", 0, 0, None);
        new_prompt_hook(None);
        new_preexec_hook(None);
        new_postexec_hook(None, None, None);
        new_intercepted_key_hook(None, "", "");
    }
}
