use holochain::test_utils::itertools::Itertools;
use holochain::{prelude::AppBundleSource, sweettest::SweetConductor};
use holochain_client::{
    AdminWebsocket, AppAgentWebsocket, AppWebsocket, AuthorizeSigningCredentialsPayload,
    ClientAgentSigner, InstallAppPayload, InstalledAppId,
};
use holochain_conductor_api::{CellInfo, StorageBlob};
use holochain_types::websocket::AllowedOrigins;
use holochain_zome_types::prelude::ExternIO;
use std::{collections::HashMap, path::PathBuf};

#[tokio::test(flavor = "multi_thread")]
async fn app_interfaces() {
    let conductor = SweetConductor::from_standard_config().await;
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let mut admin_ws = AdminWebsocket::connect(format!("127.0.0.1:{}", admin_port))
        .await
        .unwrap();

    let app_interfaces = admin_ws.list_app_interfaces().await.unwrap();

    assert_eq!(app_interfaces.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn signed_zome_call() {
    let conductor = SweetConductor::from_standard_config().await;
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let mut admin_ws = AdminWebsocket::connect(format!("127.0.0.1:{}", admin_port))
        .await
        .unwrap();
    let app_id: InstalledAppId = "test-app".into();
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
    let app_ws_port = admin_ws
        .attach_app_interface(30000, AllowedOrigins::Any)
        .await
        .unwrap();
    let mut app_ws = AppWebsocket::connect(format!("127.0.0.1:{}", app_ws_port))
        .await
        .unwrap();
    let installed_app = app_ws.app_info(app_id.clone()).await.unwrap().unwrap();

    let cells = installed_app.cell_info.into_values().next().unwrap();
    let cell_id = match cells[0].clone() {
        CellInfo::Provisioned(c) => c.cell_id,
        _ => panic!("Invalid cell type"),
    };

    // ******** SIGNED ZOME CALL  ********

    const TEST_ZOME_NAME: &str = "foo";
    const TEST_FN_NAME: &str = "foo";

    let mut signer = ClientAgentSigner::default();
    let credentials = admin_ws
        .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
            cell_id: cell_id.clone(),
            functions: None,
        })
        .await
        .unwrap();
    signer.add_credentials(cell_id.clone(), credentials);

    let mut app_ws = AppAgentWebsocket::from_existing(app_ws, app_id, signer.into())
        .await
        .unwrap();

    let response = app_ws
        .call_zome(
            cell_id.into(),
            TEST_ZOME_NAME.into(),
            TEST_FN_NAME.into(),
            ExternIO::encode(()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        ExternIO::decode::<String>(&response).unwrap(),
        TEST_FN_NAME.to_string()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn storage_info() {
    let conductor = SweetConductor::from_standard_config().await;
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let mut admin_ws = AdminWebsocket::connect(format!("127.0.0.1:{}", admin_port))
        .await
        .unwrap();
    let app_id: InstalledAppId = "test-app".into();
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

    let storage_info = admin_ws.storage_info().await.unwrap();

    let matched_storage_info = storage_info
        .blobs
        .iter()
        .filter(|b| match b {
            StorageBlob::Dna(dna_storage_info) => dna_storage_info.used_by.contains(&app_id),
        })
        .collect_vec();
    assert_eq!(1, matched_storage_info.len());
}

#[tokio::test(flavor = "multi_thread")]
async fn dump_network_stats() {
    let conductor = SweetConductor::from_standard_config().await;
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let mut admin_ws = AdminWebsocket::connect(format!("127.0.0.1:{}", admin_port))
        .await
        .unwrap();
    let app_id: InstalledAppId = "test-app".into();
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

    let network_stats = admin_ws.dump_network_stats().await.unwrap();

    assert!(network_stats.contains("\"backend\": \"tx2-quic\""));
}
