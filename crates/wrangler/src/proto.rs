use crate::{BASH_SHIMS_CONTENT, CMD_SHIMS_CONTENT};
use extism_pdk::{host_fn, plugin_fn, FnResult, Json};
use npm_registry_api::{decode_sri, fetch_npm_registry, find_package_with_version_spec, schema::NpmPackageSummary};
use proto_pdk::*;
use starbase_utils::fs;
use std::collections::HashMap;

const NPM_REGISTRY_URL: &'static str = "https://registry.npmjs.org/wrangler";

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

#[plugin_fn]
pub fn register_tool(_: ()) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        default_version: Some(UnresolvedVersionSpec::parse("latest")?),
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
    let package = find_package_with_version_spec(NPM_REGISTRY_URL, &version)?;

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some("package".to_string()),
        checksum: Some(Checksum::sha512(decode_sri(package.dist.integrity)?)),
        download_url: package.dist.tarball,
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
    let dist_meta = find_package_with_version_spec(NPM_REGISTRY_URL, &input.context.version)?;
    let need_install: HashMap<String, String> = dist_meta.dependencies
        .into_iter()
        .chain(dist_meta.peer_dependencies.unwrap_or_default())
        .collect();
    // remove package.json before install packages
    fs::remove_file(input.context.tool_dir.join("package.json"))?;
    let tool_dir_real_path = input.context.tool_dir
        .real_path()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let mut args = vec![
        "install".to_string(),
        "--prefix".to_string(),
        tool_dir_real_path,
    ];

    args.extend(need_install.into_iter().map(|(k, v)| format!("{k}@{v}")));

    exec_command!(input, ExecCommandInput {
        command: "npm".to_string(),
        args,
        stream: true,
        ..ExecCommandInput::default()
    });

    Ok(())
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let rsp: NpmPackageSummary = fetch_npm_registry(NPM_REGISTRY_URL)?;

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

fn create_shim(env: &HostEnvironment, install_dir: &VirtualPath, filename: &str) -> AnyResult<()> {
    fs::write_file(
        install_dir.join("shims").join(filename),
        if env.os.is_windows() { CMD_SHIMS_CONTENT } else { BASH_SHIMS_CONTENT }
    )?;

    Ok(())
}
