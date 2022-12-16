use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
};

use arbitrary::Arbitrary;
use ed25519_dalek::Keypair;
use holochain::{
    prelude::{DeleteCloneCellPayload, DisableCloneCellPayload, EnableCloneCellPayload},
    sweettest::SweetConductor,
};
use holochain_client::{AdminWebsocket, AgentPubKey, AppWebsocket, InstallAppPayload};
use holochain_conductor_api::{CellInfo, ZomeCall};
use holochain_types::prelude::{
    AppBundleSource, CloneCellId, CloneId, CreateCloneCellPayload, DnaModifiersOpt, ExternIO,
    InstalledAppId,
};
use holochain_zome_types::{
    CapAccess, CapSecret, GrantZomeCallCapabilityPayload, Nonce256Bits, RoleName, Signature,
    Timestamp, ZomeCallCapGrant, ZomeCallUnsigned,
};

#[tokio::test(flavor = "multi_thread")]
async fn clone_cell_management() {
    use ed25519_dalek::Signer;

    let conductor = SweetConductor::from_standard_config().await;
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let mut admin_ws = AdminWebsocket::connect(format!("ws://localhost:{}", admin_port))
        .await
        .unwrap();
    let app_id: InstalledAppId = "test-app".into();
    let role_name: RoleName = "foo".into();
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
    let app_api_port = admin_ws.attach_app_interface(30000).await.unwrap();
    let mut app_ws = AppWebsocket::connect(format!("ws://localhost:{}", app_api_port))
        .await
        .unwrap();
    let app_info = app_ws.app_info(app_id.clone()).await.unwrap().unwrap();
    let cell_id = match app_info.cell_info.into_values().next().unwrap()[0].clone() {
        CellInfo::Provisioned(c) => c.cell_id,
        _ => panic!("Invalid cell type"),
    };

    // Grant capability to a new keypair
    const TEST_ZOME_NAME: &str = "foo";
    const TEST_FN_NAME: &str = "foo";

    let mut rng = rand::thread_rng();
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
                functions: functions.clone(),
                access: CapAccess::Assigned {
                    secret: cap_secret.clone(),
                    assignees: assignees.clone(),
                },
            },
        })
        .await
        .unwrap();

    // create clone cell
    let clone_cell = app_ws
        .create_clone_cell(CreateCloneCellPayload {
            app_id: app_id.clone(),
            role_name: role_name.clone(),
            modifiers: DnaModifiersOpt::none().with_network_seed("seed".into()),
            membrane_proof: None,
            name: None,
        })
        .await
        .unwrap();
    assert_eq!(*clone_cell.as_id().agent_pubkey(), agent_key);
    assert_eq!(
        *clone_cell.as_role_name(),
        CloneId::new(&role_name, 0).to_string()
    );

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
    // call clone cell should succeed
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
    assert_eq!(response.decode::<String>().unwrap(), "foo");

    // disable clone cell
    app_ws
        .disable_clone_cell(DisableCloneCellPayload {
            app_id: app_id.clone(),
            clone_cell_id: CloneCellId::CloneId(
                CloneId::try_from(clone_cell.as_role_name().clone()).unwrap(),
            ),
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

    // call disabled clone cell should fail
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
        .await;
    assert!(response.is_err());

    // enable clone cell
    let enabled_cell = app_ws
        .enable_clone_cell(EnableCloneCellPayload {
            app_id: app_id.clone(),
            clone_cell_id: CloneCellId::CloneId(
                CloneId::try_from(clone_cell.as_role_name().clone()).unwrap(),
            ),
        })
        .await
        .unwrap();
    assert_eq!(enabled_cell, clone_cell);

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

    // call enabled clone cell should succeed
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
    assert_eq!(response.decode::<String>().unwrap(), "foo");

    // disable clone cell again
    app_ws
        .disable_clone_cell(DisableCloneCellPayload {
            app_id: app_id.clone(),
            clone_cell_id: CloneCellId::CloneId(
                CloneId::try_from(clone_cell.as_role_name().clone()).unwrap(),
            ),
        })
        .await
        .unwrap();

    // delete disabled clone cell
    admin_ws
        .delete_clone_cell(DeleteCloneCellPayload {
            app_id: app_id.clone(),
            clone_cell_id: CloneCellId::CellId(clone_cell.as_id().clone()),
        })
        .await
        .unwrap();

    // restore deleted clone cells should fail
    let enable_clone_cell_response = app_ws
        .enable_clone_cell(EnableCloneCellPayload {
            app_id: app_id.clone(),
            clone_cell_id: CloneCellId::CloneId(
                CloneId::try_from(clone_cell.as_role_name().clone()).unwrap(),
            ),
        })
        .await;
    assert!(enable_clone_cell_response.is_err());
}
