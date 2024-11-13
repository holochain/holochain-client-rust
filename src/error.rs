use holochain_conductor_api::ExternalApiWireError;
use std::error::Error;

#[derive(Debug, uniffi::Error)]
pub enum ConductorApiError {
    WebsocketError(holochain_websocket::WebsocketError),
    ExternalApiWireError(ExternalApiWireError),
    FreshNonceError(Box<dyn Error + Sync + Send>),
    SignZomeCallError(String),
    CellNotFound,
}

pub type ConductorApiResult<T> = Result<T, ConductorApiError>;
