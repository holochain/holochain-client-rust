use std::sync::Arc;

use anyhow::{Context, Result};
use holochain_conductor_api::{
    AppInfo, AppRequest, AppResponse, ClonedCell, NetworkInfo, ZomeCall,
};
use holochain_types::prelude::NetworkInfoRequestPayload;
use holochain_websocket::{connect, WebsocketConfig, WebsocketSender};
use holochain_zome_types::ExternIO;
use url::Url;

use crate::error::{ConductorApiError, ConductorApiResult};
use crate::types::{
    AppCreateCloneCellPayload, AppDisableCloneCellPayload, AppEnableCloneCellPayload
};

#[derive(Clone)]
pub struct AppWebsocket {
    tx: WebsocketSender,
    app_id: String,
}

impl AppWebsocket {
    pub async fn connect(
        app_url: String,
        installed_app_id: String,
    ) -> Result<Self> {
        let url = Url::parse(&app_url).context("invalid ws:// URL")?;
        let websocket_config = Arc::new(WebsocketConfig::default());
        let (tx, _rx) = again::retry(|| {
            let websocket_config = Arc::clone(&websocket_config);
            connect(url.clone().into(), websocket_config)
        })
        .await?;
        Ok(Self { tx, app_id: installed_app_id })
    }

    pub async fn app_info(&mut self) -> ConductorApiResult<Option<AppInfo>> {
        let msg = AppRequest::AppInfo {
            installed_app_id: self.app_id.clone(),
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
            .map_err(|err| ConductorApiError::WebsocketError(err))?;

        match response {
            AppResponse::Error(error) => Err(ConductorApiError::ExternalApiWireError(error)),
            _ => Ok(response),
        }
    }
}
