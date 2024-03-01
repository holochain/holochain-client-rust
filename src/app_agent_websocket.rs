use std::sync::Arc;

use anyhow::{anyhow, Result};
use holo_hash::AgentPubKey;
use holochain_conductor_api::{AppInfo, CellInfo, ProvisionedCell};
use holochain_nonce::fresh_nonce;
use holochain_types::prelude::Signal;
use holochain_zome_types::{
    clone::ClonedCell,
    prelude::{
        CellId, ExternIO, FunctionName, RoleName, Timestamp, ZomeCallUnsigned, ZomeName,
    },
};

use crate::{signing::{sign_zome_call, AgentSigner}, AppWebsocket, ConductorApiError, ConductorApiResult};

#[derive(Clone)]
pub struct AppAgentWebsocket {
    pub my_pub_key: AgentPubKey,
    app_ws: AppWebsocket,
    app_info: AppInfo,
    signer: Arc<Box<dyn AgentSigner>>,
}

impl AppAgentWebsocket {
    pub async fn connect(url: String, app_id: String, signer: Arc<Box<dyn AgentSigner>>) -> Result<Self> {
        let mut app_ws = AppWebsocket::connect(url).await?;

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

    pub async fn on_signal<F: Fn(Signal) -> () + 'static + Sync + Send>(
        &mut self,
        handler: F,
    ) -> Result<String> {
        let app_info = self.app_info.clone();
        self.app_ws
            .on_signal(move |signal| match signal.clone() {
                Signal::App {
                    cell_id,
                    zome_name: _,
                    signal: _,
                } => {
                    if app_info
                        .cell_info
                        .values()
                        .find(|cells| {
                            cells
                                .iter()
                                .find(|cell_info| match cell_info {
                                    CellInfo::Provisioned(cell) => cell.cell_id.eq(&cell_id),
                                    CellInfo::Cloned(cell) => cell.cell_id.eq(&cell_id),
                                    _ => false,
                                })
                                .is_some()
                        })
                        .is_some()
                    {
                        handler(signal);
                    }
                }
                _ => {}
            })
            .await
    }

    pub async fn call_zome(
        &mut self,
        role_name: RoleName,
        zome_name: ZomeName,
        fn_name: FunctionName,
        payload: ExternIO,
    ) -> ConductorApiResult<ExternIO> {
        let cell_id = self.get_cell_id_from_role_name(&role_name)?;

        let agent_pub_key = self.app_info.agent_pub_key.clone();

        let (nonce, expires_at) = fresh_nonce(Timestamp::now())
            .map_err(|err| crate::ConductorApiError::FreshNonceError(err))?;

        let zome_call_unsigned = ZomeCallUnsigned {
            provenance: agent_pub_key,
            cell_id,
            zome_name,
            fn_name,
            payload,
            cap_secret: None,
            expires_at,
            nonce,
        };

        let signed_zome_call = sign_zome_call(zome_call_unsigned, self.signer.clone()).await.map_err(|e| {
            ConductorApiError::SignZomeCallError(e.to_string())
        })?;
  
        let result = self.app_ws.call_zome(signed_zome_call).await?;

        Ok(result)
    }

    fn get_cell_id_from_role_name(&self, role_name: &RoleName) -> ConductorApiResult<CellId> {
        if is_clone_id(role_name) {
            let base_role_name = get_base_role_name_from_clone_id(role_name);

            let Some(role_cells) = self.app_info.cell_info.get(&base_role_name) else {
                return Err(ConductorApiError::CellNotFound);
            };

            let maybe_clone_cell: Option<ClonedCell> =
                role_cells.into_iter().find_map(|cell| match cell {
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
            return Ok(clone_cell.cell_id);
        } else {
            let Some(role_cells) = self.app_info.cell_info.get(role_name) else {
                return Err(ConductorApiError::CellNotFound);
            };

            let maybe_provisioned: Option<ProvisionedCell> =
                role_cells.into_iter().find_map(|cell| match cell {
                    CellInfo::Provisioned(provisioned_cell) => Some(provisioned_cell.clone()),
                    _ => None,
                });

            let provisioned_cell = maybe_provisioned.ok_or(ConductorApiError::CellNotFound)?;
            return Ok(provisioned_cell.cell_id);
        }
    }
}

fn is_clone_id(role_name: &RoleName) -> bool {
    role_name.as_str().contains(".")
}

fn get_base_role_name_from_clone_id(role_name: &RoleName) -> RoleName {
    RoleName::from(
        role_name
            .as_str()
            .split(".")
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
            .first()
            .unwrap(),
    )
}
