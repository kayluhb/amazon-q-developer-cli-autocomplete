use std::io::{
    Read,
    Write,
    stdin,
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

/// Reads the buffer from stdin and prints the inline suggestion for it (the buffer followed by the
/// suggested text), or nothing.
///
/// The buffer comes over stdin rather than argv so other users can't read it with `ps`. Called by
/// the zsh inline suggestions plugin on each keystroke, so every failure prints nothing rather than
/// an error.
pub async fn inline_suggestion() {
    let mut buffer = String::new();
    if let Err(err) = stdin().read_to_string(&mut buffer) {
        debug!(%err, "Failed reading buffer from stdin");
        return;
    }

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
    let response = conn
        .send_recv_message_timeout(
            FigtermRequestMessage {
                request: Some(Request::InlineShellCompletion(InlineShellCompletionRequest {
                    buffer: buffer.to_owned(),
                })),
            },
            TIMEOUT,
        )
        .await?;

    suffix_from_response(response)
}

fn suffix_from_response(response: Option<FigtermResponseMessage>) -> Result<Option<String>> {
    match response {
        Some(FigtermResponseMessage {
            response: Some(Response::InlineShellCompletion(InlineShellCompletionResponse { insert_text })),
        }) => Ok(insert_text.filter(|text| !text.is_empty())),
        None => Ok(None),
        res => bail!("unexpected response from tabterm: {res:?}"),
    }
}

#[cfg(test)]
mod tests {
    use tab_proto::figterm::DiagnosticsResponse;

    use super::*;

    fn completion(insert_text: Option<&str>) -> Option<FigtermResponseMessage> {
        Some(FigtermResponseMessage {
            response: Some(Response::InlineShellCompletion(InlineShellCompletionResponse {
                insert_text: insert_text.map(Into::into),
            })),
        })
    }

    #[test]
    fn suffix_from_response_cases() {
        assert_eq!(
            suffix_from_response(completion(Some("ckout"))).unwrap().as_deref(),
            Some("ckout")
        );
        assert_eq!(suffix_from_response(completion(Some(""))).unwrap(), None);
        assert_eq!(suffix_from_response(completion(None)).unwrap(), None);
        assert_eq!(suffix_from_response(None).unwrap(), None);
        assert!(
            suffix_from_response(Some(FigtermResponseMessage {
                response: Some(Response::Diagnostics(DiagnosticsResponse::default())),
            }))
            .is_err()
        );
    }
}
