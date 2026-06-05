use serde_json::Value;

#[test]
fn panel_window_can_receive_backend_events() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let capability_path = manifest_dir.join("capabilities/default.json");
    let capability: Value =
        serde_json::from_str(&std::fs::read_to_string(capability_path).unwrap()).unwrap();

    let windows = capability["windows"].as_array().unwrap();
    assert!(
        windows.iter().any(|window| window == "main"),
        "main window must keep its capability"
    );
    assert!(
        windows.iter().any(|window| window == "panel"),
        "panel window needs IPC capability to listen for open/close events"
    );

    let permissions = capability["permissions"].as_array().unwrap();
    assert!(
        permissions
            .iter()
            .any(|permission| permission == "core:event:default"),
        "panel open/close is delivered through Tauri event listen"
    );
}
