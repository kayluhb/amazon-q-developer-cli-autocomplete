use tab_proto::fig::{
    CheckForUpdatesRequest,
    CheckForUpdatesResponse,
};

use super::{
    RequestResult,
    ServerOriginatedSubMessage,
};

/// The standalone build has no update server; updates come from rebuilding
pub async fn check_for_updates(_request: CheckForUpdatesRequest) -> RequestResult {
    Ok(Box::new(ServerOriginatedSubMessage::CheckForUpdatesResponse(
        CheckForUpdatesResponse {
            is_update_available: Some(false),
            version: None,
        },
    )))
}
