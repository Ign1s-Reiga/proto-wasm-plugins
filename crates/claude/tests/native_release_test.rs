use proto_pdk_test_utils::*;

// Releases before 2.1.110 aren't published to npm as platform packages, so
// this exercises the downloads.claude.ai fallback of download_prebuilt.
generate_download_install_tests!("claude_tool", "2.0.77");
