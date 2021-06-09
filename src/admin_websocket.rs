use std::sync::Arc;

use anyhow::{Context, Result};
use holochain_conductor_api::{AdminRequest, AdminResponse, InstalledAppInfo};
use holochain_types::{
    app::{InstallAppBundlePayload, InstalledAppId},
    dna::AgentPubKey,
};
use holochain_websocket::{connect, WebsocketConfig, WebsocketSender};
use url::Url;

use crate::error::{ConductorApiError, ConductorApiResult};

#[derive(Clone)]
pub struct AdminWebsocket {
    tx: WebsocketSender,
}

impl AdminWebsocket {
    pub async fn connect(admin_url: String) -> Result<Self> {
        let url = Url::parse(&admin_url).context("invalid ws:// URL")?;
        let websocket_config = Arc::new(WebsocketConfig::default());
        let (tx, _rx) = again::retry(|| {
            let websocket_config = Arc::clone(&websocket_config);
            connect(url.clone().into(), websocket_config)
        })
        .await?;
        Ok(Self { tx })
    }

    pub async fn generate_agent_pub_key(&mut self) -> ConductorApiResult<AgentPubKey> {
        // Create agent key in Lair and save it in file
        let response = self.send(AdminRequest::GenerateAgentPubKey).await?;
        match response {
            AdminResponse::AgentPubKeyGenerated(key) => Ok(key),
            _ => unreachable!(format!("Unexpected response {:?}", response)),
        }
    }

    pub async fn list_app_interfaces(&mut self) -> ConductorApiResult<Vec<u16>> {
        let msg = AdminRequest::ListAppInterfaces;
        let response = self.send(msg).await?;
        match response {
            AdminResponse::AppInterfacesListed(ports) => Ok(ports),
            _ => unreachable!(format!("Unexpected response {:?}", response)),
        }
    }

    pub async fn attach_app_interface(&mut self, port: u16) -> ConductorApiResult<u16> {
        let msg = AdminRequest::AttachAppInterface { port: Some(port) };
        let response = self.send(msg).await?;
        match response {
            AdminResponse::AppInterfaceAttached { port } => Ok(port),
            _ => unreachable!(format!("Unexpected response {:?}", response)),
        }
    }

    pub async fn list_active_apps(&mut self) -> ConductorApiResult<Vec<InstalledAppId>> {
        let response = self.send(AdminRequest::ListActiveApps).await?;
        match response {
            AdminResponse::ActiveAppsListed(app_ids) => Ok(app_ids),
            _ => unreachable!(format!("Unexpected response {:?}", response)),
        }
    }

    pub async fn install_app_bundle(
        &mut self,
        payload: InstallAppBundlePayload,
    ) -> ConductorApiResult<InstalledAppInfo> {
        let msg = AdminRequest::InstallAppBundle(Box::new(payload));
        let response = self.send(msg).await?;

        match response {
            AdminResponse::AppBundleInstalled(app_info) => Ok(app_info),
            _ => unreachable!(format!("Unexpected response {:?}", response)),
        }
    }

    pub async fn activate_app(&mut self, installed_app_id: String) -> ConductorApiResult<()> {
        let msg = AdminRequest::ActivateApp { installed_app_id };
        let response = self.send(msg).await?;

        match response {
            AdminResponse::AppActivated => Ok(()),
            _ => unreachable!(format!("Unexpected response {:?}", response)),
        }
    }

    pub async fn deactivate_app(&mut self, installed_app_id: String) -> ConductorApiResult<()> {
        let msg = AdminRequest::DeactivateApp { installed_app_id };
        let response = self.send(msg).await?;

        match response {
            AdminResponse::AppDeactivated => Ok(()),
            _ => unreachable!(format!("Unexpected response {:?}", response)),
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
