use proto_pdk_test_utils::*;

// Since 0.133.0 codex ships a package described by codex-package.json. It has
// to stay intact, or codex refuses to start its app-server daemon with "this
// CLI has no complete local package".
#[tokio::test(flavor = "multi_thread")]
async fn keeps_package_layout_intact() {
    let sandbox = create_empty_proto_sandbox();
    let mut plugin = sandbox.create_plugin("codex_tool").await;
    let mut spec = ToolSpec::parse("0.163.0").unwrap();

    flow::manage::Manager::new(&mut plugin.tool)
        .install(&mut spec, flow::install::InstallOptions::default())
        .await
        .unwrap();

    let exe = flow::locate::Locator::new(&plugin.tool, &spec)
        .locate_exe_file()
        .await
        .unwrap();
    let bin_dir = exe.parent().unwrap();
    let package_dir = bin_dir.parent().unwrap();

    assert!(exe.exists());
    assert_eq!(bin_dir.file_name().unwrap(), "bin");
    assert!(package_dir.join("codex-package.json").exists());
    assert!(package_dir.join("codex-path").is_dir());
    assert!(package_dir.join("codex-resources").is_dir());
}
