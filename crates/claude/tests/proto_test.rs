use proto_pdk_test_utils::*;

generate_download_install_tests!("claude_tool", "2.1.310");

// `latest` resolution is covered by the generated `resolves_latest_alias` test;
// pin to old releases here so the expectations don't change over time.
generate_resolve_versions_tests!("claude_tool", {
    "1.0" => "1.0.128",
    "2.0" => "2.0.77",
    "2.1.110" => "2.1.110",
});

#[tokio::test(flavor = "multi_thread")]
async fn loads_native_versions_and_stable_alias() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("claude_tool").await;
    let output = plugin.load_versions(LoadVersionsInput::default()).await;

    // 1.0.36 and earlier were only published as a Node.js package
    assert!(output.versions.iter().any(|v| v.to_string() == "1.0.37"));
    assert!(!output.versions.iter().any(|v| v.to_string() == "1.0.36"));
    assert!(output.aliases.contains_key("stable"));
}
