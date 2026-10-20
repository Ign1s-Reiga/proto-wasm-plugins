use proto_pdk_test_utils::*;

generate_download_install_tests!("codex_tool", "0.130.0");

// `latest` resolution is covered by the generated `resolves_latest_alias` test;
// pin to an old release here so the expectation doesn't change over time.
generate_resolve_versions_tests!("codex_tool", {
    "0.130" => "0.130.0",
    "0.130.0" => "0.130.0",
});
