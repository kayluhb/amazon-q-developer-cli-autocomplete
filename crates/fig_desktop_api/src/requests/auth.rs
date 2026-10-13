use fig_proto::fig::{
    AuthStatusRequest,
    AuthStatusResponse,
};

use super::{
    RequestResult,
    ServerOriginatedSubMessage,
};

/// The standalone build has no account; always report signed out
pub async fn status(_request: AuthStatusRequest) -> RequestResult {
    Ok(ServerOriginatedSubMessage::AuthStatusResponse(AuthStatusResponse {
        authed: false,
        auth_kind: None,
        start_url: None,
        region: None,
    })
    .into())
}
