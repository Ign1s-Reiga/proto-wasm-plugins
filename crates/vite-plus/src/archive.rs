use extism_pdk::Error;
use proto_pdk::{exec_captured, exec_streamed, get_host_environment, AnyResult, VirtualPath};
use starbase_utils::fs;

#[derive(PartialEq)]
enum TarVariant {
    GNU,
    BSD,
}

pub fn decompress_tarball(input_file: &VirtualPath, output_dir: &VirtualPath) -> AnyResult<()> {
    let input_file_path = input_file.real_path_string()
        .ok_or_else(|| Error::msg("Couldn't get tgz path"))?;
    let output_dir_path = output_dir.real_path()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| Error::msg("Couldn't get output dir path"))?;

    fs::create_dir_all(&output_dir)?;

    let variant = check_tar_variant()?;
    let is_windows = get_host_environment()?.os.is_windows();
    let mut args = vec![
        "xzvf".to_string(),
        input_file_path,
        format!("-C {output_dir_path}"),
        "--strip-components=1".to_string()
    ];
    if variant == TarVariant::GNU && is_windows { args.push("--force-local".to_string()) }

    exec_streamed("tar", args)?;
    // debug!("Unpack Command Result: exit_code={}, stderr={}", result.exit_code, result.stderr);

    Ok(())
}

fn check_tar_variant() -> AnyResult<TarVariant> {
    let result = exec_captured("tar", vec!["--version"])?;
    match result.stdout {
        s if s.contains("GNU tar") => Ok(TarVariant::GNU),
        s if s.contains("bsdtar") => Ok(TarVariant::BSD),
        _ => Err(Error::msg("Failed to execute tar.")),
    }
}
