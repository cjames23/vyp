//! Dynamic index scoping.
//!
//! A [`MetadataProvider`](crate::MetadataProvider) backed by a named, explicit
//! index can be restricted to only serve a subset of packages. Unlike a static
//! allow-list, an [`IndexScope`] is consulted live during resolution so that
//! membership can grow as the transitive closure of the index's declared root
//! packages is discovered.
//!
//! The canonical implementor is `vyp_core::IndexRouter`'s per-index view, which
//! routes a package to a named index only when it is reachable *exclusively*
//! through that index's declared roots (the "default index wins on overlap"
//! policy).

use std::fmt::Debug;

/// Decides whether a named index is allowed to serve a given package.
///
/// Implementations must be cheap to call and safe to query from the solver
/// thread. `package` is the normalized (PEP 503) package name.
pub trait IndexScope: Send + Sync + Debug {
    /// Returns `true` if the index this scope belongs to may serve `package`.
    fn allows(&self, package: &str) -> bool;
}
