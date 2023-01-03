use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
};

use arbitrary::Arbitrary;
use ed25519_dalek::Keypair;
use holochain::{
    prelude::{AgentPubKey, AppBundleSource},
    sweettest::SweetConductor,
};
use holochain_client::{AdminWebsocket, AppWebsocket, InstallAppPayload, InstalledAppId, ZomeCall};
use holochain_conductor_api::CellInfo;
use holochain_zome_types::{
    CapAccess, CapSecret, ExternIO, GrantZomeCallCapabilityPayload, Nonce256Bits, Signature,
    Timestamp, ZomeCallCapGrant, ZomeCallUnsigned,
};

#[tokio::test(flavor = "multi_thread")]
async fn app_interfaces() {
    let conductor = SweetConductor::from_standard_config().await;
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let mut admin_ws = AdminWebsocket::connect(format!("ws://localhost:{}", admin_port))
        .await
        .unwrap();

    let app_interfaces = admin_ws.list_app_interfaces().await.unwrap();

    assert_eq!(app_interfaces.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn signed_zome_call() {
    use ed25519_dalek::Signer;
    use rand::Rng;

    let conductor = SweetConductor::from_standard_config().await;
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let mut admin_ws = AdminWebsocket::connect(format!("ws://localhost:{}", admin_port))
        .await
        .unwrap();

    let mut rng = rand::thread_rng();
    let random_number: u8 = rng.gen();
    let app_id: InstalledAppId = format!("test-app-{}", random_number).into();

    let agent_key = admin_ws.generate_agent_pub_key().await.unwrap();

    admin_ws
        .install_app(InstallAppPayload {
            agent_key: agent_key.clone(),
            installed_app_id: Some(app_id.clone()),
            membrane_proofs: HashMap::new(),
            network_seed: None,
            source: AppBundleSource::Path(PathBuf::from("./fixture/test.happ")),
        })
        .await
        .unwrap();

    admin_ws.enable_app(app_id.clone()).await.unwrap();

    let app_ws_port = admin_ws.attach_app_interface(30000).await.unwrap();
    let mut app_ws = AppWebsocket::connect(format!("ws://localhost:{}", app_ws_port))
        .await
        .unwrap();

    let installed_app = app_ws.app_info(app_id).await.unwrap().unwrap();

    let cells = installed_app.cell_info.into_values().next().unwrap();
    let cell_id = match cells[0].clone() {
        CellInfo::Provisioned(c) => c.cell_id,
        _ => panic!("Invalid cell type"),
    };

    // ******** SIGNED ZOME CALL  ********

    const TEST_ZOME_NAME: &str = "foo";
    const TEST_FN_NAME: &str = "foo";

    let keypair: Keypair = Keypair::generate(&mut rng);
    let signing_key = AgentPubKey::from_raw_32(keypair.public.as_bytes().to_vec());

    let mut buf = arbitrary::Unstructured::new(&[]);
    let cap_secret = CapSecret::arbitrary(&mut buf).unwrap();

    let mut functions = BTreeSet::new();
    let granted_function = (TEST_ZOME_NAME.into(), TEST_FN_NAME.into());
    functions.insert(granted_function.clone());

    let mut assignees = BTreeSet::new();
    assignees.insert(signing_key.clone());

    let _ = admin_ws
        .grant_zome_call_capability(GrantZomeCallCapabilityPayload {
            cell_id: cell_id.clone(),
            cap_grant: ZomeCallCapGrant {
                tag: "zome-call-signing-key".into(),
                functions: holochain_zome_types::GrantedFunctions::Listed(functions.clone()),
                access: CapAccess::Assigned {
                    secret: cap_secret.clone(),
                    assignees: assignees.clone(),
                },
            },
        })
        .await
        .unwrap();

    let unsigned_zome_call_payload = ZomeCallUnsigned {
        cap_secret: Some(cap_secret.clone()),
        cell_id: cell_id.clone(),
        zome_name: TEST_ZOME_NAME.into(),
        fn_name: TEST_FN_NAME.into(),
        provenance: signing_key.clone(),
        payload: ExternIO::encode(()).unwrap(),
        nonce: Nonce256Bits::from([0; 32]),
        expires_at: Timestamp(Timestamp::now().as_micros() + 100000),
    };
    let hashed_zome_call = unsigned_zome_call_payload.data_to_sign().unwrap();

    let signature = keypair.sign(&hashed_zome_call);

    let response = app_ws
        .call_zome(ZomeCall {
            cap_secret: unsigned_zome_call_payload.cap_secret,
            cell_id: unsigned_zome_call_payload.cell_id,
            zome_name: unsigned_zome_call_payload.zome_name,
            fn_name: unsigned_zome_call_payload.fn_name,
            provenance: unsigned_zome_call_payload.provenance,
            payload: unsigned_zome_call_payload.payload,
            nonce: unsigned_zome_call_payload.nonce,
            expires_at: unsigned_zome_call_payload.expires_at,
            signature: Signature::from(signature.to_bytes()),
        })
        .await
        .unwrap();
    assert_eq!(
        ExternIO::decode::<String>(&response).unwrap(),
        TEST_FN_NAME.to_string()
    );
}
