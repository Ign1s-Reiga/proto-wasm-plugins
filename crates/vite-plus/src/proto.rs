use crate::archive::decompress_tarball;
use extism_pdk::{host_fn, plugin_fn, Error, FnResult, Json};
use npm_registry_api::schema::NpmPackageSummary;
use npm_registry_api::{decode_sri, fetch_npm_registry, fetch_package_manifest};
use proto_pdk::*;
use starbase_utils::fs;
use std::collections::HashMap;
use std::path::PathBuf;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

#[plugin_fn]
pub fn register_tool(_: ()) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        default_version: Some(UnresolvedVersionSpec::parse("latest")?),
        name: "Vite+".to_string(),
        type_of: PluginType::CommandLine,
        minimum_proto_version: Some(Version::new(0, 50, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        self_upgrade_commands: vec!["vp".to_string(), "upgrade".to_string()],
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn download_prebuilt(Json(input): Json<DownloadPrebuiltInput>) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    check_supported_os_and_arch(
        "Vite+",
        &env,
        permutations! [
            HostOS::Windows => [HostArch::X64, HostArch::Arm64],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Linux => [HostArch::X64, HostArch::Arm64],
        ]
    )?;

    let manifest = fetch_package_manifest(get_package_url(&env), &input.context.version)?;

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some("package".to_string()),
        checksum: Some(Checksum::sha512(decode_sri(manifest.dist.integrity)?)),
        download_url: manifest.dist.tarball,
        ..DownloadPrebuiltOutput::default()
    }))
}

#[plugin_fn]
pub fn unpack_archive(Json(input): Json<UnpackArchiveInput>) -> FnResult<()> {
    let bin_dir = input.output_dir.join("bin");
    decompress_tarball(&input.input_file, &bin_dir)?;

    let bin_name = if get_host_environment()?.os.is_windows() { "vp.exe" } else { "vp" };
    // chmod 755 the binary
    fs::update_perms(bin_dir.join(bin_name), None)?;
    // Remove unnecessary package.json file
    fs::remove_file(bin_dir.join("package.json"))?;

    Ok(())
}

#[plugin_fn]
pub fn locate_executables(Json(input): Json<LocateExecutablesInput>) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let primary = ExecutableConfig {
        exe_path: Some(PathBuf::from(if env.os.is_windows() { "bin/vp.exe" } else { "bin/vp" })),
        primary: true,
        no_bin: true,
        shim_env_vars: Some(HashMap::from_iter([
            ("VITE_PLUS_HOME".to_string(), input.install_dir.real_path_string().ok_or_else(|| Error::msg("Couldn't get real path"))?)
        ])),
        ..ExecutableConfig::default()
    };

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([("vp".to_string(), primary)]),
        ..LocateExecutablesOutput::default()
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let env = get_host_environment()?;
    let rsp: NpmPackageSummary = fetch_npm_registry(get_package_url(&env))?;

    for item in rsp.versions.values() {
        output.versions.push(VersionSpec::parse(&item.version)?);
    }

    for (alias, version) in rsp.dist_tags {
        if alias == "latest" {
            output.latest = Some(UnresolvedVersionSpec::parse(&version)?);
        } else if alias == "alpha" {
            output.aliases.insert(alias, UnresolvedVersionSpec::parse(&version)?);
        }
    }

    Ok(Json(output))
}

fn get_package_url(env: &HostEnvironment) -> String {
    let os = match env.os {
        HostOS::Windows => "win32",
        HostOS::MacOS => "darwin",
        HostOS::Linux => "linux",
        _ => unreachable!(),
    };
    let arch = match env.arch {
        HostArch::X64 => "x64",
        HostArch::Arm64 => "arm64",
        _ => unreachable!(),
    };
    let crt = match env.os {
        HostOS::Windows => Some("msvc"),
        HostOS::Linux => Some("gnu"),
        _  => None
    };

    let platform_suffix = if let Some(c) = crt {
        format!("{os}-{arch}-{c}")
    } else {
        format!("{os}-{arch}")
    };
    format!("https://registry.npmjs.org/@voidzero-dev/vite-plus-cli-{platform_suffix}")
}
