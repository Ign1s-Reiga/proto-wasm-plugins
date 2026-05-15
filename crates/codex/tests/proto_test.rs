use proto_pdk_test_utils::*;

generate_download_install_tests!("codex_tool", "0.130.0");

generate_resolve_versions_tests!("codex_tool", {
    "latest" => "0.130.0",
});
