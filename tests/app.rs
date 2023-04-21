use holochain::{
    prelude::{
        kitsune_p2p::dependencies::kitsune_p2p_fetch::FetchPoolInfo, AppBundleSource,
        NetworkInfoRequestPayload,
    },
    sweettest::SweetConductor,
};
use holochain_client::{AdminWebsocket, AppWebsocket, InstallAppPayload, InstalledAppId};
use holochain_conductor_api::NetworkInfo;
use std::{collections::HashMap, path::PathBuf};

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
            bytes_since_last_time_queried: 1844,
            completed_rounds_since_last_time_queried: 0
        }
    );
}
