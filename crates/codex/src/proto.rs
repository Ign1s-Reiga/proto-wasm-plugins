use extism_pdk::{host_fn, plugin_fn, FnResult, Json, Error};
use npm_registry_api::{decode_sri, fetch_package_manifest, find_package_manifest};
use proto_pdk::*;
use serde::Deserialize;
use starbase_utils::fs;
use std::collections::HashMap;

const GITHUB_REPO: &str = "openai/codex";
// Platform builds are published to npm as `@openai/codex@{version}-{platform}`.
// Using the registry avoids GitHub's unauthenticated API rate limit and always
// provides an integrity hash for the tarball.
const NPM_REGISTRY_URL: &str = "https://registry.npmjs.org/@openai/codex";

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

#[derive(Deserialize)]
struct GithubRelease {
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

fn npm_platform_str(env: &HostEnvironment) -> AnyResult<String> {
    let HostEnvironment { os, arch, .. } = env;

    let os_str = match os {
        HostOS::Windows => "win32",
        HostOS::MacOS => "darwin",
        HostOS::Linux => "linux",
        _ => return Err(Error::msg("Unsupported OS for Codex")),
    };
    let arch_str = match arch {
        HostArch::X64 => "x64",
        HostArch::Arm64 => "arm64",
        _ => return Err(Error::msg("Unsupported arch for Codex")),
    };
    Ok(format!("{os_str}-{arch_str}"))
}

fn rust_target_str(env: &HostEnvironment) -> AnyResult<String> {
    let HostEnvironment { os, arch, .. } = env;

    let arch_str = match arch {
        HostArch::X64 => "x86_64",
        HostArch::Arm64 => "aarch64",
        _ => return Err(Error::msg("Unsupported arch for Codex")),
    };
    Ok(match os {
        HostOS::Windows => format!("{arch_str}-pc-windows-msvc"),
        HostOS::MacOS => format!("{arch_str}-apple-darwin"),
        // install.sh uses musl for Linux, not gnu
        HostOS::Linux => format!("{arch_str}-unknown-linux-musl"),
        _ => return Err(Error::msg("Unsupported OS for Codex")),
    })
}

fn normalize_tag(tag: &str) -> &str {
    tag.strip_prefix("rust-v")
        .or_else(|| tag.strip_prefix("v"))
        .unwrap_or(tag)
}

#[plugin_fn]
pub fn register_tool(_: ()) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        default_version: Some(UnresolvedVersionSpec::parse("latest")?),
        name: "Codex".to_string(),
        type_of: PluginType::CommandLine,
        minimum_proto_version: Some(Version::new(0, 60, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags(format!("https://github.com/{GITHUB_REPO}"))?;

    let mut output = LoadVersionsOutput::default();
    for tag in &tags {
        if !tag.starts_with("rust-v") {
            continue;
        }
        if let Ok(spec) = VersionSpec::parse(normalize_tag(tag)) {
            output.versions.push(spec);
        }
    }

    let latest = fetch_package_manifest(NPM_REGISTRY_URL, "latest")?;
    output.latest = Some(UnresolvedVersionSpec::parse(&latest.version)?);

    Ok(Json(output))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    check_supported_os_and_arch(
        "Codex",
        &env,
        permutations! [
            HostOS::Windows => [HostArch::X64, HostArch::Arm64],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Linux => [HostArch::X64, HostArch::Arm64],
        ],
    )?;

    let version = &input.context.version;
    let platform = npm_platform_str(&env)?;

    let (download_url, checksum) = match find_package_manifest(NPM_REGISTRY_URL, format!("{version}-{platform}"))? {
        Some(manifest) => (
            manifest.dist.tarball,
            Checksum::sha512(decode_sri(manifest.dist.integrity)?),
        ),
        None => github_release_asset(version, &platform).map_err(|e| {
            Error::msg(format!(
                "Codex {version} for {platform} isn't published to npm, and the GitHub release fallback failed: {e}"
            ))
        })?,
    };

    // Both npm and GitHub tarballs wrap their contents in `package/`.
    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some("package".to_string()),
        checksum: Some(checksum),
        download_url,
        ..DownloadPrebuiltOutput::default()
    }))
}

// A few releases (e.g. 0.102.0) were only published as GitHub release assets,
// not to npm. The GitHub API is only queried for those, so the unauthenticated
// rate limit isn't hit in normal use.
fn github_release_asset(version: &VersionSpec, platform: &str) -> AnyResult<(String, Checksum)> {
    let asset_name = format!("codex-npm-{platform}-{version}.tgz");
    let release: GithubRelease = fetch_json(format!(
        "https://api.github.com/repos/{GITHUB_REPO}/releases/tags/rust-v{version}"
    ))?;
    let asset = release
        .assets
        .into_iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| Error::msg(format!("Release rust-v{version} has no {asset_name} asset")))?;
    let digest = asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .ok_or_else(|| Error::msg(format!("Asset {asset_name} has no sha256 digest")))?;

    Ok((asset.browser_download_url, Checksum::sha256(digest.to_string())))
}

// The tarball has the structure:
//   package/vendor/{target}/codex/codex[.exe]
//   package/vendor/{target}/path/rg[.exe]
//   package/vendor/{target}/codex-resources/bwrap  (Linux only)
//
// The codex binary expects its resources adjacent to itself:
//   codex[.exe]
//   codex-resources/rg[.exe]
//   codex-resources/bwrap  (Linux)
//   codex-resources/codex-command-runner.exe  (Windows)
//   codex-resources/codex-windows-sandbox-setup.exe  (Windows)
//
// unpack_archive is called during the extraction step (before the executable
// is verified), so using it instead of post_install ensures the restructuring
// is done before proto checks that the binary exists at the locate_executables
// path.
#[plugin_fn]
pub fn unpack_archive(Json(input): Json<UnpackArchiveInput>) -> FnResult<()> {
    let env = get_host_environment()?;
    let target = rust_target_str(&env)?;

    let archive = input.input_file
        .to_real_path()?
        .ok_or_else(|| Error::msg("Cannot resolve archive path"))?
        .to_string_lossy()
        .to_string();
    let out = input.output_dir
        .to_real_path()?
        .ok_or_else(|| Error::msg("Cannot resolve output dir"))?
        .to_string_lossy()
        .to_string();

    fs::create_dir_all(&input.output_dir)?;

    // Strip the leading "package/" component so vendor/{target}/... lands
    // directly under output_dir.
    let mut args = vec![
        "xzf".to_string(),
        archive,
        "-C".to_string(),
        out,
        "--strip-components=1".to_string(),
    ];

    // GNU tar on Windows misinterprets drive-letter paths (C:\...) as remote
    // host specifications without this flag.
    if env.os.is_windows() {
        let v = exec_captured("tar", vec!["--version".to_string()])?;
        if v.stdout.contains("GNU tar") {
            args.push("--force-local".to_string());
        }
    }

    exec_streamed("tar", args)?;

    // Restructure from vendor/{target}/ to the flat layout codex expects.
    let vendor = input.output_dir.join(format!("vendor/{target}"));
    let resources = input.output_dir.join("codex-resources");
    fs::create_dir_all(&resources)?;

    if env.os.is_windows() {
        for (src_rel, dst_rel) in [
            ("codex/codex.exe", "codex.exe"),
            ("bin/codex.exe", "codex.exe"),
            ("codex/codex-command-runner.exe", "codex-resources/codex-command-runner.exe"),
            ("codex-resources/codex-command-runner.exe", "codex-resources/codex-command-runner.exe"),
            ("codex/codex-windows-sandbox-setup.exe", "codex-resources/codex-windows-sandbox-setup.exe"),
            ("codex-resources/codex-windows-sandbox-setup.exe", "codex-resources/codex-windows-sandbox-setup.exe"),
            ("path/rg.exe", "codex-resources/rg.exe"),
            ("codex-path/rg.exe", "codex-resources/rg.exe"),
        ] {
            let src = vendor.join(src_rel);
            if src.exists() {
                fs::rename(src, input.output_dir.join(dst_rel))?;
            }
        }
    } else {
        let src_bin = vendor.join("codex/codex");
        let dst_bin = input.output_dir.join("codex");
        if src_bin.exists() {
            fs::rename(src_bin, &dst_bin)?;
        }
        fs::update_perms(&dst_bin, None)?;

        let src_rg = vendor.join("path/rg");
        let dst_rg = resources.join("rg");
        if src_rg.exists() {
            fs::rename(src_rg, &dst_rg)?;
        }
        fs::update_perms(&dst_rg, None)?;

        // bwrap is Linux-only (bubblewrap sandbox)
        let src_bwrap = vendor.join("codex-resources/bwrap");
        let dst_bwrap = resources.join("bwrap");
        if src_bwrap.exists() {
            fs::rename(src_bwrap, &dst_bwrap)?;
            fs::update_perms(&dst_bwrap, None)?;
        }
    }

    let vendor_root = input.output_dir.join("vendor");
    if vendor_root.exists() {
        fs::remove_dir_all(vendor_root)?;
    }

    Ok(())
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    // post_install moves the binary to the root of the tool directory
    let bin_name = if env.os.is_windows() { "codex.exe" } else { "codex" };

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([(
            "codex".to_string(),
            ExecutableConfig::new_primary(bin_name),
        )]),
        ..LocateExecutablesOutput::default()
    }))
}
