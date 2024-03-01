use std::sync::Arc;
use anyhow::Result;
use holo_hash::AgentPubKey;
use holochain_zome_types::{cell::CellId, dependencies::holochain_integrity_types::Signature};
use lair_keystore_api::LairClient;
use async_trait::async_trait;
use crate::AgentSigner;


pub struct LairAgentSigner {
    lair_client: Arc<LairClient>,
}

impl LairAgentSigner {
    pub fn new(lair_client: Arc<LairClient>) -> Self {
        Self { lair_client }
    }
}

#[async_trait]
impl AgentSigner for LairAgentSigner {
    async fn sign(&self, _cell_id: &CellId, provenance: AgentPubKey, data_to_sign: Arc<[u8]>) -> Result<Signature> {
        let public_key: [u8; 32] = provenance.get_raw_32().try_into()?;

        let signature = self.lair_client.sign_by_pub_key(
            public_key.into(),
            None,
            data_to_sign,
        ).await?;

        Ok(Signature(*signature.0))
    }
}
