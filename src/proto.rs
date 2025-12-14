use crate::{NpmPackage, PrototoolsConfig, BASH_SHIMS_CONTENT, CMD_SHIMS_CONTENT};
use extism_pdk::{host_fn, json, plugin_fn, FnResult, Json};
use proto_pdk::{exec_command, fetch_text, get_host_environment, AnyResult, DetectVersionInput, DetectVersionOutput, DownloadPrebuiltInput, DownloadPrebuiltOutput, ExecCommandInput, ExecCommandOutput, ExecutableConfig, HostEnvironment, InstallHook, LoadVersionsInput, LoadVersionsOutput, LocateExecutablesInput, LocateExecutablesOutput, ParseVersionFileInput, ParseVersionFileOutput, PluginType, RegisterToolOutput, ResolveVersionInput, ResolveVersionOutput, UnresolvedVersionSpec, Version, VersionSpec, VirtualPath};
use starbase_utils::fs;
use std::collections::HashMap;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

#[plugin_fn]
pub fn register_tool(_: ()) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        name: "Wrangler".to_string(),
        type_of: PluginType::CommandLine,
        minimum_proto_version: Some(Version::new(0, 50, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        requires: vec!["npm".to_string()],
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn download_prebuilt(Json(input): Json<DownloadPrebuiltInput>) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let version = input.context.version;
    let filename = format!("wrangler-{version}.tgz");

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some("package".to_string()),
        download_url: format!("https://registry.npmjs.org/wrangler/-/{filename}"),
        ..DownloadPrebuiltOutput::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(Json(input): Json<LocateExecutablesInput>) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let filename = if env.os.is_windows() { "wrangler.cmd" } else { "wrangler" };
    if !input.install_dir.join("shims").exists() {
        create_shim(&env, &input.install_dir, filename)?;
    }

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([("wrangler".to_string(), ExecutableConfig::new_primary(format!("shims/{filename}")))]),
        ..LocateExecutablesOutput::default()
    }))
}

#[plugin_fn]
pub fn post_install(Json(input): Json<InstallHook>) -> FnResult<()> {
    exec_command!(input, ExecCommandInput {
        command: "npm".to_string(),
        args: vec!["install".to_string(), "--omit=dev".to_string()],
        cwd: Some(input.context.tool_dir),
        stream: true,
        ..ExecCommandInput::default()
    });

    Ok(())
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let rsp: NpmPackage = json::from_str(&fetch_text("https://registry.npmjs.org/wrangler")?)?;

    for item in rsp.versions.values() {
        output.versions.push(VersionSpec::parse(&item.version)?);
    }

    for (alias, version) in rsp.dist_tags {
        if alias == "latest" {
            output.latest = Some(UnresolvedVersionSpec::parse(&version)?);
        }
    }

    Ok(Json(output))
}

// TODO: resolve aliases
#[plugin_fn]
pub fn resolve_version(Json(_): Json<ResolveVersionInput>) -> FnResult<Json<ResolveVersionOutput>> {
    Ok(Json(ResolveVersionOutput::default()))
}

#[plugin_fn]
pub fn detect_version_files(_: Json<DetectVersionInput>) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".prototools".to_string()],
        ..DetectVersionOutput::default()
    }))
}

#[plugin_fn]
pub fn parse_version_file(Json(input): Json<ParseVersionFileInput>) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut version = None;

    if input.file == ".prototools" {
        // parse as toml
        version = match toml::from_str::<PrototoolsConfig>(input.content.as_str()) {
            Ok(prototools) => UnresolvedVersionSpec::parse(prototools.wrangler).ok(),
            Err(_) => None,
        };
    } else {
        version = UnresolvedVersionSpec::parse(input.content.trim()).ok();
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

fn create_shim(env: &HostEnvironment, install_dir: &VirtualPath, filename: &str) -> AnyResult<()> {
    fs::write_file(
        install_dir.join("shims").join(filename),
        if env.os.is_windows() { CMD_SHIMS_CONTENT } else { BASH_SHIMS_CONTENT }
    )?;

    Ok(())
}
