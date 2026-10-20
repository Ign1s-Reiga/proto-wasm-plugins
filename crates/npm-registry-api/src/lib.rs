use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use extism_pdk::{json, Error};
use proto_pdk::{fetch_text, AnyResult};
use crate::schema::{NpmPackageManifest, NpmPackageSummary};
use std::fmt::Display;

pub mod schema;

pub fn fetch_npm_registry<S: AsRef<str>>(url: S) -> AnyResult<NpmPackageSummary> {
    Ok(json::from_str::<NpmPackageSummary>(&fetch_text(url)?)?)
}

pub fn fetch_package_manifest<S: AsRef<str>, V: Display>(url: S, version: V) -> AnyResult<NpmPackageManifest> {
    let text = fetch_text(format!("{}/{}", url.as_ref(), version))
        .map_err(|e| Error::msg(format!("Failed to fetch package manifest for version {version}: {e}")))?;
    json::from_str::<NpmPackageManifest>(&text)
        .map_err(|e| Error::msg(format!("Invalid package manifest for version {version}: {e}")))
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
