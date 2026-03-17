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
