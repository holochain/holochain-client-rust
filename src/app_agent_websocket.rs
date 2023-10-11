use std::sync::Arc;
use anyhow::{Context, Result, anyhow};
use holochain_conductor_api::{
    AppInfo, AppRequest, AppResponse, ClonedCell, NetworkInfo
};
use holochain_types::{
    app::InstalledAppId,
    prelude::ExternIO,
};
use holochain_websocket::{connect, WebsocketConfig, WebsocketSender};
use url::Url;
use holochain_types::prelude::AgentPubKey;

use crate::{error::{ConductorApiError, ConductorApiResult}, AppAgentNetworkInfoRequestPayload, AppCreateCloneCellPayload, AppEnableCloneCellPayload, AppDisableCloneCellPayload, AppAgentZomeCall};

#[derive(Clone)]
pub struct AppAgentWebsocket {
    tx: WebsocketSender,
    app_id: String,
    my_pub_key: AgentPubKey,
}

impl From<ConductorApiError> for anyhow::Error {
    fn from(value: ConductorApiError) -> Self {
       anyhow!(format!("{:?}", value))
    }
}
impl AppAgentWebsocket {
    pub async fn connect(
        app_url: String,
        installed_app_id: InstalledAppId,
    ) -> Result<Self> {
        let url = Url::parse(&app_url).context("invalid ws:// URL")?;
        let websocket_config = Arc::new(WebsocketConfig::default());
        let (tx, _rx) = again::retry(|| {
            let websocket_config = Arc::clone(&websocket_config);
            connect(url.clone().into(), websocket_config)
        })
        .await
        .map_err(|e| ConductorApiError::WebsocketError(e))?;

        let app_info = AppAgentWebsocket::app_info_inner(&self, &tx, installed_app_id.clone()).await?.unwrap();

        Ok(Self { tx, app_id: installed_app_id, my_pub_key: app_info.agent_pub_key.clone() })
    }

    pub async fn app_info(&mut self) -> ConductorApiResult<Option<AppInfo>> {
        self.app_info_inner(&self.tx, self.app_id.clone()).await
    }

    async fn app_info_inner(
        &self,
        tx: &WebsocketSender, 
        installed_app_id: String
    ) -> ConductorApiResult<Option<AppInfo>> {
        let msg = AppRequest::AppInfo {
            installed_app_id
        };
        let response = AppAgentWebsocket::send_inner(tx.clone(), msg.clone()).await?;
        match response {
            AppResponse::AppInfo(app_info) => Ok(app_info.clone()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn call_zome(
        &mut self, 
        msg: AppAgentZomeCall
    ) -> ConductorApiResult<ExternIO> {
        let app_request = AppRequest::CallZome(Box::new(msg.into_zome_call(self.my_pub_key.clone())));
        let response = self.send(app_request).await?;

        match response {
            AppResponse::ZomeCalled(result) => Ok(*result),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn create_clone_cell(
        &mut self,
        msg: AppCreateCloneCellPayload,
    ) -> ConductorApiResult<ClonedCell> {
        let app_request = AppRequest::CreateCloneCell(Box::new(msg.into_create_clone_cell_payload(self.app_id.clone())));
        let response = self.send(app_request).await?;
        match response {
            AppResponse::CloneCellCreated(clone_cell) => Ok(clone_cell),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn enable_clone_cell(
        &mut self,
        payload: AppEnableCloneCellPayload,
    ) -> ConductorApiResult<ClonedCell> {
        let msg = AppRequest::EnableCloneCell(Box::new(payload.into_enable_clone_cell_payload(self.app_id.clone())));
        let response = self.send(msg).await?;
        match response {
            AppResponse::CloneCellEnabled(enabled_cell) => Ok(enabled_cell),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn disable_clone_cell(
        &mut self,
        payload: AppDisableCloneCellPayload,
    ) -> ConductorApiResult<()> {
        let app_request = AppRequest::DisableCloneCell(Box::new(payload.into_disable_clone_cell_payload(self.app_id.clone())));
        let response = self.send(app_request).await?;
        match response {
            AppResponse::CloneCellDisabled => Ok(()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn network_info(
        &mut self,
        payload: AppAgentNetworkInfoRequestPayload,
    ) -> ConductorApiResult<Vec<NetworkInfo>> {
        let msg = AppRequest::NetworkInfo(Box::new(payload.into_network_info_request_payload(self.my_pub_key.clone())));
        let response = self.send(msg).await?;
        match response {
            AppResponse::NetworkInfo(infos) => Ok(infos),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    async fn send(&mut self, msg: AppRequest) -> ConductorApiResult<AppResponse> {
        AppAgentWebsocket::send_inner(self.tx.clone(), msg).await
    }

    async fn send_inner(mut tx: WebsocketSender, msg: AppRequest) -> ConductorApiResult<AppResponse> {
        let response = tx
            .request(msg)
            .await
            .map_err(|err| ConductorApiError::WebsocketError(err))?;

        match response {
            AppResponse::Error(error) => Err(ConductorApiError::ExternalApiWireError(error)),
            _ => Ok(response),
        }
    }
}
