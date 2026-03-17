use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use extism_pdk::{json, Error};
use proto_pdk::{fetch_text, AnyResult, VersionSpec};
use crate::schema::{NpmPackageManifest, NpmPackageSummary};

pub mod schema;

pub fn fetch_npm_registry(url: &'static str) -> AnyResult<NpmPackageSummary> {
    let rsp: NpmPackageSummary = json::from_str(&fetch_text(url)?)?;
    Ok(rsp)
}

pub fn find_package_with_version_spec(url: &'static str, version: &VersionSpec) -> AnyResult<NpmPackageManifest> {
    let mut summary = fetch_npm_registry(url)?;
    let version_string = match version {
        VersionSpec::Alias(alias) => summary
            .dist_tags
            .get(alias.as_str())
            .cloned()
            .ok_or_else(|| Error::msg(format!("Unknown alias {alias}")))?,
        _ => version.to_string(),
    };

    if let Some(package) = summary.versions.remove(&version_string) {
        Ok(package)
    } else {
        Err(Error::msg(format!(
            "No package found matching the requested version: {version_string}"
        )))
    }
}

pub fn decode_sri(sri: String) -> AnyResult<String> {
    let (_, encoded_hash) = sri.split_once('-').ok_or_else(|| Error::msg("Invalid SRI format, expected algorithm-hash"))?;
    let decoded_bytes = BASE64_STANDARD.decode(encoded_hash)?;
    let mut hex_string = String::with_capacity(decoded_bytes.len() * 2);

    for b in decoded_bytes {
        use std::fmt::Write;
        write!(hex_string, "{:02x}", b).expect("Writing to String should not fail");
    }
    Ok(hex_string)
}
