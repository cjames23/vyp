use serde::{Deserialize, Serialize};
use std::fmt;

/// A dependency override that forces a package to resolve within a specific
/// constraint. When `transitive` is true, the override propagates to consumers
/// via `vyp-overrides.toml`.
///
/// Replaces the old `OverrideStack`/`OverrideRule` layering system and the
/// separate `VersionPinOverride` -- an exact pin is just `constraint = "==1.24.0"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyOverride {
    /// Package to override.
    pub package: String,
    /// PEP 440 version specifier, e.g. `">=1.26,<2"` or `"==1.24.0"`.
    pub constraint: String,
    /// Propagate to consumers in the dependency graph.
    #[serde(default)]
    pub transitive: bool,
    /// Why this override exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Where this override originated (for inherited overrides).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// Chain of packages through which this was inherited.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub propagation_path: Vec<String>,
}

impl DependencyOverride {
    pub fn new(package: impl Into<String>, constraint: impl Into<String>) -> Self {
        Self {
            package: package.into(),
            constraint: constraint.into(),
            transitive: false,
            reason: None,
            origin: None,
            propagation_path: Vec::new(),
        }
    }

    pub fn with_transitive(mut self, transitive: bool) -> Self {
        self.transitive = transitive;
        self
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Returns true if this is an exact-version pin (constraint starts with `==`).
    pub fn is_exact_pin(&self) -> bool {
        let trimmed = self.constraint.trim();
        trimmed.starts_with("==") && !trimmed.starts_with("===")
    }

    /// For exact pins, extract the version string after `==`.
    pub fn pinned_version(&self) -> Option<&str> {
        if self.is_exact_pin() {
            Some(self.constraint.trim().trim_start_matches("==").trim())
        } else {
            None
        }
    }

    /// Create an inherited copy extending the propagation path.
    pub fn inherit_through(&self, package_name: &str) -> Self {
        let mut inherited = self.clone();
        if inherited.origin.is_none() {
            inherited.origin = Some(package_name.to_string());
        }
        inherited.propagation_path.push(package_name.to_string());
        inherited
    }
}

impl fmt::Display for DependencyOverride {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.package, self.constraint)?;
        if self.transitive {
            write!(f, " (transitive)")?;
        }
        if let Some(reason) = &self.reason {
            write!(f, " -- {}", reason)?;
        }
        if let Some(origin) = &self.origin {
            write!(f, " [from {}]", origin)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_pin_detection() {
        let pin = DependencyOverride::new("numpy", "==1.24.0");
        assert!(pin.is_exact_pin());
        assert_eq!(pin.pinned_version(), Some("1.24.0"));

        let range = DependencyOverride::new("numpy", ">=1.26,<2");
        assert!(!range.is_exact_pin());
        assert_eq!(range.pinned_version(), None);
    }

    #[test]
    fn test_inherit_through() {
        let original = DependencyOverride::new("numpy", "==1.24.0").with_transitive(true);
        let inherited = original.inherit_through("pkg-a");
        assert_eq!(inherited.origin.as_deref(), Some("pkg-a"));
        assert_eq!(inherited.propagation_path, vec!["pkg-a"]);

        let double = inherited.inherit_through("pkg-b");
        assert_eq!(double.origin.as_deref(), Some("pkg-a"));
        assert_eq!(double.propagation_path, vec!["pkg-a", "pkg-b"]);
    }

    #[test]
    fn test_display() {
        let o = DependencyOverride::new("numpy", ">=1.26,<2")
            .with_transitive(true)
            .with_reason("security fix");
        let s = o.to_string();
        assert!(s.contains("numpy"));
        assert!(s.contains(">=1.26,<2"));
        assert!(s.contains("(transitive)"));
        assert!(s.contains("security fix"));
    }
}
