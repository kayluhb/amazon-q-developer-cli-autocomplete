use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{
    AtomicU64,
    Ordering,
};

use anyhow::{
    Context,
    Result,
};
use tab_ipc::{
    BufferedReader,
    RecvMessage,
    SendMessage,
};
use tab_proto::figterm::{
    InsertTextRequest,
    InterceptRequest,
    SetBufferRequest,
    SetInlineSuggestionRequest,
    intercept_request,
};
use tab_proto::local::ShellContext;
use tab_proto::remote::clientbound::request::Request;
use tab_proto::remote::clientbound::{
    self,
    HandshakeResponse,
};
use tab_proto::remote::{
    Clientbound,
    Hostbound,
    RunProcessRequest,
    hostbound,
};
use tab_util::PTY_BINARY_NAME;
use time::OffsetDateTime;
use tokio::net::UnixStream;
use tokio::select;
use tokio::sync::Notify;
use tokio::time::{
    Duration,
    Instant,
    MissedTickBehavior,
};
use tracing::{
    debug,
    error,
    info,
    trace,
    warn,
};
use uuid::Uuid;

use crate::RemoteHookHandler;
use crate::figterm::{
    EditBuffer,
    FigtermCommand,
    FigtermSession,
    FigtermState,
    InterceptMode,
};

pub async fn start_remote_ipc(
    socket_path: PathBuf,
    figterm_state: Arc<FigtermState>,
    hook: impl RemoteHookHandler + Send + Clone + 'static,
) -> Result<()> {
    let listener = tab_ipc::socket_bind(&socket_path)
        .await
        .context("Failed binding remote socket")?;

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_remote_ipc(stream, figterm_state.clone(), hook.clone()));
    }

    Ok(())
}

pub async fn handle_remote_ipc(
    stream: UnixStream,
    figterm_state: Arc<FigtermState>,
    mut hook: impl RemoteHookHandler + Send,
) {
    let (reader, writer) = tokio::io::split(stream);
    let (clientbound_tx, clientbound_rx) = flume::unbounded();

    let bad_connection = Arc::new(Notify::new());

    let (on_close_tx, mut on_close_rx) = tokio::sync::broadcast::channel(1);

    let outgoing_task = tokio::spawn(handle_outgoing(
        writer,
        clientbound_rx,
        bad_connection.clone(),
        on_close_tx.subscribe(),
    ));

    let ping_task = tokio::spawn(send_pings(clientbound_tx.clone(), on_close_tx.subscribe()));

    let mut initialized = false;
    let session_id = Uuid::new_v4();

    let mut reader = BufferedReader::new(reader);
    loop {
        tokio::select! {
            _ = on_close_rx.recv() => {
                debug!("Connection closed");
                break;
            }
            message = reader.recv_message::<Hostbound>() => match message {
                Ok(Some(message)) => {
                    // The handshake carries the client secret, so do not log it
                    if !matches!(message.packet, Some(hostbound::Packet::Handshake(_))) {
                        trace!(?message, "Received remote message");
                    }
                    if let Some(response) = match message.packet {
                        Some(hostbound::Packet::Handshake(handshake)) => {
                            let result = if initialized {
                                // maybe they missed our response, but they should've been listening harder
                                Some(clientbound::Packet::HandshakeResponse(HandshakeResponse {
                                    success: false,
                                }))
                            } else {
                                initialized = true;
                                let (command_tx, command_rx) = flume::unbounded();
                                tokio::spawn(handle_commands(command_rx, figterm_state.clone(), session_id));
                                debug!("Client auth for {} accepted", handshake.id);
                                figterm_state.insert(FigtermSession {
                                    id: session_id,
                                    sender: command_tx,
                                    writer: Some(clientbound_tx.clone()),
                                    dead_since: None,
                                    last_receive: Instant::now(),
                                    edit_buffer: EditBuffer {
                                        text: "".to_string(),
                                        cursor: 0,
                                    },
                                    context: None,
                                    terminal_cursor_coordinates: None,
                                    current_session_metrics: None,
                                    response_map: HashMap::new(),
                                    nonce_counter: Arc::new(AtomicU64::new(0)),
                                    on_close_tx: on_close_tx.clone(),
                                    intercept: InterceptMode::Unlocked,
                                    intercept_global: InterceptMode::Unlocked
                                });
                                Some(clientbound::Packet::HandshakeResponse(HandshakeResponse {
                                    success: true,
                                }))
                            };

                            if matches!(result, Some(clientbound::Packet::HandshakeResponse(HandshakeResponse { success: true }))) {
                                if let Some(parent_id) = handshake.parent_id {
                                    let inner = figterm_state.inner.lock();
                                    let sessions = inner.linked_sessions.values();
                                    for session in sessions {
                                        if let Some(ref writer) = session.writer {
                                            let notification = clientbound::Packet::NotifyChildSessionStarted(
                                                clientbound::NotifyChildSessionStarted { parent_id: parent_id.clone() }
                                            );
                                            writer.send(
                                                Clientbound {
                                                    packet: Some(notification)
                                                }
                                            ).ok();
                                        }
                                    }
                                }
                            }

                            result
                        },
                        Some(hostbound::Packet::Request(hostbound::Request { request: Some(mut request), nonce })) => {
                            if matches!(
                                request,
                                hostbound::request::Request::EditBuffer(_)
                                | hostbound::request::Request::Prompt(_)
                                | hostbound::request::Request::PreExec(_)
                                | hostbound::request::Request::InterceptedKey(_)
                            ) && !initialized {
                                debug!("Client tried to send remote hook without auth");
                                Some(clientbound::Packet::HandshakeResponse(HandshakeResponse {
                                    success: false,
                                }))
                            } else {
                                if let Some(shell_context) = sanitize_request(&mut request, session_id) {
                                    hook.shell_context(shell_context, session_id).await;
                                }

                                let res = match request {
                                    hostbound::request::Request::EditBuffer(edit_buffer) => {
                                        hook.edit_buffer(&edit_buffer, session_id, &figterm_state).await
                                    },
                                    hostbound::request::Request::Prompt(prompt) => {
                                        hook.prompt(&prompt, session_id, &figterm_state).await
                                    },
                                    hostbound::request::Request::PreExec(pre_exec) => {
                                        hook.pre_exec(&pre_exec, session_id, &figterm_state).await
                                    },
                                    hostbound::request::Request::PostExec(post_exec) => {
                                        hook.post_exec(&post_exec, session_id, &figterm_state).await
                                    },
                                    hostbound::request::Request::InterceptedKey(intercepted_key) => {
                                        hook.intercepted_key(intercepted_key, session_id).await
                                    },
                                };

                                match res {
                                    Ok(inner) => inner.map(|inner| clientbound::Packet::Response(clientbound::Response { nonce, response: Some(inner) })),
                                    Err(err) => {
                                        error!(%err, "Failed processing hook");
                                        None
                                    }
                                }
                            }
                        },
                        Some(hostbound::Packet::Response(hostbound::Response {
                            nonce,
                            response: Some(response),
                        })) => {
                            if initialized {
                                if let Some(nonce) = nonce {
                                    figterm_state
                                        .with(&session_id, |session| session.response_map.remove(&nonce))
                                        .flatten()
                                        .map(|channel| channel.send(response));
                                }
                            }
                            None
                        },
                        Some(hostbound::Packet::Pong(())) => {
                            trace!(?session_id, "Received pong");
                            figterm_state.with(&session_id, |session| {
                                session.last_receive = Instant::now();
                            });
                            None
                        },
                        Some(hostbound::Packet::Request(hostbound::Request { request: None, .. })
                            | hostbound::Packet::Response(hostbound::Response { response: None, .. }))
                            | None => {
                            warn!(?message.packet, "Received unknown remote packet");
                            None
                        }
                    } {
                        let _ = clientbound_tx.send(Clientbound { packet: Some(response) });
                    }
                }
                Ok(None) => {
                    debug!("{PTY_BINARY_NAME} connection closed");
                    break;
                }
                Err(err) => {
                    if !err.is_disconnect() {
                        warn!(%err, "Failed receiving remote message");
                    }
                    break;
                }
            }
        }
    }

    let _ = on_close_tx.send(());
    drop(clientbound_tx);

    // figterm_state.with_update(session_id.clone(), |session| {
    //     session.writer = None;
    //     session.dead_since = Some(Instant::now());
    // });
    figterm_state.remove_id(&session_id);

    if let Err(err) = ping_task.await {
        error!(%err, "remote ping task join error");
    }

    if let Err(err) = outgoing_task.await {
        error!(%err, "remote outgoing task join error");
    }

    info!("Disconnect from {session_id:?}");
}

async fn handle_outgoing(
    mut writer: tokio::io::WriteHalf<UnixStream>,
    outgoing: flume::Receiver<Clientbound>,
    bad_connection: Arc<Notify>,
    mut on_close_rx: tokio::sync::broadcast::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = on_close_rx.recv() => {
                debug!("remote outgoing task exiting");
                break;
            },
            message = outgoing.recv_async() => {
                if let Ok(message) = message {
                    trace!(?message, "Sending remote message");
                    if let Err(err) = writer.send_message(message).await {
                        error!(%err, "remote outgoing task send error");
                        bad_connection.notify_one();
                        return;
                    }
                } else {
                    debug!("remote outgoing task exiting");
                    break;
                }
            }
        }
    }
}

async fn handle_commands(
    incoming: flume::Receiver<FigtermCommand>,
    figterm_state: Arc<FigtermState>,
    session_id: Uuid,
) -> Option<()> {
    while let Ok(command) = incoming.recv_async().await {
        let (request, nonce_channel) = match command {
            FigtermCommand::InterceptFigJs {
                intercept_keystrokes,
                intercept_global_keystrokes,
                actions,
                override_actions,
            } => (
                Request::Intercept(InterceptRequest {
                    intercept_command: Some(intercept_request::InterceptCommand::SetFigjsIntercepts(
                        intercept_request::SetFigjsIntercepts {
                            intercept_bound_keystrokes: intercept_keystrokes,
                            intercept_global_keystrokes,
                            actions,
                            override_actions,
                        },
                    )),
                }),
                None,
            ),
            FigtermCommand::InterceptFigJSVisible { visible } => (
                Request::Intercept(InterceptRequest {
                    intercept_command: Some(intercept_request::InterceptCommand::SetFigjsVisible(
                        intercept_request::SetFigjsVisible { visible },
                    )),
                }),
                None,
            ),
            FigtermCommand::InsertText {
                insertion,
                deletion,
                offset,
                immediate,
                insertion_buffer,
                insert_during_command,
            } => (
                Request::InsertText(InsertTextRequest {
                    insertion,
                    deletion: deletion.map(|x| x as u64),
                    offset,
                    immediate,
                    insertion_buffer,
                    insert_during_command,
                }),
                None,
            ),
            FigtermCommand::SetBuffer { text, cursor_position } => {
                (Request::SetBuffer(SetBufferRequest { text, cursor_position }), None)
            },
            FigtermCommand::SetInlineSuggestion { buffer, suffix } => (
                Request::SetInlineSuggestion(SetInlineSuggestionRequest { buffer, suffix }),
                None,
            ),
            FigtermCommand::RunProcess {
                channel,
                executable,
                arguments,
                working_directory,
                env,
                timeout,
            } => (
                Request::RunProcess(RunProcessRequest {
                    executable,
                    arguments,
                    working_directory,
                    env,
                    timeout: timeout.map(Into::into),
                }),
                Some(channel),
            ),
        };

        let nonce = if let Some(channel) = nonce_channel {
            Some(figterm_state.with(&session_id, |session| {
                let nonce = session.nonce_counter.fetch_add(1, Ordering::Relaxed);
                session.response_map.insert(nonce, channel);
                nonce
            })?)
        } else {
            None
        };

        let is_insert_request = matches!(request, Request::InsertText(_));
        figterm_state.with(&session_id, |session| {
            if let Some(writer) = &session.writer {
                if writer
                    .try_send(Clientbound {
                        packet: Some(clientbound::Packet::Request(clientbound::Request {
                            request: Some(request),
                            nonce,
                        })),
                    })
                    .is_ok()
                {
                    if is_insert_request {
                        if let Some(ref mut metrics) = session.current_session_metrics {
                            metrics.num_insertions += 1;
                            metrics.end_time =
                                OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
                        }
                    }
                    session.last_receive = Instant::now();
                };
            }
        })?;
    }

    None
}

async fn send_pings(outgoing: flume::Sender<Clientbound>, mut on_close_rx: tokio::sync::broadcast::Receiver<()>) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        select! {
            _ = interval.tick() => {
                let _ = outgoing.try_send(Clientbound {
                    packet: Some(clientbound::Packet::Ping(())),
                });
            }
            _ = on_close_rx.recv() => break
        }
    }
}

/// Overwrites the session id in the shell context of `request` with the id of the connection.
///
/// A client can put any session id in a hook. Every request must go through this function
/// before dispatch, so that a client cannot forge a message from another session. The match
/// has no wildcard arm, so a new request type does not compile until it is handled here.
///
/// Returns the sanitized shell context, if the request has one.
fn sanitize_request(request: &mut hostbound::request::Request, session_id: Uuid) -> Option<&ShellContext> {
    use hostbound::request::Request;

    let context = match request {
        Request::EditBuffer(hook) => &mut hook.context,
        Request::Prompt(hook) => &mut hook.context,
        Request::PreExec(hook) => &mut hook.context,
        Request::PostExec(hook) => &mut hook.context,
        Request::InterceptedKey(hook) => &mut hook.context,
    };

    let context = context.as_mut()?;
    context.session_id = Some(session_id.to_string());
    Some(context)
}

#[cfg(test)]
mod tests {
    use tab_proto::local::{
        EditBufferHook,
        InterceptedKeyHook,
        PostExecHook,
        PreExecHook,
        PromptHook,
    };

    use super::*;

    fn forged_context() -> Option<ShellContext> {
        Some(ShellContext {
            session_id: Some("forged".into()),
            ..Default::default()
        })
    }

    #[test]
    fn sanitize_request_overwrites_session_id_for_every_request() {
        use hostbound::request::Request;

        let session_id = Uuid::new_v4();
        let requests = [
            Request::EditBuffer(EditBufferHook {
                context: forged_context(),
                ..Default::default()
            }),
            Request::Prompt(PromptHook {
                context: forged_context(),
            }),
            Request::PreExec(PreExecHook {
                context: forged_context(),
                ..Default::default()
            }),
            Request::PostExec(PostExecHook {
                context: forged_context(),
                ..Default::default()
            }),
            Request::InterceptedKey(InterceptedKeyHook {
                context: forged_context(),
                ..Default::default()
            }),
        ];

        for mut request in requests {
            let sanitized = sanitize_request(&mut request, session_id).and_then(|context| context.session_id.clone());
            assert_eq!(sanitized, Some(session_id.to_string()), "{request:?}");
        }
    }

    #[test]
    fn sanitize_request_without_context_returns_none() {
        let mut request = hostbound::request::Request::Prompt(PromptHook { context: None });
        assert!(sanitize_request(&mut request, Uuid::new_v4()).is_none());
    }
}
