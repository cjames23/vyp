//! Sample vyp plugin demonstrating how to implement a custom
//! ConflictStrategy.
//!
//! Build with: `cargo build --release`
//! The resulting `.dylib`/`.so`/`.dll` can be loaded by vyp via:
//!
//! ```toml
//! [tool.vyp.plugins]
//! search-paths = ["path/to/target/release/"]
//! ```

use vyp_api::plugin_abi::{PluginRegistration, VYP_ABI_VERSION};
use vyp_api::traits::conflict_strategy::{
    ConflictContext, ConflictStrategy, ConflictSuggestion, StrategyVerdict,
};

/// Entry point called by vyp when loading this plugin.
///
/// # Safety
/// Must be called by a compatible version of vyp with matching ABI.
#[no_mangle]
pub unsafe fn vyp_plugin_init() -> PluginRegistration {
    PluginRegistration {
        abi_version: VYP_ABI_VERSION,
        name: "sample-plugin".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        strategies: vec![Box::new(LoggingStrategy)],
        metadata_providers: Vec::new(),
        filters: Vec::new(),
    }
}

/// A minimal strategy that logs conflicts and abstains from handling them.
/// This serves as a reference implementation for plugin authors.
#[derive(Debug)]
struct LoggingStrategy;

impl ConflictStrategy for LoggingStrategy {
    fn name(&self) -> &str {
        "sample-logging"
    }

    fn priority(&self) -> i32 {
        5
    }

    fn evaluate(&self, context: &ConflictContext) -> StrategyVerdict {
        eprintln!(
            "[sample-plugin] Conflict detected on {} with {} inherited conflict(s)",
            context.contested_package.name(),
            context.inherited_conflicts.len()
        );
        for (requester, range) in &context.requirements {
            eprintln!("  {} requires {}", requester, range);
        }
        StrategyVerdict::Abstain
    }

    fn suggest(&self, _context: &ConflictContext) -> Vec<ConflictSuggestion> {
        vec![ConflictSuggestion {
            source: "sample-plugin".to_string(),
            message: "This is a suggestion from the sample plugin".to_string(),
            command: None,
        }]
    }
}
