use std::{collections::HashMap, path::PathBuf};

use holochain::sweettest::SweetConductor;
use holochain_client::{AdminWebsocket, AppWebsocket, InstallAppBundlePayload};
use holochain_types::prelude::{AppBundleSource, CreateCloneCellPayload, InstalledAppId, DnaPhenotypeOpt, CloneId, AppRoleId};

#[tokio::test(flavor = "multi_thread")]
async fn clone_cell_management() {
    let conductor = SweetConductor::from_standard_config().await;
    let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
    let mut admin_ws = AdminWebsocket::connect(format!("ws://localhost:{}", admin_port))
        .await
        .unwrap();
    let app_id: InstalledAppId = "test-app".into();
    let role_id: AppRoleId = "foo".into();
    let agent_key = admin_ws.generate_agent_pub_key().await.unwrap();
    admin_ws
        .install_app_bundle(InstallAppBundlePayload {
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
    let clone_cell = app_ws
        .create_clone_cell(CreateCloneCellPayload {
            app_id: app_id.clone(),
            role_id: role_id.clone(),
            phenotype: DnaPhenotypeOpt::none().with_network_seed("seed".into()),
            membrane_proof: None,
            name: None,
        })
        .await
        .unwrap();
    assert_eq!(*clone_cell.as_id().agent_pubkey(), agent_key);
    assert_eq!(*clone_cell.as_role_id(), CloneId::new(&role_id, 0).to_string());
}
