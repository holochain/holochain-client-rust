use std::sync::Arc;

use anyhow::{Context, Result};
use holochain_conductor_api::{AppRequest, AppResponse, InstalledAppInfo, ZomeCall};
use holochain_types::{app::InstalledAppId, prelude::ExternIO};
use holochain_websocket::{connect, WebsocketConfig, WebsocketSender};
use url::Url;

use crate::error::{ConductorApiError, ConductorApiResult};

#[derive(Clone)]
pub struct AppWebsocket {
    tx: WebsocketSender,
}

impl AppWebsocket {
    pub async fn connect(app_url: String) -> Result<Self> {
        let url = Url::parse(&app_url).context("invalid ws:// URL")?;
        let websocket_config = Arc::new(WebsocketConfig::default());
        let (tx, _rx) = again::retry(|| {
            let websocket_config = Arc::clone(&websocket_config);
            connect(url.clone().into(), websocket_config)
        })
        .await?;
        Ok(Self { tx })
    }

    pub async fn app_info(
        &mut self,
        app_id: InstalledAppId,
    ) -> ConductorApiResult<Option<InstalledAppInfo>> {
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
        let app_request = AppRequest::ZomeCall(Box::new(msg));
        let response = self.send(app_request).await?;

        match response {
            AppResponse::ZomeCall(result) => Ok(*result),
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
