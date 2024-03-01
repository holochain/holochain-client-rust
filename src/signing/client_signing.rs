use std::{collections::HashMap, sync::Arc};
use async_trait::async_trait;
use ed25519_dalek::Signer;
use holo_hash::AgentPubKey;
use holochain_zome_types::{cell::CellId, dependencies::holochain_integrity_types::Signature};
use super::AgentSigner;

pub struct SigningCredentials {
    pub signing_agent_key: holo_hash::AgentPubKey,
    pub keypair: ed25519_dalek::SigningKey,
    pub cap_secret: [u8; holochain_zome_types::capability::CAP_SECRET_BYTES],
}

pub struct ClientAgentSigner {
    credentials: HashMap<CellId, SigningCredentials>,
}

impl ClientAgentSigner {
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
        }
    }

    pub fn add_credentials(&mut self, cell_id: CellId, credentials: SigningCredentials) {
        self.credentials.insert(cell_id, credentials);
    }
}

#[async_trait]
impl AgentSigner for ClientAgentSigner {
    async fn sign(&self, cell_id: &CellId, _provenance: AgentPubKey, data_to_sign: Arc<[u8]>) -> Result<Signature, anyhow::Error> {
        let credentials = self.credentials.get(cell_id).ok_or_else(|| anyhow::anyhow!("No credentials found for cell: {:?}", cell_id))?;
        let signature = credentials.keypair.try_sign(&data_to_sign)?;
        Ok(Signature(signature.to_bytes()))
    }
}
