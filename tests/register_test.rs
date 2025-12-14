mod register {
    use proto_pdk_test_utils::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("wrangler-test").await;

        let output = plugin.register_tool(RegisterToolInput { id: Id::raw("wrangler-test") }).await;

        assert_eq!(output.name, "Wrangler");
        assert_eq!(output.type_of, PluginType::CommandLine);
        assert_eq!(
            output.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }
}
