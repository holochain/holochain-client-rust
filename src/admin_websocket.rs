use std::sync::Arc;

use anyhow::{Context, Result};
use holochain_conductor_api::{AdminRequest, AdminResponse, AppStatusFilter, InstalledAppInfo};
use holochain_types::{
    dna::AgentPubKey,
    prelude::{CellId, DeleteCloneCellPayload, InstallAppPayload},
};
use holochain_websocket::{connect, WebsocketConfig, WebsocketReceiver, WebsocketSender};
use holochain_zome_types::GrantZomeCallCapabilityPayload;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{ConductorApiError, ConductorApiResult};

pub struct AdminWebsocket {
    tx: WebsocketSender,
    rx: WebsocketReceiver,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnableAppResponse {
    pub app: InstalledAppInfo,
    pub errors: Vec<(CellId, String)>,
}

impl AdminWebsocket {
    pub async fn connect(admin_url: String) -> Result<Self> {
        let url = Url::parse(&admin_url).context("invalid ws:// URL")?;
        let websocket_config = Arc::new(WebsocketConfig::default());
        let (tx, rx) = again::retry(|| {
            let websocket_config = Arc::clone(&websocket_config);
            connect(url.clone().into(), websocket_config)
        })
        .await?;

        Ok(Self { tx, rx })
    }

    pub fn close(&mut self) -> () {
        match self.rx.take_handle() {
            Some(h) => h.close(),
            None => (),
        }
    }

    pub async fn generate_agent_pub_key(&mut self) -> ConductorApiResult<AgentPubKey> {
        // Create agent key in Lair and save it in file
        let response = self.send(AdminRequest::GenerateAgentPubKey).await?;
        match response {
            AdminResponse::AgentPubKeyGenerated(key) => Ok(key),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn list_app_interfaces(&mut self) -> ConductorApiResult<Vec<u16>> {
        let msg = AdminRequest::ListAppInterfaces;
        let response = self.send(msg).await?;
        match response {
            AdminResponse::AppInterfacesListed(ports) => Ok(ports),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn attach_app_interface(&mut self, port: u16) -> ConductorApiResult<u16> {
        let msg = AdminRequest::AttachAppInterface { port: Some(port) };
        let response = self.send(msg).await?;
        match response {
            AdminResponse::AppInterfaceAttached { port } => Ok(port),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn list_apps(
        &mut self,
        status_filter: Option<AppStatusFilter>,
    ) -> ConductorApiResult<Vec<InstalledAppInfo>> {
        let response = self.send(AdminRequest::ListApps { status_filter }).await?;
        match response {
            AdminResponse::AppsListed(apps_infos) => Ok(apps_infos),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn install_app(
        &mut self,
        payload: InstallAppPayload,
    ) -> ConductorApiResult<InstalledAppInfo> {
        let msg = AdminRequest::InstallApp(Box::new(payload));
        let response = self.send(msg).await?;

        match response {
            AdminResponse::AppInstalled(app_info) => Ok(app_info),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn uninstall_app(&mut self, installed_app_id: String) -> ConductorApiResult<()> {
        let msg = AdminRequest::UninstallApp { installed_app_id };
        let response = self.send(msg).await?;

        match response {
            AdminResponse::AppUninstalled => Ok(()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn enable_app(
        &mut self,
        installed_app_id: String,
    ) -> ConductorApiResult<EnableAppResponse> {
        let msg = AdminRequest::EnableApp { installed_app_id };
        let response = self.send(msg).await?;

        match response {
            AdminResponse::AppEnabled { app, errors } => Ok(EnableAppResponse { app, errors }),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn disable_app(&mut self, installed_app_id: String) -> ConductorApiResult<()> {
        let msg = AdminRequest::DisableApp { installed_app_id };
        let response = self.send(msg).await?;

        match response {
            AdminResponse::AppDisabled => Ok(()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn start_app(&mut self, installed_app_id: String) -> ConductorApiResult<bool> {
        let msg = AdminRequest::StartApp { installed_app_id };
        let response = self.send(msg).await?;

        match response {
            AdminResponse::AppStarted(started) => Ok(started),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn grant_zome_call_capability(
        &mut self,
        payload: GrantZomeCallCapabilityPayload,
    ) -> ConductorApiResult<()> {
        let msg = AdminRequest::GrantZomeCallCapability(Box::new(payload));
        let response = self.send(msg).await?;

        match response {
            AdminResponse::ZomeCallCapabilityGranted => Ok(()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn delete_clone_cell(
        &mut self,
        payload: DeleteCloneCellPayload,
    ) -> ConductorApiResult<()> {
        let msg = AdminRequest::DeleteCloneCell(Box::new(payload));
        let response = self.send(msg).await?;
        match response {
            AdminResponse::CloneCellDeleted => Ok(()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    async fn send(&mut self, msg: AdminRequest) -> ConductorApiResult<AdminResponse> {
        let response: AdminResponse = self
            .tx
            .request(msg)
            .await
            .map_err(|err| ConductorApiError::WebsocketError(err))?;
        match response {
            AdminResponse::Error(error) => Err(ConductorApiError::ExternalApiWireError(error)),
            _ => Ok(response),
        }
    }
}
