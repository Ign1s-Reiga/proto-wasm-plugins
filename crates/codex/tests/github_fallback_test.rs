use proto_pdk_test_utils::*;

// 0.102.0 was only published as a GitHub release asset, not to npm,
// so this exercises the GitHub release fallback of download_prebuilt.
generate_download_install_tests!("codex_tool", "0.102.0");
