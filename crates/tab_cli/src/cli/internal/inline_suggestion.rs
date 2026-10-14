use std::io::{
    Write,
    stdout,
};
use std::time::Duration;

use eyre::{
    Result,
    bail,
};
use tab_ipc::{
    BufferedUnixStream,
    SendRecvMessage,
};
use tab_proto::figterm::figterm_request_message::Request;
use tab_proto::figterm::figterm_response_message::Response;
use tab_proto::figterm::{
    FigtermRequestMessage,
    FigtermResponseMessage,
    InlineShellCompletionRequest,
    InlineShellCompletionResponse,
};
use tab_util::directories::figterm_socket_path;
use tab_util::env_var::TABTERM_SESSION_ID;
use tracing::debug;

/// Longer than tabterm's own wait for the popup, so tabterm always answers first
const TIMEOUT: Duration = Duration::from_millis(300);

/// Prints the inline suggestion for `buffer` (the buffer followed by the suggested text), or
/// nothing.
///
/// Called by the zsh inline suggestions plugin on each keystroke, so every failure prints nothing
/// rather than an error.
pub async fn inline_suggestion(buffer: String) {
    match fetch_suffix(&buffer).await {
        Ok(Some(suffix)) => {
            let mut stdout = stdout().lock();
            let _ = write!(stdout, "{buffer}{suffix}");
            let _ = stdout.flush();
        },
        Ok(None) => {},
        Err(err) => debug!(%err, "No inline suggestion"),
    }
}

async fn fetch_suffix(buffer: &str) -> Result<Option<String>> {
    let Ok(session_id) = std::env::var(TABTERM_SESSION_ID) else {
        return Ok(None);
    };

    let mut conn = BufferedUnixStream::connect(figterm_socket_path(&session_id)?).await?;
    let response: Option<FigtermResponseMessage> = conn
        .send_recv_message_timeout(
            FigtermRequestMessage {
                request: Some(Request::InlineShellCompletion(InlineShellCompletionRequest {
                    buffer: buffer.to_owned(),
                })),
            },
            TIMEOUT,
        )
        .await?;

    match response {
        Some(FigtermResponseMessage {
            response: Some(Response::InlineShellCompletion(InlineShellCompletionResponse { insert_text })),
        }) => Ok(insert_text.filter(|text| !text.is_empty())),
        None => Ok(None),
        res => bail!("unexpected response from tabterm: {res:?}"),
    }
}
