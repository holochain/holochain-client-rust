use std::sync::Arc;

use anyhow::{Context, Result};
use event_emitter_rs::EventEmitter;
use futures::lock::Mutex;
use futures::stream::StreamExt;
use url::Url;

use holochain_conductor_api::{
    AppInfo, AppRequest, AppResponse, ClonedCell, NetworkInfo, ZomeCall,
};
use holochain_types::{
    app::InstalledAppId,
    prelude::{
        CreateCloneCellPayload, DisableCloneCellPayload, EnableCloneCellPayload, ExternIO,
        NetworkInfoRequestPayload,
    },
    signal::Signal,
};
use holochain_websocket::{connect, Respond, WebsocketConfig, WebsocketSender};

use crate::error::{ConductorApiError, ConductorApiResult};

#[derive(Clone)]
pub struct AppWebsocket {
    tx: WebsocketSender,
    event_emitter_mutex: Arc<Mutex<EventEmitter>>,
}

impl AppWebsocket {
    pub async fn connect(app_url: String) -> Result<Self> {
        let url = Url::parse(&app_url).context("invalid ws:// URL")?;
        let websocket_config = Arc::new(WebsocketConfig::default());
        let (tx, mut rx) = again::retry(|| {
            let websocket_config = Arc::clone(&websocket_config);
            connect(url.clone().into(), websocket_config)
        })
        .await?;

        let event_emitter = EventEmitter::new();
        let mutex = Arc::new(Mutex::new(event_emitter));

        let m = mutex.clone();

        std::thread::spawn(move || {
            futures::executor::block_on(async {
                while let Some((msg, resp)) = rx.next().await {
                    if let Respond::Signal = resp {
                        let mut ee = m.lock().await;
                        let signal = Signal::try_from(msg).expect("Malformed signal");
                        ee.emit("signal", signal);
                    }
                }
            });
        });

        Ok(Self {
            tx,
            event_emitter_mutex: mutex,
        })
    }

    pub async fn on_signal<F: Fn(Signal) + 'static + Sync + Send>(
        &mut self,
        handler: F,
    ) -> Result<String> {
        let mut ee = self.event_emitter_mutex.lock().await;
        let id = ee.on("signal", handler);
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
