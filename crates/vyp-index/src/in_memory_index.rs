use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use vyp_api::{VypVersion, Requirement};
use vyp_api::traits::metadata_provider::PackageMetadata;

/// Thread-safe in-memory index that bridges the solver thread and the
/// background fetcher. The solver calls `wait_blocking` to get data, and
/// the fetcher calls `set_*` to populate results.
///
/// Slots are heap-allocated behind `Arc` so that references remain valid
/// even when the DashMap rehashes and relocates its internal storage.
pub struct InMemoryIndex {
    versions: DashMap<String, Arc<Slot<VersionsResult>>>,
    metadata: DashMap<(String, VypVersion), Arc<Slot<MetadataResult>>>,
}

/// Result of fetching versions, including PEP 658 file info.
#[derive(Clone, Debug)]
pub struct VersionsResult {
    pub versions: Vec<VypVersion>,
    /// Wheel files per version with PEP 658 metadata availability.
    pub wheel_info: HashMap<VypVersion, Vec<WheelInfo>>,
}

/// Information about a wheel file from the Simple API response.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WheelInfo {
    pub filename: String,
    pub url: String,
    pub has_metadata: bool,
    pub requires_python: Option<String>,
}

/// Metadata result stored in the index.
#[derive(Clone, Debug)]
pub struct MetadataResult {
    pub dependencies: Vec<Requirement>,
    pub full_metadata: Option<PackageMetadata>,
}

/// A slot that starts empty and is filled exactly once.
/// Uses `Condvar` for blocking waits — lighter than `futures::executor::block_on`
/// since it avoids creating a mini async runtime on each wait.
struct Slot<T> {
    value: std::sync::Mutex<Option<Arc<T>>>,
    ready: std::sync::Condvar,
}

impl<T> Slot<T> {
    fn new() -> Self {
        Self {
            value: std::sync::Mutex::new(None),
            ready: std::sync::Condvar::new(),
        }
    }

    fn set(&self, val: T) {
        *self.value.lock().expect("poisoned") = Some(Arc::new(val));
        self.ready.notify_all();
    }

    fn wait_blocking(&self) -> Arc<T> {
        let mut guard = self.value.lock().expect("poisoned");
        loop {
            if let Some(v) = guard.as_ref() {
                return Arc::clone(v);
            }
            guard = self.ready.wait(guard).expect("poisoned");
        }
    }

    fn try_get(&self) -> Option<Arc<T>> {
        self.value.lock().expect("poisoned").as_ref().map(Arc::clone)
    }
}

impl InMemoryIndex {
    pub fn new() -> Self {
        Self {
            versions: DashMap::new(),
            metadata: DashMap::new(),
        }
    }

    pub fn register_versions(&self, package: &str) -> bool {
        use dashmap::mapref::entry::Entry;
        match self.versions.entry(package.to_string()) {
            Entry::Occupied(_) => false,
            Entry::Vacant(v) => {
                v.insert(Arc::new(Slot::new()));
                true
            }
        }
    }

    pub fn wait_versions(&self, package: &str) -> Arc<VersionsResult> {
        let slot = self.versions
            .entry(package.to_string())
            .or_insert_with(|| Arc::new(Slot::new()))
            .clone();
        slot.wait_blocking()
    }

    pub fn try_get_versions(&self, package: &str) -> Option<Arc<VersionsResult>> {
        let entry = self.versions.get(package)?;
        entry.value().try_get()
    }

    pub fn get_wheel_info(&self, package: &str, version: &VypVersion) -> Option<Vec<WheelInfo>> {
        let entry = self.versions.get(package)?;
        entry.value().try_get().and_then(|r| r.wheel_info.get(version).cloned())
    }

    pub fn set_versions(&self, package: &str, result: VersionsResult) {
        let slot = self.versions
            .entry(package.to_string())
            .or_insert_with(|| Arc::new(Slot::new()))
            .clone();
        slot.set(result);
    }

    pub fn register_metadata(&self, package: &str, version: &VypVersion) -> bool {
        use dashmap::mapref::entry::Entry;
        match self.metadata.entry((package.to_string(), version.clone())) {
            Entry::Occupied(_) => false,
            Entry::Vacant(v) => {
                v.insert(Arc::new(Slot::new()));
                true
            }
        }
    }

    pub fn wait_metadata(&self, package: &str, version: &VypVersion) -> Arc<MetadataResult> {
        let key = (package.to_string(), version.clone());
        let slot = self.metadata
            .entry(key)
            .or_insert_with(|| Arc::new(Slot::new()))
            .clone();
        slot.wait_blocking()
    }

    pub fn try_get_metadata(
        &self,
        package: &str,
        version: &VypVersion,
    ) -> Option<Arc<MetadataResult>> {
        let key = (package.to_string(), version.clone());
        let entry = self.metadata.get(&key)?;
        entry.value().try_get()
    }

    pub fn set_metadata(
        &self,
        package: &str,
        version: &VypVersion,
        result: MetadataResult,
    ) {
        let key = (package.to_string(), version.clone());
        let slot = self.metadata
            .entry(key)
            .or_insert_with(|| Arc::new(Slot::new()))
            .clone();
        slot.set(result);
    }
}

impl Default for InMemoryIndex {
    fn default() -> Self {
        Self::new()
    }
}
