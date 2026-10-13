use tab_proto::fig::{
    TelemetryPageRequest,
    TelemetryTrackRequest,
};

use super::{
    RequestResult,
    RequestResultImpl,
};

// Telemetry is not collected in the standalone build; accept and drop events

pub async fn handle_track_request(_request: TelemetryTrackRequest) -> RequestResult {
    RequestResult::success()
}

pub async fn handle_page_request(_request: TelemetryPageRequest) -> RequestResult {
    RequestResult::success()
}
