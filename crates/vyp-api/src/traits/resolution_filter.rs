use crate::types::package::VypPackage;
use crate::types::version::VypVersion;
use std::fmt::Debug;

/// A candidate version for a package, possibly annotated with filter metadata.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub package: VypPackage,
    pub version: VypVersion,
    /// If true, this candidate has been marked for exclusion by a filter.
    pub excluded: bool,
    /// Reason for exclusion, if any.
    pub exclusion_reason: Option<String>,
}

impl Candidate {
    pub fn new(package: VypPackage, version: VypVersion) -> Self {
        Self {
            package,
            version,
            excluded: false,
            exclusion_reason: None,
        }
    }

    pub fn exclude(mut self, reason: impl Into<String>) -> Self {
        self.excluded = true;
        self.exclusion_reason = Some(reason.into());
        self
    }
}

/// Pre-filters or re-ranks version candidates before PubGrub processes them.
///
/// Use cases include: corporate allow/deny lists, license filtering,
/// vulnerability exclusion, and preferred-source pinning.
pub trait ResolutionFilter: Send + Sync + Debug {
    /// Human-readable name of this filter.
    fn name(&self) -> &str;

    /// Priority order (higher = applied first).
    fn priority(&self) -> i32;

    /// Filter a list of candidates for a package.
    /// Implementations should mark unwanted candidates as excluded
    /// rather than removing them, to preserve provenance information.
    fn filter(&self, candidates: &mut Vec<Candidate>);
}
