use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use extism_pdk::{host_fn, json, Error, Json};
use proto_pdk::{fetch_text, populate_send_request_output, send_request, AnyResult, SendRequestInput, SendRequestOutput};
use crate::schema::{NpmPackageManifest, NpmPackageSummary};
use std::fmt::Display;

pub mod schema;

#[host_fn]
extern "ExtismHost" {
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
}

pub fn fetch_npm_registry<S: AsRef<str>>(url: S) -> AnyResult<NpmPackageSummary> {
    Ok(json::from_str::<NpmPackageSummary>(&fetch_text(url)?)?)
}

pub fn fetch_package_manifest<S: AsRef<str>, V: Display>(url: S, version: V) -> AnyResult<NpmPackageManifest> {
    find_package_manifest(url, &version)?
        .ok_or_else(|| Error::msg(format!("Package manifest for version {version} not found")))
}

/// Like [`fetch_package_manifest`], but returns `None` instead of failing when
/// the registry doesn't have the requested version, so callers can tell a
/// missing version apart from network or registry errors.
pub fn find_package_manifest<S: AsRef<str>, V: Display>(url: S, version: V) -> AnyResult<Option<NpmPackageManifest>> {
    let response = request(format!("{}/{}", url.as_ref(), version))
        .map_err(|e| Error::msg(format!("Failed to fetch package manifest for version {version}: {e}")))?;

    if response.status == 404 {
        return Ok(None);
    }
    if !(200..300).contains(&response.status) {
        return Err(Error::msg(format!(
            "Failed to fetch package manifest for version {version}: HTTP {}",
            response.status
        )));
    }

    response
        .json::<NpmPackageManifest>()
        .map(Some)
        .map_err(|e| Error::msg(format!("Invalid package manifest for version {version}: {e}")))
}

// proto_pdk's fetch helpers turn every non-2xx status into an opaque error,
// so send the request directly to inspect the status code.
fn request(url: String) -> AnyResult<SendRequestOutput> {
    Ok(send_request!(url))
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
