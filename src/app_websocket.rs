use crate::app_websocket_inner::AppWebsocketInner;
use crate::{
    signing::{sign_zome_call, AgentSigner},
    ConductorApiError, ConductorApiResult,
};
use anyhow::{anyhow, Result};
use holo_hash::AgentPubKey;
use holochain_conductor_api::{
    AppAuthenticationToken, AppInfo, AppRequest, AppResponse, CellInfo, NetworkInfo,
    ProvisionedCell, ZomeCall,
};
use holochain_nonce::fresh_nonce;
use holochain_types::app::{
    CreateCloneCellPayload, DisableCloneCellPayload, EnableCloneCellPayload, MemproofMap,
    NetworkInfoRequestPayload,
};
use holochain_types::prelude::{CloneId, Signal};
use holochain_zome_types::{
    clone::ClonedCell,
    prelude::{CellId, ExternIO, FunctionName, RoleName, Timestamp, ZomeCallUnsigned, ZomeName},
};
use std::net::ToSocketAddrs;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppWebsocket {
    pub my_pub_key: AgentPubKey,
    inner: AppWebsocketInner,
    app_info: AppInfo,
    signer: Arc<dyn AgentSigner + Send + Sync>,
}

impl AppWebsocket {
    /// Connect to a Conductor API AppWebsocket with a specific app id.
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
    /// let mut admin_ws = holochain_client::AdminWebsocket::connect((Ipv4Addr::LOCALHOST, 30_000)).await?;
    ///
    /// let app_id = "test-app".to_string();
    /// let issued = admin_ws.issue_app_auth_token(app_id.clone().into()).await.unwrap();
    /// let signer = holochain_client::ClientAgentSigner::default();
    /// let app_ws = holochain_client::AppWebsocket::connect((Ipv4Addr::LOCALHOST, 30_001), issued.token, signer.into()).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// As string `"localhost:30000"`
    /// As tuple `([127.0.0.1], 30000)`
    pub async fn connect(
        socket_addr: impl ToSocketAddrs,
        token: AppAuthenticationToken,
        signer: Arc<dyn AgentSigner + Send + Sync>,
    ) -> Result<Self> {
        let app_ws = AppWebsocketInner::connect(socket_addr).await?;

        app_ws
            .authenticate(token)
            .await
            .map_err(|err| anyhow!("Failed to send authentication: {err:?}"))?;

        let app_info = app_ws
            .app_info()
            .await
            .map_err(|err| anyhow!("Error fetching app_info {err:?}"))?
            .ok_or(anyhow!("App doesn't exist"))?;

        Ok(AppWebsocket {
            my_pub_key: app_info.agent_pub_key.clone(),
            inner: app_ws,
            app_info,
            signer,
        })
    }

    pub async fn on_signal<F: Fn(Signal) + 'static + Sync + Send>(
        &self,
        handler: F,
    ) -> Result<String> {
        let app_info = self.app_info.clone();
        self.inner
            .on_signal(move |signal| {
                if let Signal::App {
                    cell_id,
                    zome_name: _,
                    signal: _,
                } = signal.clone()
                {
                    if app_info.cell_info.values().any(|cells| {
                        cells.iter().any(|cell_info| match cell_info {
                            CellInfo::Provisioned(cell) => cell.cell_id.eq(&cell_id),
                            CellInfo::Cloned(cell) => cell.cell_id.eq(&cell_id),
                            _ => false,
                        })
                    }) {
                        handler(signal);
                    }
                }
            })
            .await
    }

    pub async fn app_info(&self) -> ConductorApiResult<Option<AppInfo>> {
        self.inner.app_info().await
    }

    pub async fn call_zome(
        &self,
        target: ZomeCallTarget,
        zome_name: ZomeName,
        fn_name: FunctionName,
        payload: ExternIO,
    ) -> ConductorApiResult<ExternIO> {
        let cell_id = match target {
            ZomeCallTarget::CellId(cell_id) => cell_id,
            ZomeCallTarget::RoleName(role_name) => self.get_cell_id_from_role_name(&role_name)?,
            ZomeCallTarget::CloneId(clone_id) => self.get_cell_id_from_role_name(&clone_id.0)?,
        };

        let (nonce, expires_at) =
            fresh_nonce(Timestamp::now()).map_err(ConductorApiError::FreshNonceError)?;

        let zome_call_unsigned = ZomeCallUnsigned {
            provenance: self.signer.get_provenance(&cell_id).ok_or(
                ConductorApiError::SignZomeCallError("Provenance not found".to_string()),
            )?,
            cap_secret: self.signer.get_cap_secret(&cell_id),
            cell_id: cell_id.clone(),
            zome_name,
            fn_name,
            payload,
            expires_at,
            nonce,
        };

        let signed_zome_call = sign_zome_call(zome_call_unsigned, self.signer.clone())
            .await
            .map_err(|e| ConductorApiError::SignZomeCallError(e.to_string()))?;

        self.signed_call_zome(signed_zome_call).await
    }

    pub async fn signed_call_zome(&self, call: ZomeCall) -> ConductorApiResult<ExternIO> {
        let app_request = AppRequest::CallZome(Box::new(call));
        let response = self.inner.send(app_request).await?;

        match response {
            AppResponse::ZomeCalled(result) => Ok(*result),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn provide_memproofs(&self, memproofs: MemproofMap) -> ConductorApiResult<()> {
        let app_request = AppRequest::ProvideMemproofs(memproofs);
        let response = self.inner.send(app_request).await?;
        match response {
            AppResponse::Ok => Ok(()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn create_clone_cell(
        &self,
        msg: CreateCloneCellPayload,
    ) -> ConductorApiResult<ClonedCell> {
        let app_request = AppRequest::CreateCloneCell(Box::new(msg));
        let response = self.inner.send(app_request).await?;
        match response {
            AppResponse::CloneCellCreated(clone_cell) => Ok(clone_cell),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn disable_clone_cell(
        &self,
        payload: DisableCloneCellPayload,
    ) -> ConductorApiResult<()> {
        let app_request = AppRequest::DisableCloneCell(Box::new(payload));
        let response = self.inner.send(app_request).await?;
        match response {
            AppResponse::CloneCellDisabled => Ok(()),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn enable_clone_cell(
        &self,
        payload: EnableCloneCellPayload,
    ) -> ConductorApiResult<ClonedCell> {
        let msg = AppRequest::EnableCloneCell(Box::new(payload));
        let response = self.inner.send(msg).await?;
        match response {
            AppResponse::CloneCellEnabled(enabled_cell) => Ok(enabled_cell),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn network_info(
        &self,
        payload: NetworkInfoRequestPayload,
    ) -> ConductorApiResult<Vec<NetworkInfo>> {
        let msg = AppRequest::NetworkInfo(Box::new(payload));
        let response = self.inner.send(msg).await?;
        match response {
            AppResponse::NetworkInfo(infos) => Ok(infos),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    pub async fn list_wasm_host_functions(&self) -> ConductorApiResult<Vec<String>> {
        let msg = AppRequest::ListWasmHostFunctions;
        let response = self.inner.send(msg).await?;
        match response {
            AppResponse::ListWasmHostFunctions(functions) => Ok(functions),
            _ => unreachable!("Unexpected response {:?}", response),
        }
    }

    /// Gets a new copy of the [AppInfo] for the app this agent is connected to.
    ///
    /// This is useful if you have made changes to the app, such as creating new clone cells, and need to refresh the app info.
    pub async fn refresh_app_info(&mut self) -> Result<()> {
        self.app_info = self
            .app_info()
            .await
            .map_err(|err| anyhow!("Error fetching app_info {err:?}"))?
            .ok_or(anyhow!("App doesn't exist"))?;

        Ok(())
    }

    fn get_cell_id_from_role_name(&self, role_name: &RoleName) -> ConductorApiResult<CellId> {
        if is_clone_id(role_name) {
            let base_role_name = get_base_role_name_from_clone_id(role_name);

            let Some(role_cells) = self.app_info.cell_info.get(&base_role_name) else {
                return Err(ConductorApiError::CellNotFound);
            };

            let maybe_clone_cell: Option<ClonedCell> =
                role_cells.iter().find_map(|cell| match cell {
                    CellInfo::Cloned(cloned_cell) => {
                        if cloned_cell.clone_id.0.eq(role_name) {
                            Some(cloned_cell.clone())
                        } else {
                            None
                        }
                    }
                    _ => None,
                });

            let clone_cell = maybe_clone_cell.ok_or(ConductorApiError::CellNotFound)?;
            Ok(clone_cell.cell_id)
        } else {
            let Some(role_cells) = self.app_info.cell_info.get(role_name) else {
                return Err(ConductorApiError::CellNotFound);
            };

            let maybe_provisioned: Option<ProvisionedCell> =
                role_cells.iter().find_map(|cell| match cell {
                    CellInfo::Provisioned(provisioned_cell) => Some(provisioned_cell.clone()),
                    _ => None,
                });

            let provisioned_cell = maybe_provisioned.ok_or(ConductorApiError::CellNotFound)?;
            Ok(provisioned_cell.cell_id)
        }
    }
}

pub enum ZomeCallTarget {
    CellId(CellId),
    /// Call a cell by its role name.
    ///
    /// Note that when using clone cells, if you create them after creating the [AppWebsocket], you will need to call [AppWebsocket::refresh_app_info]
    /// for the right CellId to be found to make the call.
    RoleName(RoleName),
    /// Call a cell by its clone id.
    ///
    /// Note that when using clone cells, if you create them after creating the [AppWebsocket], you will need to call [AppWebsocket::refresh_app_info]
    /// for the right CellId to be found to make the call.
    CloneId(CloneId),
}

impl From<CellId> for ZomeCallTarget {
    fn from(cell_id: CellId) -> Self {
        ZomeCallTarget::CellId(cell_id)
    }
}

impl From<RoleName> for ZomeCallTarget {
    fn from(role_name: RoleName) -> Self {
        ZomeCallTarget::RoleName(role_name)
    }
}

impl From<CloneId> for ZomeCallTarget {
    fn from(clone_id: CloneId) -> Self {
        ZomeCallTarget::CloneId(clone_id)
    }
}

fn is_clone_id(role_name: &RoleName) -> bool {
    role_name.as_str().contains('.')
}

fn get_base_role_name_from_clone_id(role_name: &RoleName) -> RoleName {
    RoleName::from(
        role_name
            .as_str()
            .split('.')
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
            .first()
            .unwrap(),
    )
}
