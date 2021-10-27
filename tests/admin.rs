use holochain::sweettest::SweetConductor;
use holochain_conductor_client::AdminWebsocket;

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
