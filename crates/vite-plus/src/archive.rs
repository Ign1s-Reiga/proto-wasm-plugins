use extism_pdk::Error;
use proto_pdk::{exec_captured, exec_streamed, get_host_environment, AnyResult, VirtualPath, VirtualPathExt};
use starbase_utils::fs;

pub fn decompress_tarball(input_file: &VirtualPath, output_dir: &VirtualPath) -> AnyResult<()> {
    let input_file_path = input_file.to_real_path()?
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| Error::msg("Couldn't get tgz path"))?;
    let output_dir_path = output_dir.to_real_path()?
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| Error::msg("Couldn't get output dir path"))?;

    fs::create_dir_all(&output_dir)?;

    let mut args = vec![
        "xzvf".to_string(),
        input_file_path,
        "-C".to_string(),
        output_dir_path,
        "--strip-components=1".to_string()
    ];
    // GNU tar on Windows misinterprets drive-letter paths (C:\...) as remote
    // host specifications without this flag. Other platforms and tar
    // implementations (bsdtar, BusyBox) don't need the variant check.
    if get_host_environment()?.os.is_windows() && is_gnu_tar()? {
        args.push("--force-local".to_string());
    }

    exec_streamed("tar", args)?;
    // debug!("Unpack Command Result: exit_code={}, stderr={}", result.exit_code, result.stderr);

    Ok(())
}

fn is_gnu_tar() -> AnyResult<bool> {
    let result = exec_captured("tar", vec!["--version"])?;
    Ok(result.stdout.contains("GNU tar"))
}
