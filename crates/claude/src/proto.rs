use extism_pdk::{plugin_fn, Error, FnResult, Json};
use npm_registry_api::{decode_sri, fetch_npm_registry, find_package_manifest, schema::NpmPackageSummary};
use proto_pdk::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

// The launcher package; its versions and dist-tags drive version resolution.
// Since 2.1.110 the native builds are also published to npm as
// `@anthropic-ai/claude-code-{platform}`, which are tried first since they are
// compressed and come with an integrity hash. Older builds are only hosted on
// the download server of the official installer (see `native_release`).
const NPM_REGISTRY_URL: &str = "https://registry.npmjs.org/@anthropic-ai/claude-code";
const DOWNLOAD_BASE_URL: &str = "https://downloads.claude.ai/claude-code-releases";

#[derive(Deserialize)]
struct NativeManifest {
    platforms: HashMap<String, NativePlatform>,
}

#[derive(Deserialize)]
struct NativePlatform {
    checksum: String,
}

fn platform_str(env: &HostEnvironment) -> AnyResult<String> {
    let HostEnvironment { os, arch, libc, .. } = env;

    let os_str = match os {
        HostOS::Windows => "win32",
        HostOS::MacOS => "darwin",
        HostOS::Linux => "linux",
        _ => return Err(Error::msg(format!("Unsupported OS for Claude Code: {os}"))),
    };
    let arch_str = match arch {
        HostArch::X64 => "x64",
        HostArch::Arm64 => "arm64",
        _ => return Err(Error::msg(format!("Unsupported arch for Claude Code: {arch}"))),
    };
    // Alpine and other musl distros have their own builds
    let libc_suffix = if matches!(os, HostOS::Linux) && matches!(libc, HostLibc::Musl) {
        "-musl"
    } else {
        ""
    };
    Ok(format!("{os_str}-{arch_str}{libc_suffix}"))
}

#[plugin_fn]
pub fn register_tool(_: ()) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        default_version: Some(UnresolvedVersionSpec::parse("latest")?),
        name: "Claude Code".to_string(),
        type_of: PluginType::CommandLine,
        minimum_proto_version: Some(Version::new(0, 60, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        self_upgrade_commands: vec!["update".to_string(), "upgrade".to_string()],
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let rsp: NpmPackageSummary = fetch_npm_registry(NPM_REGISTRY_URL)?;
    // Earlier releases were only published as a Node.js package.
    let min_native_version = Version::new(1, 0, 37);

    for item in rsp.versions.values() {
        let spec = VersionSpec::parse(&item.version)?;
        if spec.as_version().is_some_and(|v| *v >= min_native_version) {
            output.versions.push(spec);
        }
    }

    for (alias, version) in rsp.dist_tags {
        if alias == "latest" {
            output.latest = Some(UnresolvedVersionSpec::parse(&version)?);
        } else if alias == "stable" {
            output.aliases.insert(alias, UnresolvedVersionSpec::parse(&version)?);
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    check_supported_os_and_arch(
        "Claude Code",
        env,
        permutations! [
            HostOS::Windows => [HostArch::X64, HostArch::Arm64],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Linux => [HostArch::X64, HostArch::Arm64],
        ],
    )?;

    let version = &input.context.version;
    let platform = platform_str(env)?;

    let output = match find_package_manifest(format!("{NPM_REGISTRY_URL}-{platform}"), version)? {
        // The tarball wraps the binary in `package/`.
        Some(manifest) => DownloadPrebuiltOutput {
            archive_prefix: Some("package".to_string()),
            checksum: Some(Checksum::sha512(decode_sri(manifest.dist.integrity)?)),
            download_url: manifest.dist.tarball,
            ..DownloadPrebuiltOutput::default()
        },
        None => native_release(env, version, &platform).map_err(|e| {
            Error::msg(format!(
                "Claude Code {version} for {platform} isn't published to npm, and the native release fallback failed: {e}"
            ))
        })?,
    };

    Ok(Json(output))
}

// Releases before 2.1.110 are only available from the download server used by
// the official install script, as a bare executable listed with its SHA-256
// in the release manifest.
fn native_release(
    env: &HostEnvironment,
    version: &VersionSpec,
    platform: &str,
) -> AnyResult<DownloadPrebuiltOutput> {
    let manifest: NativeManifest = fetch_json(format!("{DOWNLOAD_BASE_URL}/{version}/manifest.json"))?;
    let checksum = manifest
        .platforms
        .get(platform)
        .map(|p| p.checksum.clone())
        .ok_or_else(|| Error::msg(format!("Release {version} has no build for {platform}")))?;

    Ok(DownloadPrebuiltOutput {
        checksum: Some(Checksum::sha256(checksum)),
        download_url: format!(
            "{DOWNLOAD_BASE_URL}/{version}/{platform}/{}",
            env.os.get_exe_name("claude")
        ),
        ..DownloadPrebuiltOutput::default()
    })
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let mut exe_path = env.os.get_exe_name("claude");

    // Builds from the download server aren't archives, so proto moves them
    // into the tool directory named after the plugin ID instead.
    if !input.install_dir.join(&exe_path).exists() {
        let moved_exe_path = env.os.get_exe_name(get_plugin_id()?.as_str());
        if input.install_dir.join(&moved_exe_path).exists() {
            exe_path = moved_exe_path;
        }
    }

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([(
            "claude".to_string(),
            ExecutableConfig {
                exe_path: Some(PathBuf::from(exe_path)),
                primary: true,
                // Versions are managed by proto, so keep Claude Code from
                // installing updates of its own in the background.
                shim_env_vars: Some(HashMap::from_iter([(
                    "DISABLE_AUTOUPDATER".to_string(),
                    "1".to_string(),
                )])),
                ..ExecutableConfig::default()
            },
        )]),
        ..LocateExecutablesOutput::default()
    }))
}
