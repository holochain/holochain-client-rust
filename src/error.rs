use holochain_conductor_api::ExternalApiWireError;
use holochain_websocket::WebsocketError;

#[derive(Debug)]
pub enum ConductorApiError {
    WebsocketError(WebsocketError),
    ExternalApiWireError(ExternalApiWireError),
}

pub type ConductorApiResult<T> = Result<T, ConductorApiError>;
