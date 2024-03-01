use std::sync::Arc;

use anyhow::Result;
use holo_hash::AgentPubKey;
use holochain_conductor_api::ZomeCall;
use holochain_zome_types::{cell::CellId, dependencies::holochain_integrity_types::Signature, zome_io::ZomeCallUnsigned};
use async_trait::async_trait;

#[cfg(feature = "client_signing")]
pub(crate) mod client_signing;

#[cfg(feature = "lair_signing")]
pub(crate) mod lair_signing;

#[async_trait]
pub trait AgentSigner {
    /// Sign the given data with the public key found in the agent id of the provenance.
    async fn sign(&self, cell_id: &CellId, provenance: AgentPubKey, data_to_sign: Arc<[u8]>) -> Result<Signature>;
}

/// Signs an unsigned zome call using the provided signing implementation
pub(crate) async fn sign_zome_call(
    zome_call_unsigned: ZomeCallUnsigned,
    signer: Arc<Box<dyn AgentSigner>>,
) -> Result<ZomeCall> {
    let pub_key = zome_call_unsigned.provenance.clone();
    let mut pub_key_2 = [0; 32];
    pub_key_2.copy_from_slice(pub_key.get_raw_32());

    let data_to_sign = zome_call_unsigned
        .data_to_sign()
        .map_err(|e| anyhow::anyhow!("Failed to get data to sign from unsigned zome call: {}", e))?;

    let signature = signer.sign(&zome_call_unsigned.cell_id, pub_key, data_to_sign).await?;

    // let signature = Signature(*sig.0);

    Ok(ZomeCall {
        cell_id: zome_call_unsigned.cell_id,
        zome_name: zome_call_unsigned.zome_name,
        fn_name: zome_call_unsigned.fn_name,
        payload: zome_call_unsigned.payload,
        cap_secret: zome_call_unsigned.cap_secret,
        provenance: zome_call_unsigned.provenance,
        nonce: zome_call_unsigned.nonce,
        expires_at: zome_call_unsigned.expires_at,
        signature,
    })
}

