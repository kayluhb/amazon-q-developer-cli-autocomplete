use tab_proto::fig::server_originated_message::Submessage as ServerOriginatedSubMessage;
use tab_proto::fig::{
    GetLocalStateRequest,
    GetLocalStateResponse,
    UpdateLocalStateRequest,
};
use tab_settings::State;

use super::{
    RequestResult,
    value_store,
};

pub async fn get(request: GetLocalStateRequest) -> RequestResult {
    let json_blob = value_store::get_json_blob(&State::new(), "state", request.key)?;

    let response = ServerOriginatedSubMessage::GetLocalStateResponse(GetLocalStateResponse {
        json_blob: Some(json_blob),
    });

    Ok(response.into())
}

pub async fn update(request: UpdateLocalStateRequest) -> RequestResult {
    value_store::update(&State::new(), request.key.as_deref(), request.value)
}
