use arbitrary::Arbitrary;
use ed25519_dalek::{Keypair, Signer};
pub use holochain_client::{AdminWebsocket, AgentPubKey, ZomeCall};
use holochain_nonce::fresh_nonce;
use holochain_zome_types::{
    CapAccess, CapSecret, CellId, ExternIO, FunctionName, GrantZomeCallCapabilityPayload,
    Signature, Timestamp, ZomeCallCapGrant, ZomeCallUnsigned, ZomeName,
};
use std::collections::BTreeSet;

pub struct SigningCredentials {
    cap_secret: CapSecret,
    keypair: Keypair,
    signing_key: AgentPubKey,
}

pub async fn authorize_signing_credentials(
    admin_ws: &mut AdminWebsocket,
    cell_id: &CellId,
) -> SigningCredentials {
    let mut rng = rand::thread_rng();
    let keypair: Keypair = Keypair::generate(&mut rng);
    let signing_key = AgentPubKey::from_raw_32(keypair.public.as_bytes().to_vec());

    let mut buf = arbitrary::Unstructured::new(&[]);
    let cap_secret = CapSecret::arbitrary(&mut buf).unwrap();

    let mut assignees = BTreeSet::new();
    assignees.insert(signing_key.clone());

    admin_ws
        .grant_zome_call_capability(GrantZomeCallCapabilityPayload {
            cell_id: cell_id.clone(),
            cap_grant: ZomeCallCapGrant {
                tag: "zome-call-signing-key".into(),
                functions: holochain_zome_types::GrantedFunctions::All,
                access: CapAccess::Assigned {
                    secret: cap_secret.clone(),
                    assignees: assignees.clone(),
                },
            },
        })
        .await
        .unwrap();

    SigningCredentials {
        cap_secret,
        keypair,
        signing_key,
    }
}

pub async fn sign_zome_call(
    cell_id: &CellId,
    zome_name: &str,
    fn_name: &str,
    signing_credentials: &SigningCredentials,
) -> ZomeCall {
    let (nonce, expires_at) = fresh_nonce(Timestamp::now()).unwrap();
    let unsigned_zome_call_payload = ZomeCallUnsigned {
        cap_secret: Some(signing_credentials.cap_secret),
        cell_id: cell_id.clone(),
        zome_name: ZomeName::from(zome_name),
        fn_name: FunctionName::from(fn_name),
        provenance: signing_credentials.signing_key.clone(),
        payload: ExternIO::encode(()).unwrap(),
        nonce,
        expires_at,
    };
    let hashed_zome_call = unsigned_zome_call_payload.data_to_sign().unwrap();

    let signature = signing_credentials.keypair.sign(&hashed_zome_call);

    ZomeCall {
        cap_secret: unsigned_zome_call_payload.cap_secret,
        cell_id: unsigned_zome_call_payload.cell_id,
        zome_name: unsigned_zome_call_payload.zome_name,
        fn_name: unsigned_zome_call_payload.fn_name,
        provenance: unsigned_zome_call_payload.provenance,
        payload: unsigned_zome_call_payload.payload,
        nonce: unsigned_zome_call_payload.nonce,
        expires_at: unsigned_zome_call_payload.expires_at,
        signature: Signature::from(signature.to_bytes()),
    }
}
