use crate::error::{ConductorApiError, ConductorApiResult};
use anyhow::Result;
use event_emitter_rs::EventEmitter;
use holochain_conductor_api::{AppInfo, AppRequest, AppResponse, NetworkInfo, ZomeCall};
use holochain_types::{
    app::InstalledAppId,
    prelude::{
        CreateCloneCellPayload, DisableCloneCellPayload, EnableCloneCellPayload, ExternIO,
        NetworkInfoRequestPayload,
    },
    signal::Signal,
};
use holochain_websocket::{connect, WebsocketConfig, WebsocketSender};
use holochain_zome_types::clone::ClonedCell;
use std::{net::ToSocketAddrs, sync::Arc};
use tokio::sync::Mutex;
use url::Url;

#[derive(Clone)]
pub struct AppWebsocket {
    tx: WebsocketSender,
    event_emitter: Arc<Mutex<EventEmitter>>,
}

impl AppWebsocket {
    pub async fn connect(app_url: String) -> Result<Self> {
        let url = Url::parse(&app_url)?;
        let host = url
            .host_str()
            .expect("websocket url does not have valid host part");
        let port = url.port().expect("websocket url does not have valid port");
        let app_addr = format!("{}:{}", host, port);
        let addr = app_addr
            .to_socket_addrs()?
            .next()
            .expect("Failed to resolve localhost");

        let websocket_config = Arc::new(WebsocketConfig::default());
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

    pub async fn on_signal<F: Fn(Signal) + 'static + Sync + Send>(
        &mut self,
        handler: F,
    ) -> Result<String> {
        let mut event_emitter = self.event_emitter.lock().await;
        let id = event_emitter.on("signal", handler);
        Ok(id)
    }

    pub async fn app_info(
        &mut self,
        app_id: InstalledAppId,
    ) -> ConductorApiResult<Option<AppInfo>> {
        let msg = AppRequest::AppInfo {
            installed_app_id: app_id,
        };
        let response = self.send(msg).await?;
        match response {
            AppResponse::AppInfo(app_info) => Ok(app_info),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn call_zome(&mut self, msg: ZomeCall) -> ConductorApiResult<ExternIO> {
        let app_request = AppRequest::CallZome(Box::new(msg));
        let response = self.send(app_request).await?;

        match response {
            AppResponse::ZomeCalled(result) => Ok(*result),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn create_clone_cell(
        &mut self,
        msg: CreateCloneCellPayload,
    ) -> ConductorApiResult<ClonedCell> {
        let app_request = AppRequest::CreateCloneCell(Box::new(msg));
        let response = self.send(app_request).await?;
        match response {
            AppResponse::CloneCellCreated(clone_cell) => Ok(clone_cell),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn enable_clone_cell(
        &mut self,
        payload: EnableCloneCellPayload,
    ) -> ConductorApiResult<ClonedCell> {
        let msg = AppRequest::EnableCloneCell(Box::new(payload));
        let response = self.send(msg).await?;
        match response {
            AppResponse::CloneCellEnabled(enabled_cell) => Ok(enabled_cell),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn disable_clone_cell(
        &mut self,
        payload: DisableCloneCellPayload,
    ) -> ConductorApiResult<()> {
        let app_request = AppRequest::DisableCloneCell(Box::new(payload));
        let response = self.send(app_request).await?;
        match response {
            AppResponse::CloneCellDisabled => Ok(()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn network_info(
        &mut self,
        payload: NetworkInfoRequestPayload,
    ) -> ConductorApiResult<Vec<NetworkInfo>> {
        let msg = AppRequest::NetworkInfo(Box::new(payload));
        let response = self.send(msg).await?;
        match response {
            AppResponse::NetworkInfo(infos) => Ok(infos),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    async fn send(&mut self, msg: AppRequest) -> ConductorApiResult<AppResponse> {
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
