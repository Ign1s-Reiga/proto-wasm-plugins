mod versions {
    use proto_pdk_test_utils::*;

    generate_resolve_versions_tests!("wrangler-test", {
        "3.114" => "3.114.15",
        "4.54.0" => "4.54.0",
        "4" => "4.54.0",
    });
}
