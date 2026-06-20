use serde::{Deserialize, Serialize};
use std::fmt;

/// Declares a set of packages that are interchangeable.
///
/// Only one package from the set will be installed.
/// A virtual capability name ties them together in the resolver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstitutionSet {
    /// The virtual capability name (e.g., "opencv").
    pub provides: String,
    /// The concrete packages that can fulfill this capability.
    pub packages: Vec<String>,
    /// The preferred package to select by default.
    pub prefer: Option<String>,
}

impl SubstitutionSet {
    pub fn new(provides: impl Into<String>, packages: Vec<String>) -> Self {
        Self {
            provides: provides.into(),
            packages,
            prefer: None,
        }
    }

    pub fn with_preference(mut self, preferred: impl Into<String>) -> Self {
        self.prefer = Some(preferred.into());
        self
    }

    pub fn contains(&self, package_name: &str) -> bool {
        self.packages.iter().any(|p| p == package_name)
    }
}

impl fmt::Display for SubstitutionSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "substitution '{}': [{}]",
            self.provides,
            self.packages.join(", ")
        )?;
        if let Some(pref) = &self.prefer {
            write!(f, " (prefer: {})", pref)?;
        }
        Ok(())
    }
}
