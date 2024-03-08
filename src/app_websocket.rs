use crate::error::{ConductorApiError, ConductorApiResult};
use anyhow::Result;
use holochain_conductor_api::{
    AppInfo, AppRequest, AppResponse, ClonedCell, NetworkInfo, ZomeCall,
};
use holochain_types::{
    app::InstalledAppId,
    prelude::{
        CreateCloneCellPayload, DisableCloneCellPayload, EnableCloneCellPayload, ExternIO,
        NetworkInfoRequestPayload,
    },
};
use holochain_websocket::{connect, WebsocketConfig, WebsocketSender};
use std::{net::ToSocketAddrs, sync::Arc};
use url::Url;

#[derive(Clone)]
pub struct AppWebsocket {
    tx: WebsocketSender,
}

impl AppWebsocket {
    pub async fn connect(app_url: String) -> Result<Self> {
        let url = Url::parse(&app_url)?;
        let host = url
            .host_str()
            .expect("websocket url does not have valid host part");
        let port = url.port().expect("websocket url does not have valid port");
        println!("port is {port}");
        let app_addr = format!("{}:{}", host, port);
        let addr = app_addr
            .to_socket_addrs()?
            .find(|addr| addr.is_ipv4())
            .expect("no valid ipv4 websocket addresses found");
        println!("addr {addr:?}");

        let websocket_config = Arc::new(WebsocketConfig::default());
        let (tx, mut rx) = again::retry(|| {
            let websocket_config = Arc::clone(&websocket_config);
            connect(websocket_config, addr)
        })
        .await?;

        // WebsocketReceiver needs to be polled in order to receive responses
        // from remote to sender requests.
        tokio::task::spawn(async move { while rx.recv::<AppResponse>().await.is_ok() {} });

        Ok(Self { tx })
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
