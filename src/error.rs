use std::error::Error;

use holochain_conductor_api::ExternalApiWireError;
use holochain_websocket::WebsocketError;

#[derive(Debug)]
pub enum ConductorApiError {
    WebsocketError(WebsocketError),
    ExternalApiWireError(ExternalApiWireError),
    FreshNonceError(Box<dyn Error + Sync + Send>),
    SignZomeCallError(String),
    CellNotFound,
}

pub type ConductorApiResult<T> = Result<T, ConductorApiError>;
