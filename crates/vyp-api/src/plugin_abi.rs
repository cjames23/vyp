use crate::traits::conflict_strategy::ConflictStrategy;
use crate::traits::metadata_provider::MetadataProvider;
use crate::traits::resolution_filter::ResolutionFilter;

/// The ABI version for the plugin contract.
/// Plugins must be compiled against the same ABI version as the host.
pub const VYP_ABI_VERSION: u32 = 1;

/// Registration returned by a plugin's `vyp_plugin_init` function.
pub struct PluginRegistration {
    /// ABI version the plugin was compiled against.
    pub abi_version: u32,
    /// Human-readable plugin name.
    pub name: String,
    /// Plugin version string.
    pub version: String,
    /// Conflict strategies provided by this plugin.
    pub strategies: Vec<Box<dyn ConflictStrategy>>,
    /// Metadata providers provided by this plugin.
    pub metadata_providers: Vec<Box<dyn MetadataProvider>>,
    /// Resolution filters provided by this plugin.
    pub filters: Vec<Box<dyn ResolutionFilter>>,
}

impl PluginRegistration {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            abi_version: VYP_ABI_VERSION,
            name: name.into(),
            version: version.into(),
            strategies: Vec::new(),
            metadata_providers: Vec::new(),
            filters: Vec::new(),
        }
    }

    pub fn with_strategy(mut self, strategy: Box<dyn ConflictStrategy>) -> Self {
        self.strategies.push(strategy);
        self
    }

    pub fn with_metadata_provider(mut self, provider: Box<dyn MetadataProvider>) -> Self {
        self.metadata_providers.push(provider);
        self
    }

    pub fn with_filter(mut self, filter: Box<dyn ResolutionFilter>) -> Self {
        self.filters.push(filter);
        self
    }
}

/// Type signature for the plugin init function.
/// Plugins must export: `#[no_mangle] pub fn vyp_plugin_init() -> PluginRegistration`
///
/// Note: this uses Rust ABI (not `extern "C"`) because `PluginRegistration`
/// contains Rust types (Box<dyn Trait>, Vec, String) that are not FFI-safe.
/// Plugins must be compiled with the same Rust compiler version as the host.
pub type PluginInitFn = unsafe fn() -> PluginRegistration;
