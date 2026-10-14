use tab_proto::fig::{
    InsertTextRequest,
    SetInlineSuggestionRequest,
};
use tab_remote_ipc::figterm::{
    FigtermCommand,
    FigtermState,
};
use uuid::Uuid;

use super::{
    RequestResult,
    RequestResultImpl,
};

pub async fn insert_text(request: InsertTextRequest, state: &FigtermState) -> RequestResult {
    let figterm_command = match request.r#type {
        Some(some) => match some {
            tab_proto::fig::insert_text_request::Type::Text(text) => FigtermCommand::InsertText {
                insertion: Some(text),
                deletion: None,
                immediate: None,
                offset: None,
                insertion_buffer: None,
                insert_during_command: None,
            },
            tab_proto::fig::insert_text_request::Type::Update(update) => FigtermCommand::InsertText {
                insertion: update.insertion,
                deletion: update.deletion,
                immediate: update.immediate,
                offset: update.offset,
                insertion_buffer: update.insertion_buffer,
                insert_during_command: None,
            },
        },
        None => return RequestResult::error("InsertTextRequest expects a request type"),
    };

    send_figterm_command(request.terminal_session_id, figterm_command, state)
}

pub async fn set_inline_suggestion(request: SetInlineSuggestionRequest, state: &FigtermState) -> RequestResult {
    let figterm_command = FigtermCommand::SetInlineSuggestion {
        buffer: request.buffer,
        suffix: request.suffix,
    };
    send_figterm_command(request.terminal_session_id, figterm_command, state)
}

fn send_figterm_command(
    terminal_session_id: Option<String>,
    figterm_command: FigtermCommand,
    state: &FigtermState,
) -> RequestResult {
    let uuid_str = terminal_session_id.ok_or("terminal_session_id is required")?;
    let uuid = Uuid::parse_str(&uuid_str).map_err(|err| format!("terminal_session_id is not a valid UUID: {err}"))?;

    match state.with(&uuid, |session| session.sender.clone()) {
        Some(sender) => {
            sender
                .send(figterm_command)
                .map_err(|err| format!("Failed sending command to figterm session: {err}"))?;
            RequestResult::success()
        },
        None => RequestResult::error("No figterm sessions"),
    }
}
