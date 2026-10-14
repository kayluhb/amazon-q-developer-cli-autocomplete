use tab_proto::fig::server_originated_message::Submessage as ServerOriginatedSubMessage;
use tab_proto::fig::{
    GetSettingsPropertyRequest,
    GetSettingsPropertyResponse,
    UpdateSettingsPropertyRequest,
};
use tab_settings::Settings;

use super::{
    RequestResult,
    value_store,
};

pub async fn get(request: GetSettingsPropertyRequest) -> RequestResult {
    let json_blob = value_store::get_json_blob(&Settings::new(), "settings", request.key)?;

    let response = ServerOriginatedSubMessage::GetSettingsPropertyResponse(GetSettingsPropertyResponse {
        json_blob: Some(json_blob),
        is_default: None,
    });

    Ok(response.into())
}

pub async fn update(request: UpdateSettingsPropertyRequest) -> RequestResult {
    value_store::update(&Settings::new(), request.key.as_deref(), request.value)
}
