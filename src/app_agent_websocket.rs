use std::{ops::DerefMut, sync::Arc};

use crate::{
    signing::{sign_zome_call, AgentSigner},
    AppWebsocket, ConductorApiError, ConductorApiResult,
};
use anyhow::{anyhow, Result};
use holo_hash::AgentPubKey;
use holochain_conductor_api::{AppInfo, CellInfo, ClonedCell, ProvisionedCell};
use holochain_state::nonce::fresh_nonce;
use holochain_types::{app::InstalledAppId, prelude::CloneCellId};
use holochain_zome_types::prelude::{
    CellId, ExternIO, FunctionName, RoleName, Timestamp, ZomeCallUnsigned, ZomeName,
};
use std::ops::Deref;

#[derive(Clone)]
pub struct AppAgentWebsocket {
    pub my_pub_key: AgentPubKey,
    app_ws: AppWebsocket,
    app_info: AppInfo,
    signer: Arc<Box<dyn AgentSigner + Send + Sync>>,
}

impl AppAgentWebsocket {
    pub async fn connect(
        url: String,
        app_id: InstalledAppId,
        signer: Arc<Box<dyn AgentSigner + Send + Sync>>,
    ) -> Result<Self> {
        let app_ws = AppWebsocket::connect(url).await?;
        AppAgentWebsocket::from_existing(app_ws, app_id, signer).await
    }

    pub async fn from_existing(
        mut app_ws: AppWebsocket,
        app_id: InstalledAppId,
        signer: Arc<Box<dyn AgentSigner + Send + Sync>>,
    ) -> Result<Self> {
        let app_info = app_ws
            .app_info(app_id.clone())
            .await
            .map_err(|err| anyhow!("Error fetching app_info {err:?}"))?
            .ok_or(anyhow!("App doesn't exist"))?;

        Ok(AppAgentWebsocket {
            my_pub_key: app_info.agent_pub_key.clone(),
            app_ws,
            app_info,
            signer,
        })
    }

    pub async fn call_zome(
        &mut self,
        target: ZomeCallTarget,
        zome_name: ZomeName,
        fn_name: FunctionName,
        payload: ExternIO,
    ) -> ConductorApiResult<ExternIO> {
        let cell_id = match target {
            ZomeCallTarget::CellId(cell_id) => cell_id,
            ZomeCallTarget::RoleName(role_name) => self.get_cell_id_from_role_name(&role_name)?,
            ZomeCallTarget::CloneId(clone_id) => match clone_id {
                CloneCellId::CellId(cell_id) => cell_id,
                CloneCellId::CloneId(clone_id) => self.get_cell_id_from_role_name(&clone_id.0)?,
            },
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

        let result = self.app_ws.call_zome(signed_zome_call).await?;

        Ok(result)
    }

    /// Gets a new copy of the [AppInfo] for the app this agent is connected to.
    ///
    /// This is useful if you have made changes to the app, such as creating new clone cells, and need to refresh the app info.
    pub async fn refresh_app_info(&mut self) -> Result<()> {
        self.app_info = self
            .app_ws
            .app_info(self.app_info.installed_app_id.clone())
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
    /// Note that when using clone cells, if you create them after creating the [AppAgentWebsocket], you will need to call [AppAgentWebsocket::refresh_app_info]
    /// for the right CellId to be found to make the call.
    RoleName(RoleName),
    /// Call a cell by its clone cell id.
    ///
    /// Note that when using clone cells, if you create them after creating the [AppAgentWebsocket], you will need to call [AppAgentWebsocket::refresh_app_info]
    /// for the right CellId to be found to make the call.
    CloneId(CloneCellId),
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

impl From<CloneCellId> for ZomeCallTarget {
    fn from(clone_id: CloneCellId) -> Self {
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

/// Make the [AppWebsocket] functionality available through the [AppAgentWebsocket]
impl Deref for AppAgentWebsocket {
    type Target = AppWebsocket;

    fn deref(&self) -> &Self::Target {
        &self.app_ws
    }
}

impl DerefMut for AppAgentWebsocket {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.app_ws
    }
}
