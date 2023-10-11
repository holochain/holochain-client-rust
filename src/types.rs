
use holo_hash::{AgentPubKey, DnaHash};
use holochain_conductor_api::ZomeCall;
use holochain_types::prelude::{InstalledAppId, CloneCellId, EnableCloneCellPayload, DisableCloneCellPayload, CreateCloneCellPayload, NetworkInfoRequestPayload};
use holochain_zome_types::{RoleName, DnaModifiersOpt, YamlProperties, MembraneProof, CellId, ZomeName, FunctionName, ExternIO, CapSecret, Signature, Nonce256Bits, Timestamp};


pub struct AppEnableCloneCellPayload {
  pub clone_cell_id: CloneCellId
}

impl AppEnableCloneCellPayload {
  pub fn into_enable_clone_cell_payload(self, app_id: InstalledAppId) -> EnableCloneCellPayload {
      EnableCloneCellPayload {
        app_id,
        clone_cell_id: self.clone_cell_id,
      }
  }
}

pub struct AppDisableCloneCellPayload {
  pub clone_cell_id: CloneCellId
}

impl AppDisableCloneCellPayload {
  pub fn into_disable_clone_cell_payload(self, app_id: InstalledAppId) -> DisableCloneCellPayload {
      DisableCloneCellPayload {
        app_id,
        clone_cell_id: self.clone_cell_id
      }
  }
}

pub struct AppCreateCloneCellPayload {
    /// The DNA's role name to clone
    pub role_name: RoleName,
    /// Modifiers to set for the new cell.
    /// At least one of the modifiers must be set to obtain a distinct hash for
    /// the clone cell's DNA.
    pub modifiers: DnaModifiersOpt<YamlProperties>,
    /// Optionally set a proof of membership for the clone cell
    pub membrane_proof: Option<MembraneProof>,
    /// Optionally a name for the DNA clone
    pub name: Option<String>,
}

impl AppCreateCloneCellPayload {
  pub fn into_create_clone_cell_payload(self, app_id: InstalledAppId) -> CreateCloneCellPayload {
      CreateCloneCellPayload {
        app_id,
        role_name: self.role_name,
        modifiers: self.modifiers,
        membrane_proof: self.membrane_proof,
        name: self.name,
      }
  }
}

/// The data provided over an app agent interface in order to make a zome call
pub struct AppAgentZomeCall {
    /// The ID of the cell containing the zome to be called
    pub cell_id: CellId,
    /// The zome containing the function to be called
    pub zome_name: ZomeName,
    /// The name of the zome function to call
    pub fn_name: FunctionName,
    /// The serialized data to pass as an argument to the zome function call
    pub payload: ExternIO,
    /// The capability request authorization
    ///
    /// This can be `None` and still succeed in the case where the function
    /// in the zome being called has been given an `Unrestricted` status
    /// via a `CapGrant`. Otherwise it will be necessary to provide a `CapSecret` for every call.
    pub cap_secret: Option<CapSecret>,
    pub signature: Signature,
    pub nonce: Nonce256Bits,
    pub expires_at: Timestamp,
}

impl AppAgentZomeCall {
  pub fn into_zome_call(self, agent_pub_key: AgentPubKey) -> ZomeCall {
      ZomeCall {
        provenance: agent_pub_key,
        cell_id: self.cell_id,
        zome_name: self.zome_name,
        fn_name: self.fn_name,
        payload: self.payload,
        cap_secret: self.cap_secret,
        signature: self.signature,
        nonce: self.nonce,
        expires_at: self.expires_at,
      }
  }
}

pub struct AppAgentNetworkInfoRequestPayload {
  /// Get gossip info for these DNAs
  pub dnas: Vec<DnaHash>,
  /// Timestamp in ms since which received amount of bytes from peers will
  /// be returned. Defaults to UNIX_EPOCH.
  pub last_time_queried: Option<Timestamp>,
}


impl AppAgentNetworkInfoRequestPayload {
  pub fn into_network_info_request_payload(self, agent_pub_key: AgentPubKey) -> NetworkInfoRequestPayload {
      NetworkInfoRequestPayload {
        agent_pub_key,
        dnas: self.dnas,
        last_time_queried: self.last_time_queried,
      }
  }
}
