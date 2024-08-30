use crate::error::{ConductorApiError, ConductorApiResult};
use anyhow::Result;
use holo_hash::DnaHash;
use holochain_conductor_api::{
    AdminRequest, AdminResponse, AppAuthenticationTokenIssued, AppInfo, AppInterfaceInfo,
    AppStatusFilter, CompatibleCells, IssueAppAuthenticationTokenPayload, RevokeAgentKeyPayload,
    StorageInfo,
};
use holochain_types::websocket::AllowedOrigins;
use holochain_types::{
    dna::AgentPubKey,
    prelude::{CellId, DeleteCloneCellPayload, InstallAppPayload, UpdateCoordinatorsPayload},
};
use holochain_websocket::{connect, WebsocketConfig, WebsocketSender};
use holochain_zome_types::{
    capability::GrantedFunctions,
    prelude::{DnaDef, GrantZomeCallCapabilityPayload, Record},
};
use kitsune_p2p_types::agent_info::AgentInfoSigned;
use serde::{Deserialize, Serialize};
use std::{net::ToSocketAddrs, sync::Arc};
use tokio::task::JoinHandle;

pub struct AdminWebsocket {
    tx: WebsocketSender,
    poll_handle: JoinHandle<()>,
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
    /// Connect to a Conductor API AdminWebsocket.
    ///
    /// `socket_addr` is a websocket address that implements `ToSocketAddr`.
    /// See trait [`ToSocketAddr`](https://doc.rust-lang.org/std/net/trait.ToSocketAddrs.html#tymethod.to_socket_addrs).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// use std::net::Ipv4Addr;
    /// let admin_ws = holochain_client::AdminWebsocket::connect((Ipv4Addr::LOCALHOST, 30_000)).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// As string `"localhost:30000"`
    /// As tuple `([127.0.0.1], 30000)`
    pub async fn connect(socket_addr: impl ToSocketAddrs) -> Result<Self> {
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

        // WebsocketReceiver needs to be polled in order to receive responses
        // from remote to sender requests.
        let poll_handle =
            tokio::task::spawn(async move { while rx.recv::<AdminResponse>().await.is_ok() {} });

        Ok(Self { tx, poll_handle })
    }

    /// Issue an app authentication token for the specified app.
    ///
    /// A token is required to create an [AppAgentWebsocket] connection.
    pub async fn issue_app_auth_token(
        &self,
        payload: IssueAppAuthenticationTokenPayload,
    ) -> ConductorApiResult<AppAuthenticationTokenIssued> {
        let response = self
            .send(AdminRequest::IssueAppAuthenticationToken(payload))
            .await?;
        match response {
            AdminResponse::AppAuthenticationTokenIssued(issued) => Ok(issued),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn generate_agent_pub_key(&self) -> ConductorApiResult<AgentPubKey> {
        // Create agent key in Lair and save it in file
        let response = self.send(AdminRequest::GenerateAgentPubKey).await?;
        match response {
            AdminResponse::AgentPubKeyGenerated(key) => Ok(key),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn revoke_agent_key(
        &self,
        app_id: String,
        agent_key: AgentPubKey,
    ) -> ConductorApiResult<Vec<(CellId, String)>> {
        let response = self
            .send(AdminRequest::RevokeAgentKey(Box::new(
                RevokeAgentKeyPayload { app_id, agent_key },
            )))
            .await?;
        match response {
            AdminResponse::AgentKeyRevoked(errors) => Ok(errors),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    /// List all app interfaces attached to the conductor.
    ///
    /// See the documentation for [AdminWebsocket::attach_app_interface] to understand the content
    /// of `AppInterfaceInfo` and help you to select an appropriate interface to connect to.
    pub async fn list_app_interfaces(&self) -> ConductorApiResult<Vec<AppInterfaceInfo>> {
        let msg = AdminRequest::ListAppInterfaces;
        let response = self.send(msg).await?;
        match response {
            AdminResponse::AppInterfacesListed(interfaces) => Ok(interfaces),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    /// Attach an app interface to the conductor.
    ///
    /// This will create a new websocket on the specified port. Alternatively, specify the port as
    /// 0 to allow the OS to choose a port. The selected port will be returned so you know where
    /// to connect your app client.
    ///
    /// Allowed origins can be used to restrict which domains can connect to the interface.
    /// This is used to protect the interface from scripts running in web pages. In development it
    /// is acceptable to use `AllowedOrigins::All` to allow all connections. In production you
    /// should consider setting an explicit list of origins, such as `"my_cli_app".to_string().into()`.
    ///
    /// If you want to restrict this app interface so that it is only accessible to a specific
    /// installed app then you can provide the installed_app_id. The client will still need to
    /// authenticate with a valid token for the same app, but clients for other apps will not be
    /// able to connect. If you want to allow all apps to connect then set this to `None`.
    pub async fn attach_app_interface(
        &self,
        port: u16,
        allowed_origins: AllowedOrigins,
        installed_app_id: Option<String>,
    ) -> ConductorApiResult<u16> {
        let msg = AdminRequest::AttachAppInterface {
            port: Some(port),
            allowed_origins,
            installed_app_id,
        };
        let response = self.send(msg).await?;
        match response {
            AdminResponse::AppInterfaceAttached { port } => Ok(port),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn list_apps(
        &self,
        status_filter: Option<AppStatusFilter>,
    ) -> ConductorApiResult<Vec<AppInfo>> {
        let response = self.send(AdminRequest::ListApps { status_filter }).await?;
        match response {
            AdminResponse::AppsListed(apps_infos) => Ok(apps_infos),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn install_app(&self, payload: InstallAppPayload) -> ConductorApiResult<AppInfo> {
        let msg = AdminRequest::InstallApp(Box::new(payload));
        let response = self.send(msg).await?;

        match response {
            AdminResponse::AppInstalled(app_info) => Ok(app_info),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn uninstall_app(
        &self,
        installed_app_id: String,
        force: bool,
    ) -> ConductorApiResult<()> {
        let msg = AdminRequest::UninstallApp {
            installed_app_id,
            force,
        };
        let response = self.send(msg).await?;

        match response {
            AdminResponse::AppUninstalled => Ok(()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn enable_app(
        &self,
        installed_app_id: String,
    ) -> ConductorApiResult<EnableAppResponse> {
        let msg = AdminRequest::EnableApp { installed_app_id };
        let response = self.send(msg).await?;

        match response {
            AdminResponse::AppEnabled { app, errors } => Ok(EnableAppResponse { app, errors }),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn disable_app(&self, installed_app_id: String) -> ConductorApiResult<()> {
        let msg = AdminRequest::DisableApp { installed_app_id };
        let response = self.send(msg).await?;

        match response {
            AdminResponse::AppDisabled => Ok(()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn get_dna_definition(&self, hash: DnaHash) -> ConductorApiResult<DnaDef> {
        let msg = AdminRequest::GetDnaDefinition(Box::new(hash));
        let response = self.send(msg).await?;
        match response {
            AdminResponse::DnaDefinitionReturned(dna_definition) => Ok(dna_definition),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn get_compatible_cells(
        &self,
        dna_hash: DnaHash,
    ) -> ConductorApiResult<CompatibleCells> {
        let msg = AdminRequest::GetCompatibleCells(dna_hash);
        let response = self.send(msg).await?;
        match response {
            AdminResponse::CompatibleCells(compatible_cells) => Ok(compatible_cells),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn grant_zome_call_capability(
        &self,
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
        &self,
        payload: DeleteCloneCellPayload,
    ) -> ConductorApiResult<()> {
        let msg = AdminRequest::DeleteCloneCell(Box::new(payload));
        let response = self.send(msg).await?;
        match response {
            AdminResponse::CloneCellDeleted => Ok(()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn storage_info(&self) -> ConductorApiResult<StorageInfo> {
        let msg = AdminRequest::StorageInfo;
        let response = self.send(msg).await?;
        match response {
            AdminResponse::StorageInfo(info) => Ok(info),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn dump_network_stats(&self) -> ConductorApiResult<String> {
        let msg = AdminRequest::DumpNetworkStats;
        let response = self.send(msg).await?;
        match response {
            AdminResponse::NetworkStatsDumped(stats) => Ok(stats),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn update_coordinators(
        &self,
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
        &self,
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

    pub async fn agent_info(
        &self,
        cell_id: Option<CellId>,
    ) -> ConductorApiResult<Vec<AgentInfoSigned>> {
        let msg = AdminRequest::AgentInfo { cell_id };
        let response = self.send(msg).await?;
        match response {
            AdminResponse::AgentInfo(agent_info) => Ok(agent_info),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn add_agent_info(
        &self,
        agent_infos: Vec<AgentInfoSigned>,
    ) -> ConductorApiResult<()> {
        let msg = AdminRequest::AddAgentInfo { agent_infos };
        let response = self.send(msg).await?;
        match response {
            AdminResponse::AgentInfoAdded => Ok(()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn authorize_signing_credentials(
        &self,
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

    async fn send(&self, msg: AdminRequest) -> ConductorApiResult<AdminResponse> {
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

impl Drop for AdminWebsocket {
    fn drop(&mut self) {
        self.poll_handle.abort();
    }
}
