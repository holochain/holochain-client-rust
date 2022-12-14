use std::{collections::HashMap, path::PathBuf};

use holochain::{prelude::DeleteCloneCellPayload, sweettest::SweetConductor};
use holochain_client::{AdminWebsocket, AppWebsocket, InstallAppPayload};
use holochain_conductor_api::ZomeCall;
use holochain_types::prelude::{
    AppBundleSource, CloneCellId, CloneId, CreateCloneCellPayload, DnaModifiersOpt, ExternIO,
    InstalledAppId,
};
use holochain_zome_types::RoleName;

// // #[tokio::test(flavor = "multi_thread")]
// async fn clone_cell_management() {
//     let conductor = SweetConductor::from_standard_config().await;
//     let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();
//     let mut admin_ws = AdminWebsocket::connect(format!("ws://localhost:{}", admin_port))
//         .await
//         .unwrap();
//     let app_id: InstalledAppId = "test-app".into();
//     let role_name: RoleName = "foo".into();
//     let agent_key = admin_ws.generate_agent_pub_key().await.unwrap();
//     admin_ws
//         .install_app(InstallAppPayload {
//             agent_key: agent_key.clone(),
//             installed_app_id: Some(app_id.clone()),
//             membrane_proofs: HashMap::new(),
//             network_seed: None,
//             source: AppBundleSource::Path(PathBuf::from("./fixture/test.happ")),
//         })
//         .await
//         .unwrap();
//     admin_ws.enable_app(app_id.clone()).await.unwrap();
//     let app_api_port = admin_ws.attach_app_interface(30000).await.unwrap();
//     let mut app_ws = AppWebsocket::connect(format!("ws://localhost:{}", app_api_port))
//         .await
//         .unwrap();

//     // create clone cell
//     let clone_cell = app_ws
//         .create_clone_cell(CreateCloneCellPayload {
//             app_id: app_id.clone(),
//             role_name: role_name.clone(),
//             modifiers: DnaModifiersOpt::none().with_network_seed("seed".into()),
//             membrane_proof: None,
//             name: None,
//         })
//         .await
//         .unwrap();
//     assert_eq!(*clone_cell.as_id().agent_pubkey(), agent_key);
//     assert_eq!(
//         *clone_cell.as_role_name(),
//         CloneId::new(&role_name, 0).to_string()
//     );

//     // call clone cell should succeed
//     let response = app_ws
//         .call_zome(ZomeCall {
//             cell_id: clone_cell.as_id().clone(),
//             zome_name: "foo".into(),
//             fn_name: "foo".into(),
//             payload: ExternIO::encode(()).unwrap(),
//             cap_secret: None,
//             provenance: agent_key.clone(),
//         })
//         .await
//         .unwrap();
//     assert_eq!(response.decode::<String>().unwrap(), "foo");

//     // disable clone cell
//     app_ws
//         .disable_clone_cell(ArchiveCloneCellPayload {
//             app_id: app_id.clone(),
//             clone_cell_id: CloneCellId::CloneId(
//                 CloneId::try_from(clone_cell.as_role_name().clone()).unwrap(),
//             ),
//         })
//         .await
//         .unwrap();

//     // call disabled clone cell should fail
//     let response = app_ws
//         .call_zome(ZomeCall {
//             cell_id: clone_cell.as_id().clone(),
//             zome_name: "foo".into(),
//             fn_name: "foo".into(),
//             payload: ExternIO::encode(()).unwrap(),
//             cap_secret: None,
//             provenance: agent_key.clone(),
//         })
//         .await;
//     assert!(response.is_err());

//     // enable clone cell
//     let enabled_cell = app_ws
//         .enable_clone_cell(RestoreCloneCellPayload {
//             app_id: app_id.clone(),
//             clone_cell_id: CloneCellId::CloneId(
//                 CloneId::try_from(clone_cell.as_role_name().clone()).unwrap(),
//             ),
//         })
//         .await
//         .unwrap();
//     assert_eq!(enabled_cell, clone_cell);

//     // call enabled clone cell should succeed
//     let response = app_ws
//         .call_zome(ZomeCall {
//             cell_id: clone_cell.as_id().clone(),
//             zome_name: "foo".into(),
//             fn_name: "foo".into(),
//             payload: ExternIO::encode(()).unwrap(),
//             cap_secret: None,
//             provenance: agent_key.clone(),
//         })
//         .await
//         .unwrap();
//     assert_eq!(response.decode::<String>().unwrap(), "foo");

//     // disable clone cell again
//     app_ws
//         .disable_clone_cell(ArchiveCloneCellPayload {
//             app_id: app_id.clone(),
//             clone_cell_id: CloneCellId::CloneId(
//                 CloneId::try_from(clone_cell.as_role_name().clone()).unwrap(),
//             ),
//         })
//         .await
//         .unwrap();

//     // delete disabled clone cell
//     admin_ws
//         .delete_clone_cell(DeleteCloneCellPayload {
//             app_id: app_id.clone(),
//             clone_cell_id: CloneCellId::CellId(clone_cell.as_id().clone()),
//         })
//         .await
//         .unwrap();

//     // restore deleted clone cells should fail
//     let enable_clone_cell_response = app_ws
//         .enable_clone_cell(RestoreCloneCellPayload {
//             app_id: app_id.clone(),
//             clone_cell_id: CloneCellId::CloneId(
//                 CloneId::try_from(clone_cell.as_role_name().clone()).unwrap(),
//             ),
//         })
//         .await;
//     assert!(enable_clone_cell_response.is_err());
// }
