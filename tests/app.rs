use holochain::{
    prelude::{AppBundleSource, NetworkInfoRequestPayload, Signal},
    sweettest::SweetConductor,
};
use holochain_client::{
    AdminWebsocket, AppWebsocket, AuthorizeSigningCredentialsPayload, ClientAgentSigner,
    InstallAppPayload, InstalledAppId,
};
use holochain_conductor_api::{CellInfo, NetworkInfo};
use holochain_types::websocket::AllowedOrigins;
use holochain_zome_types::zome_io::ExternIO;
use kitsune_p2p_types::fetch_pool::FetchPoolInfo;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Barrier},
};

#[tokio::test(flavor = "multi_thread")]
async fn network_info() {
    let conductor = SweetConductor::from_standard_config().await;

    // Connect admin client
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let admin_ws = AdminWebsocket::connect((Ipv4Addr::LOCALHOST, admin_port))
        .await
        .unwrap();

    // Set up the test app
    let app_id: InstalledAppId = "test-app".into();
    let agent_key = admin_ws.generate_agent_pub_key().await.unwrap();
    let app_info = admin_ws
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

    // Connect app client
    let app_ws_port = admin_ws
        .attach_app_interface(0, AllowedOrigins::Any, None)
        .await
        .unwrap();
    let token_issued = admin_ws
        .issue_app_auth_token(app_id.clone().into())
        .await
        .unwrap();
    let signer = ClientAgentSigner::default().into();
    let app_ws = AppWebsocket::connect(
        (Ipv4Addr::LOCALHOST, app_ws_port),
        token_issued.token,
        signer,
    )
    .await
    .unwrap();

    let dna_hash = match &app_info.cell_info.get("foo").unwrap()[0] {
        holochain_conductor_api::CellInfo::Provisioned(cell) => cell.cell_id.dna_hash().to_owned(),
        _ => panic!("wrong cell type"),
    };
    let network_info = app_ws
        .network_info(NetworkInfoRequestPayload {
            agent_pub_key: agent_key,
            dnas: vec![dna_hash],
            last_time_queried: None,
        })
        .await
        .unwrap();

    assert_eq!(
        network_info[0],
        NetworkInfo {
            fetch_pool_info: FetchPoolInfo {
                op_bytes_to_fetch: 0,
                num_ops_to_fetch: 0,
            },
            current_number_of_peers: 1,
            arc_size: 1.0,
            total_network_peers: 1,
            // varies on local and ci machine
            bytes_since_last_time_queried: network_info[0].bytes_since_last_time_queried,
            completed_rounds_since_last_time_queried: 0,
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_signal() {
    let conductor = SweetConductor::from_standard_config().await;

    // Connect admin client
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let admin_ws = AdminWebsocket::connect((Ipv4Addr::LOCALHOST, admin_port))
        .await
        .unwrap();

    // Set up the test app
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

    // Connect app agent client
    let app_ws_port = admin_ws
        .attach_app_interface(0, AllowedOrigins::Any, None)
        .await
        .unwrap();
    let token_issued = admin_ws
        .issue_app_auth_token(app_id.clone().into())
        .await
        .unwrap();
    let mut signer = ClientAgentSigner::default();
    let app_ws = AppWebsocket::connect(
        (Ipv4Addr::LOCALHOST, app_ws_port),
        token_issued.token,
        signer.clone().into(),
    )
    .await
    .unwrap();

    let installed_app = app_ws.app_info().await.unwrap().unwrap();

    let cells = installed_app.cell_info.into_values().next().unwrap();
    let cell_id = match cells[0].clone() {
        CellInfo::Provisioned(c) => c.cell_id,
        _ => panic!("Invalid cell type"),
    };

    // ******** SIGNED ZOME CALL  ********

    const TEST_ZOME_NAME: &str = "foo";
    const TEST_FN_NAME: &str = "emitter";

    let credentials = admin_ws
        .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
            cell_id: cell_id.clone(),
            functions: None,
        })
        .await
        .unwrap();
    signer.add_credentials(cell_id.clone(), credentials);

    let barrier = Arc::new(Barrier::new(2));
    let barrier_clone = barrier.clone();

    app_ws
        .on_signal(move |signal| match signal {
            Signal::App { signal, .. } => {
                let ts: TestString = signal.into_inner().decode().unwrap();
                assert_eq!(ts.0.as_str(), "i am a signal");
                barrier_clone.wait();
            }
            _ => panic!("Invalid signal"),
        })
        .await
        .unwrap();

    app_ws
        .call_zome(
            cell_id.into(),
            TEST_ZOME_NAME.into(),
            TEST_FN_NAME.into(),
            ExternIO::encode(()).unwrap(),
        )
        .await
        .unwrap();

    barrier.wait();
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestString(pub String);
