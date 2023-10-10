
use holochain_types::prelude::{InstalledAppId, CloneCellId, EnableCloneCellPayload, DisableCloneCellPayload, CreateCloneCellPayload};
use holochain_zome_types::{RoleName, DnaModifiersOpt, YamlProperties, MembraneProof};


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


