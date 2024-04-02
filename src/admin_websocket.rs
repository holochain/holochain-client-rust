use crate::error::{ConductorApiError, ConductorApiResult};
use anyhow::Result;
use holo_hash::DnaHash;
use holochain_conductor_api::{AdminRequest, AdminResponse, AppInfo, AppStatusFilter, StorageInfo};
use holochain_types::{
    dna::AgentPubKey,
    prelude::{CellId, DeleteCloneCellPayload, InstallAppPayload, UpdateCoordinatorsPayload},
};
use holochain_websocket::{connect, WebsocketConfig, WebsocketSender};
use holochain_zome_types::{
    capability::GrantedFunctions,
    prelude::{DnaDef, GrantZomeCallCapabilityPayload, Record},
};
use serde::{Deserialize, Serialize};
use std::{net::ToSocketAddrs, sync::Arc};
use url::Url;

pub struct AdminWebsocket {
    tx: WebsocketSender,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnableAppResponse {
    pub app: AppInfo,
    pub errors: Vec<(CellId, String)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthorizeSigningCredentialsPayload {
    pub cell_id: CellId,
    pub functions: Option<GrantedFunctions>,
}

impl AdminWebsocket {
    pub async fn connect(admin_url: String) -> Result<Self> {
        let url = Url::parse(&admin_url)?;
        let host = url
            .host_str()
            .expect("websocket url does not have valid host part");
        let port = url.port().expect("websocket url does not have valid port");
        let admin_addr = format!("{}:{}", host, port);
        let addr = admin_addr
            .to_socket_addrs()?
            .find(|addr| addr.is_ipv4())
            .expect("no valid ipv4 websocket addresses found");

        // app installation takes > 2 min on CI at the moment, hence the high
        // request timeout
        let websocket_config = WebsocketConfig {
            default_request_timeout: std::time::Duration::from_secs(180),
            ..Default::default()
        };
        let websocket_config = Arc::new(websocket_config);

        let (tx, mut rx) = again::retry(|| {
            let websocket_config = Arc::clone(&websocket_config);
            connect(websocket_config, addr)
        })
        .await?;

        // WebsocketReceiver needs to be polled in order to receive responses
        // from remote to sender requests.
        tokio::task::spawn(async move { while rx.recv::<AdminResponse>().await.is_ok() {} });

        Ok(Self { tx })
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
    ) -> ConductorApiResult<Vec<AppInfo>> {
        let response = self.send(AdminRequest::ListApps { status_filter }).await?;
        match response {
            AdminResponse::AppsListed(apps_infos) => Ok(apps_infos),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn install_app(&mut self, payload: InstallAppPayload) -> ConductorApiResult<AppInfo> {
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

    pub async fn get_dna_definition(&mut self, hash: DnaHash) -> ConductorApiResult<DnaDef> {
        let msg = AdminRequest::GetDnaDefinition(Box::new(hash));
        let response = self.send(msg).await?;
        match response {
            AdminResponse::DnaDefinitionReturned(dna_definition) => Ok(dna_definition),
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

    pub async fn storage_info(&mut self) -> ConductorApiResult<StorageInfo> {
        let msg = AdminRequest::StorageInfo;
        let response = self.send(msg).await?;
        match response {
            AdminResponse::StorageInfo(info) => Ok(info),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn dump_network_stats(&mut self) -> ConductorApiResult<String> {
        let msg = AdminRequest::DumpNetworkStats;
        let response = self.send(msg).await?;
        match response {
            AdminResponse::NetworkStatsDumped(stats) => Ok(stats),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn update_coordinators(
        &mut self,
        update_coordinators_payload: UpdateCoordinatorsPayload,
    ) -> ConductorApiResult<()> {
        let msg = AdminRequest::UpdateCoordinators(Box::new(update_coordinators_payload));
        let response = self.send(msg).await?;
        match response {
            AdminResponse::CoordinatorsUpdated => Ok(()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn graft_records(
        &mut self,
        cell_id: CellId,
        validate: bool,
        records: Vec<Record>,
    ) -> ConductorApiResult<()> {
        let msg = AdminRequest::GraftRecords {
            cell_id,
            validate,
            records,
        };
        let response = self.send(msg).await?;
        match response {
            AdminResponse::RecordsGrafted => Ok(()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn authorize_signing_credentials(
        &mut self,
        request: AuthorizeSigningCredentialsPayload,
    ) -> Result<crate::signing::client_signing::SigningCredentials> {
        use holochain_zome_types::capability::{ZomeCallCapGrant, CAP_SECRET_BYTES};
        use rand::{rngs::OsRng, RngCore};
        use std::collections::BTreeSet;

        let mut csprng = OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut csprng);
        let public_key = keypair.verifying_key();
        let signing_agent_key = AgentPubKey::from_raw_32(public_key.as_bytes().to_vec());

        let mut cap_secret = [0; CAP_SECRET_BYTES];
        csprng.fill_bytes(&mut cap_secret);

        self.grant_zome_call_capability(GrantZomeCallCapabilityPayload {
            cell_id: request.cell_id,
            cap_grant: ZomeCallCapGrant {
                tag: "zome-call-signing-key".to_string(),
                access: holochain_zome_types::capability::CapAccess::Assigned {
                    secret: cap_secret.into(),
                    assignees: BTreeSet::from([signing_agent_key.clone()]),
                },
                functions: request.functions.unwrap_or(GrantedFunctions::All),
            },
        })
        .await
        .map_err(|e| anyhow::anyhow!("Conductor API error: {:?}", e))?;

        Ok(crate::signing::client_signing::SigningCredentials {
            signing_agent_key,
            keypair,
            cap_secret: cap_secret.into(),
        })
    }

    async fn send(&mut self, msg: AdminRequest) -> ConductorApiResult<AdminResponse> {
        let response: AdminResponse = self
            .tx
            .request(msg)
            .await
            .map_err(ConductorApiError::WebsocketError)?;
        match response {
            AdminResponse::Error(error) => Err(ConductorApiError::ExternalApiWireError(error)),
            _ => Ok(response),
        }
    }
}
