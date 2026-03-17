use std::collections::HashMap;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct NpmPackageSummary {
    #[serde(rename = "dist-tags")]
    pub dist_tags: HashMap<String, String>,
    pub versions: HashMap<String, NpmPackageManifest>
}

#[derive(Deserialize)]
pub struct NpmPackageManifest {
    pub version: String,
    pub dependencies: HashMap<String, String>,
    pub peer_dependencies: HashMap<String, String>,
    pub dist: NpmPackageDistMeta
}

#[derive(Deserialize)]
pub struct NpmPackageDistMeta {
    pub shasum: String,
    pub tarball: String,
}
