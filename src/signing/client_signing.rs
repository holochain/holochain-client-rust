use super::AgentSigner;
use async_trait::async_trait;
use ed25519_dalek::Signer;
use holo_hash::AgentPubKey;
use holochain_zome_types::{
    capability::CapSecret, cell::CellId, dependencies::holochain_integrity_types::Signature,
};
use std::{collections::HashMap, sync::Arc};

pub struct SigningCredentials {
    pub signing_agent_key: holo_hash::AgentPubKey,
    pub keypair: ed25519_dalek::SigningKey,
    pub cap_secret: CapSecret,
}

/// Custom debug implementation which won't attempt to print the `cap_secret` or `keypair`
impl std::fmt::Debug for SigningCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SigningCredentials")
            .field("signing_agent_key", &self.signing_agent_key)
            .finish()
    }
}

#[derive(Debug, Default)]
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
    async fn sign(
        &self,
        cell_id: &CellId,
        _provenance: AgentPubKey,
        data_to_sign: Arc<[u8]>,
    ) -> Result<Signature, anyhow::Error> {
        let credentials = self
            .credentials
            .get(cell_id)
            .ok_or_else(|| anyhow::anyhow!("No credentials found for cell: {:?}", cell_id))?;
        println!("Using credentials: {:?}", credentials);
        let signature = credentials.keypair.try_sign(&data_to_sign)?;
        println!("Signature: {:?}", signature.to_bytes());
        Ok(Signature(signature.to_bytes()))
    }

    fn get_provenance(&self, cell_id: &CellId) -> Option<AgentPubKey> {
        self.credentials
            .get(cell_id)
            .map(|c| c.signing_agent_key.clone())
    }

    fn get_cap_secret(&self, cell_id: &CellId) -> Option<CapSecret> {
        self.credentials.get(cell_id).map(|c| c.cap_secret).clone()
    }
}

/// Convert the ClientAgentSigner into an Arc<Box<dyn AgentSigner>>
impl From<ClientAgentSigner> for Arc<Box<dyn AgentSigner>> {
    fn from(cas: ClientAgentSigner) -> Self {
        Arc::new(Box::new(cas))
    }
}
