use anyhow::{anyhow, Result};
use holo_hash::AgentPubKey;
use holochain_conductor_api::{AppInfo, CellInfo, ClonedCell, ProvisionedCell, ZomeCall};
use holochain_nonce::fresh_nonce;
use holochain_types::prelude::Signal;
use holochain_zome_types::prelude::{
    CellId, ExternIO, FunctionName, RoleName, Signature, Timestamp, ZomeCallUnsigned, ZomeName,
};
use lair_keystore_api::LairClient;

use crate::{AppWebsocket, ConductorApiError, ConductorApiResult};

#[derive(Clone)]
pub struct AppAgentWebsocket {
    pub my_pub_key: AgentPubKey,
    app_ws: AppWebsocket,
    app_info: AppInfo,
    lair_client: LairClient,
}

impl AppAgentWebsocket {
    pub async fn connect(url: String, app_id: String, lair_client: LairClient) -> Result<Self> {
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
            lair_client,
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

        let signed_zome_call = sign_zome_call_with_client(zome_call_unsigned, &self.lair_client)
            .await
            .map_err(|err| crate::ConductorApiError::SignZomeCallError(err))?;

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

/// Signs an unsigned zome call with the given LairClient
pub async fn sign_zome_call_with_client(
    zome_call_unsigned: ZomeCallUnsigned,
    client: &LairClient,
) -> Result<ZomeCall, String> {
    // sign the zome call
    let pub_key = zome_call_unsigned.provenance.clone();
    let mut pub_key_2 = [0; 32];
    pub_key_2.copy_from_slice(pub_key.get_raw_32());

    let data_to_sign = zome_call_unsigned
        .data_to_sign()
        .map_err(|e| format!("Failed to get data to sign from unsigned zome call: {}", e))?;

    let sig = client
        .sign_by_pub_key(pub_key_2.into(), None, data_to_sign)
        .await
        .map_err(|e| format!("Failed to sign zome call by pubkey: {}", e.str_kind()))?;

    let signature = Signature(*sig.0);

    let signed_zome_call = ZomeCall {
        cell_id: zome_call_unsigned.cell_id,
        zome_name: zome_call_unsigned.zome_name,
        fn_name: zome_call_unsigned.fn_name,
        payload: zome_call_unsigned.payload,
        cap_secret: zome_call_unsigned.cap_secret,
        provenance: zome_call_unsigned.provenance,
        nonce: zome_call_unsigned.nonce,
        expires_at: zome_call_unsigned.expires_at,
        signature,
    };

    return Ok(signed_zome_call);
}
