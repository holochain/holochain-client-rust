use crate::error::{ConductorApiError, ConductorApiResult};
use anyhow::Result;
use event_emitter_rs::EventEmitter;
use holochain_conductor_api::{
    AppAuthenticationRequest, AppAuthenticationToken, AppInfo, AppRequest, AppResponse,
};
use holochain_types::signal::Signal;
use holochain_websocket::{connect, WebsocketConfig, WebsocketSender};
use std::{net::ToSocketAddrs, sync::Arc};
use tokio::sync::Mutex;

/// The core functionality for an app websocket.
#[derive(Clone)]
pub(crate) struct AppWebsocketInner {
    tx: WebsocketSender,
    event_emitter: Arc<Mutex<EventEmitter>>,
}

impl AppWebsocketInner {
    /// Connect to a Conductor API AppWebsocket.
    pub(crate) async fn connect(socket_addr: impl ToSocketAddrs) -> Result<Self> {
        let addr = socket_addr
            .to_socket_addrs()?
            .next()
            .expect("invalid websocket address");
        let websocket_config = Arc::new(WebsocketConfig::CLIENT_DEFAULT);
        let (tx, mut rx) = again::retry(|| {
            let websocket_config = Arc::clone(&websocket_config);
            connect(websocket_config, addr)
        })
        .await?;

        let event_emitter = EventEmitter::new();
        let mutex = Arc::new(Mutex::new(event_emitter));

        tokio::task::spawn({
            let mutex = mutex.clone();
            async move {
                while let Ok(msg) = rx.recv::<AppResponse>().await {
                    if let holochain_websocket::ReceiveMessage::Signal(signal_bytes) = msg {
                        let mut event_emitter = mutex.lock().await;
                        let signal = Signal::try_from_vec(signal_bytes).expect("Malformed signal");
                        event_emitter.emit("signal", signal);
                    }
                }
            }
        });

        Ok(Self {
            tx,
            event_emitter: mutex,
        })
    }

    pub(crate) async fn on_signal<F: Fn(Signal) + 'static + Sync + Send>(
        &mut self,
        handler: F,
    ) -> Result<String> {
        let mut event_emitter = self.event_emitter.lock().await;
        let id = event_emitter.on("signal", handler);
        Ok(id)
    }

    pub(crate) async fn app_info(&mut self) -> ConductorApiResult<Option<AppInfo>> {
        let response = self.send(AppRequest::AppInfo).await?;
        match response {
            AppResponse::AppInfo(app_info) => Ok(app_info),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub(crate) async fn authenticate(
        &mut self,
        token: AppAuthenticationToken,
    ) -> ConductorApiResult<()> {
        self.tx
            .authenticate(AppAuthenticationRequest { token })
            .await
            .map_err(ConductorApiError::WebsocketError)
    }

    pub(crate) async fn send(&mut self, msg: AppRequest) -> ConductorApiResult<AppResponse> {
        let response = self
            .tx
            .request(msg)
            .await
            .map_err(ConductorApiError::WebsocketError)?;

        match response {
            AppResponse::Error(error) => Err(ConductorApiError::ExternalApiWireError(error)),
            _ => Ok(response),
        }
    }
}
