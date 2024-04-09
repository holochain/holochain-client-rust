use holochain_conductor_api::ExternalApiWireError;
use holochain_state::prelude::DatabaseError;
use holochain_websocket::Error as WebsocketError;

#[derive(Debug)]
pub enum ConductorApiError {
    WebsocketError(WebsocketError),
    ExternalApiWireError(ExternalApiWireError),
    FreshNonceError(DatabaseError),
    SignZomeCallError(String),
    CellNotFound,
}

pub type ConductorApiResult<T> = Result<T, ConductorApiError>;
