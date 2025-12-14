use std::collections::HashMap;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct NpmPackage {
    #[serde(rename = "dist-tags")]
    pub dist_tags: HashMap<String, String>,
    pub versions: HashMap<String, NpmPackageVersion>
}

#[derive(Deserialize)]
pub struct NpmPackageVersion {
    pub version: String
}

#[derive(Deserialize)]
pub struct PrototoolsConfig {
    pub wrangler: String,
}

#[derive(Deserialize)]
pub struct PackageJson {
    pub dependencies: HashMap<String, String>,
}
