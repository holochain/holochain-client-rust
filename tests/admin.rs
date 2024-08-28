use holochain::test_utils::itertools::Itertools;
use holochain::{prelude::AppBundleSource, sweettest::SweetConductor};
use holochain_client::{
    AdminWebsocket, AppWebsocket, AuthorizeSigningCredentialsPayload, ClientAgentSigner,
    InstallAppPayload, InstalledAppId,
};
use holochain_conductor_api::{CellInfo, StorageBlob};
use holochain_types::websocket::AllowedOrigins;
use holochain_zome_types::prelude::ExternIO;
use kitsune_p2p_types::fixt::AgentInfoSignedFixturator;
use std::collections::BTreeSet;
use std::net::Ipv4Addr;
use std::{collections::HashMap, path::PathBuf};

const ROLE_NAME: &str = "foo";

#[tokio::test(flavor = "multi_thread")]
async fn app_interfaces() {
    let conductor = SweetConductor::from_standard_config().await;

    // Connect admin client
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let admin_ws = AdminWebsocket::connect((Ipv4Addr::LOCALHOST, admin_port))
        .await
        .unwrap();

    let app_interfaces = admin_ws.list_app_interfaces().await.unwrap();

    assert_eq!(app_interfaces.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn signed_zome_call() {
    let conductor = SweetConductor::from_standard_config().await;

    // Connect admin client
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let admin_ws = AdminWebsocket::connect((Ipv4Addr::LOCALHOST, admin_port))
        .await
        .unwrap();

    // Set up the test app
    let app_id: InstalledAppId = "test-app".into();
    let installed_app = admin_ws
        .install_app(InstallAppPayload {
            agent_key: None,
            installed_app_id: Some(app_id.clone()),
            membrane_proofs: None,
            network_seed: None,
            existing_cells: HashMap::new(),
            source: AppBundleSource::Path(PathBuf::from("./fixture/test.happ")),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    admin_ws.enable_app(app_id.clone()).await.unwrap();

    // Connect app agent client
    let app_ws_port = admin_ws
        .attach_app_interface(0, AllowedOrigins::Any, None)
        .await
        .unwrap();
    let issued_token = admin_ws
        .issue_app_auth_token(app_id.clone().into())
        .await
        .unwrap();
    let signer = ClientAgentSigner::default();
    let app_ws = AppWebsocket::connect(
        (Ipv4Addr::LOCALHOST, app_ws_port),
        issued_token.token,
        signer.clone().into(),
    )
    .await
    .unwrap();

    let cells = installed_app.cell_info.into_values().next().unwrap();
    let cell_id = match cells[0].clone() {
        CellInfo::Provisioned(c) => c.cell_id,
        _ => panic!("Invalid cell type"),
    };

    // ******** SIGNED ZOME CALL  ********

    const TEST_ZOME_NAME: &str = "foo";
    const TEST_FN_NAME: &str = "foo";

    let credentials = admin_ws
        .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
            cell_id: cell_id.clone(),
            functions: None,
        })
        .await
        .unwrap();
    signer.add_credentials(cell_id.clone(), credentials);

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
    let admin_ws = AdminWebsocket::connect(format!("127.0.0.1:{}", admin_port))
        .await
        .unwrap();
    let app_id: InstalledAppId = "test-app".into();
    let agent_key = admin_ws.generate_agent_pub_key().await.unwrap();
    admin_ws
        .install_app(InstallAppPayload {
            agent_key: Some(agent_key.clone()),
            installed_app_id: Some(app_id.clone()),
            membrane_proofs: None,
            network_seed: None,
            existing_cells: HashMap::new(),
            source: AppBundleSource::Path(PathBuf::from("./fixture/test.happ")),
            ignore_genesis_failure: false,
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
    let admin_ws = AdminWebsocket::connect(format!("127.0.0.1:{}", admin_port))
        .await
        .unwrap();
    let app_id: InstalledAppId = "test-app".into();
    let agent_key = admin_ws.generate_agent_pub_key().await.unwrap();
    admin_ws
        .install_app(InstallAppPayload {
            agent_key: Some(agent_key.clone()),
            installed_app_id: Some(app_id.clone()),
            membrane_proofs: None,
            network_seed: None,
            existing_cells: HashMap::new(),
            source: AppBundleSource::Path(PathBuf::from("./fixture/test.happ")),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    admin_ws.enable_app(app_id.clone()).await.unwrap();

    let network_stats = admin_ws.dump_network_stats().await.unwrap();

    assert!(network_stats.contains("\"backend\": \"tx2-quic\""));
}

#[tokio::test(flavor = "multi_thread")]
async fn get_compatible_cells() {
    let conductor = SweetConductor::from_standard_config().await;
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let admin_ws = AdminWebsocket::connect(format!("127.0.0.1:{}", admin_port))
        .await
        .unwrap();
    let app_id: InstalledAppId = "test-app".into();
    let agent_key = admin_ws.generate_agent_pub_key().await.unwrap();
    let app_info = admin_ws
        .install_app(InstallAppPayload {
            agent_key: Some(agent_key.clone()),
            installed_app_id: Some(app_id.clone()),
            membrane_proofs: None,
            network_seed: None,
            existing_cells: HashMap::new(),
            source: AppBundleSource::Path(PathBuf::from("./fixture/test.happ")),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    let cell_id = if let CellInfo::Provisioned(provisioned_cell) =
        &app_info.cell_info.get(ROLE_NAME).unwrap()[0]
    {
        provisioned_cell.cell_id.clone()
    } else {
        panic!("expected provisioned cell")
    };
    let dna_hash = cell_id.dna_hash().clone();
    let mut compatible_cells = admin_ws.get_compatible_cells(dna_hash).await.unwrap();
    assert_eq!(
        compatible_cells.len(),
        1,
        "compatible cells set should have 1 element"
    );
    let cell_1 = compatible_cells.pop_first().unwrap();
    let mut expected_cells = BTreeSet::new();
    expected_cells.insert(cell_id);
    assert_eq!(
        cell_1,
        (app_id, expected_cells),
        "only cell should be expected app cell"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn revoke_agent_key() {
    let conductor = SweetConductor::from_standard_config().await;
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let admin_ws = AdminWebsocket::connect(format!("127.0.0.1:{}", admin_port))
        .await
        .unwrap();

    let app_info = admin_ws
        .install_app(InstallAppPayload {
            agent_key: None,
            installed_app_id: None,
            membrane_proofs: None,
            network_seed: None,
            existing_cells: HashMap::new(),
            source: AppBundleSource::Path(PathBuf::from("./fixture/test.happ")),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    let app_id = app_info.installed_app_id.clone();
    admin_ws.enable_app(app_id.clone()).await.unwrap();

    let agent_key = app_info.agent_pub_key.clone();
    let response = admin_ws
        .revoke_agent_key(app_id.clone(), agent_key.clone())
        .await
        .unwrap();
    assert_eq!(response, vec![]);
    let response = admin_ws
        .revoke_agent_key(app_id, agent_key.clone())
        .await
        .unwrap();
    let cell_id = if let CellInfo::Provisioned(provisioned_cell) =
        &app_info.cell_info.get(ROLE_NAME).unwrap()[0]
    {
        provisioned_cell.cell_id.clone()
    } else {
        panic!("expected provisioned cell")
    };
    assert!(matches!(&response[0], (cell, error) if *cell == cell_id && error.contains("invalid")));
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_info() {
    let conductor = SweetConductor::from_standard_config().await;
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let admin_ws = AdminWebsocket::connect(format!("127.0.0.1:{}", admin_port))
        .await
        .unwrap();
    let app_id: InstalledAppId = "test-app".into();
    let agent_key = admin_ws.generate_agent_pub_key().await.unwrap();
    admin_ws
        .install_app(InstallAppPayload {
            agent_key: Some(agent_key.clone()),
            installed_app_id: Some(app_id.clone()),
            existing_cells: HashMap::new(),
            membrane_proofs: Some(HashMap::new()),
            network_seed: None,
            source: AppBundleSource::Path(PathBuf::from("./fixture/test.happ")),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    admin_ws.enable_app(app_id.clone()).await.unwrap();

    let agent_infos = admin_ws.agent_info(None).await.unwrap();
    assert_eq!(agent_infos.len(), 1);

    let other_agent = ::fixt::fixt!(AgentInfoSigned);
    admin_ws
        .add_agent_info(vec![other_agent.clone()])
        .await
        .unwrap();

    let agent_infos = admin_ws.agent_info(None).await.unwrap();
    assert_eq!(agent_infos.len(), 2);
    assert!(agent_infos.contains(&other_agent));
}
