use holochain::{
    prelude::{AppBundleSource, NetworkInfoRequestPayload, Signal},
    sweettest::SweetConductor,
};
use holochain_client::{AdminWebsocket, AppWebsocket, InstallAppPayload, InstalledAppId};
use holochain_conductor_api::{CellInfo, NetworkInfo};
use kitsune_p2p_types::fetch_pool::FetchPoolInfo;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Barrier},
};
use utilities::{authorize_signing_credentials, sign_zome_call};

#[tokio::test(flavor = "multi_thread")]
async fn network_info() {
    let conductor = SweetConductor::from_standard_config().await;
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let mut admin_ws = AdminWebsocket::connect(format!("ws://localhost:{}", admin_port))
        .await
        .unwrap();

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
    let app_ws_port = 33000;
    admin_ws.attach_app_interface(app_ws_port).await.unwrap();
    let mut app_ws = AppWebsocket::connect(format!("ws://localhost:{}", app_ws_port))
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
                num_ops_to_fetch: 0
            },
            current_number_of_peers: 1,
            arc_size: 1.0,
            total_network_peers: 1,
            // varies on local and ci machine
            bytes_since_last_time_queried: network_info[0].bytes_since_last_time_queried,
            completed_rounds_since_last_time_queried: 0
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_signal() {
    let conductor = SweetConductor::from_standard_config().await;
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let mut admin_ws = AdminWebsocket::connect(format!("ws://localhost:{}", admin_port))
        .await
        .unwrap();

    let app_id: InstalledAppId = "test-app".into();
    let agent_key = admin_ws.generate_agent_pub_key().await.unwrap();

    let _app_info = admin_ws
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
    let app_ws_port = 33001;
    admin_ws.attach_app_interface(app_ws_port).await.unwrap();
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
    const TEST_FN_NAME: &str = "emitter";

    let signing_credentials = authorize_signing_credentials(&mut admin_ws, &cell_id).await;
    let signed_zome_call = sign_zome_call(
        &cell_id,
        &TEST_ZOME_NAME,
        &TEST_FN_NAME,
        &signing_credentials,
    )
    .await;

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

    let _response = app_ws.call_zome(signed_zome_call).await.unwrap();

    barrier.wait();
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestString(pub String);
