use vyp_api::DependencyOverride;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A `vyp-overrides.toml` file that libraries ship alongside their
/// distribution. Contains transitive dependency overrides, enabling the
/// "middle of the diamond" to propagate override decisions to consumers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverridesExportFile {
    #[serde(rename = "overrides-version")]
    pub version: String,
    #[serde(rename = "created-by")]
    pub created_by: String,
    /// The package that generated this file.
    pub package: String,
    /// Package version at export time.
    pub package_version: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<OverrideEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverrideEntry {
    pub package: String,
    pub constraint: String,
    #[serde(default)]
    pub transitive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub propagation_path: Vec<String>,
}

impl OverridesExportFile {
    pub fn from_overrides(
        package: &str,
        package_version: &str,
        dep_overrides: &[DependencyOverride],
    ) -> Self {
        let overrides = dep_overrides
            .iter()
            .filter(|o| o.transitive)
            .map(|o| OverrideEntry {
                package: o.package.clone(),
                constraint: o.constraint.clone(),
                transitive: o.transitive,
                reason: o.reason.clone(),
                origin: o.origin.clone(),
                propagation_path: o.propagation_path.clone(),
            })
            .collect();

        OverridesExportFile {
            version: "4.0".to_string(),
            created_by: format!("vyp {}", env!("CARGO_PKG_VERSION")),
            package: package.to_string(),
            package_version: package_version.to_string(),
            overrides,
        }
    }

    pub fn write_to_file(&self, path: &Path) -> miette::Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| miette::miette!("Failed to serialize overrides export: {}", e))?;
        std::fs::write(path, content)
            .map_err(|e| miette::miette!("Failed to write overrides export: {}", e))?;
        Ok(())
    }
}
