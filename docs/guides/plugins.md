# Plugin System Overview

vyp's plugin system extends resolution behavior without modifying the core PubGrub algorithm. Plugins can add conflict strategies, metadata providers, and resolution filters. The resolver core remains sealed—PubGrub itself is not modifiable.

## Plugin Types

### ConflictStrategy

Handles conflicts when the resolver encounters mutually exclusive requirements for a package.

```rust
pub trait ConflictStrategy {
    fn name(&self) -> &str;
    fn priority(&self) -> i32;
    fn evaluate(&self, context: &ConflictContext) -> StrategyVerdict;
    fn suggest(&self, context: &ConflictContext) -> Vec<ConflictSuggestion>;
}
```

**Verdicts:**

| Verdict | Behavior |
|---------|----------|
| `Abstain` | Pass to the next strategy |
| `Fail` | Surface an error (no resolution) |
| `RewriteRanges` | Adjust version ranges to resolve |
| `Fork` | Resolve differently per conflict side |

!!! example "Built-in strategies"
    vyp ships built-in strategies: override application, substitution, transitive fork, and fail. Plugins register additional strategies with custom priorities.

### MetadataProvider

Supplies package metadata and version lists. The default implementation queries PyPI; plugins can add corporate registries, local caches, or virtual packages.

```rust
pub trait MetadataProvider {
    fn name(&self) -> &str;
    fn priority(&self) -> i32;
    fn can_provide(&self, package: &VypPackage) -> bool;
    fn available_versions(&self, package: &VypPackage) -> Result<Option<PackageVersions>, ...>;
    fn get_metadata(&self, package: &VypPackage, version: &VypVersion) -> Result<Option<PackageMetadata>, ...>;
    fn index_url(&self) -> Option<&str>;
}
```

!!! tip "Priority ordering"
    Providers are consulted in priority order (higher first). The first provider that returns `Some` wins.

### ResolutionFilter

Filters or re-ranks version candidates before PubGrub selects a version.

```rust
pub trait ResolutionFilter {
    fn name(&self) -> &str;
    fn priority(&self) -> i32;
    fn filter(&self, candidates: &mut Vec<Candidate>);
}
```

!!! example "Use cases"
    - Corporate allow/deny lists
    - License filtering
    - Vulnerability exclusion
    - Preferred-source pinning

!!! note "Exclude, don't remove"
    Mark unwanted candidates as `excluded` rather than removing them, to preserve provenance.

## The Sealed Core

PubGrub—the dependency resolution algorithm—is **not** extensible. Plugins cannot:

- Change how PubGrub builds the version graph
- Modify the core resolution loop
- Replace the algorithm

Plugins **can**:

- Influence which versions are available (MetadataProvider)
- Filter candidates before selection (ResolutionFilter)
- Handle conflicts with custom strategies (ConflictStrategy)

This keeps the core correct and predictable while allowing domain-specific extensions.

## Search Paths

Load plugins from directories:

```toml
[tool.vyp.plugins]
search-paths = [
  "target/release/",
  "/opt/vyp/plugins/",
]
```

vyp scans each path for `.so` (Linux), `.dylib` (macOS), or `.dll` (Windows) and loads them. Each library must export `vyp_plugin_init`.

!!! warning "Load order"
    Plugins are loaded in directory order. Duplicate registrations (e.g. same strategy name) may overwrite; use distinct names.

## Loading Config

Load specific plugins by path:

```toml
[tool.vyp.plugins]
load = [
  { name = "corporate-strategy", path = "/opt/plugins/corporate.so" },
  { name = "custom-provider", path = "plugins/custom.dylib", config = { url = "https://internal.pypi" } },
]
```

### Load entry fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Plugin identifier |
| `path` | No | Path to the dynamic library |
| `config` | No | Key-value config passed to the plugin |

!!! note "Config usage"
    The `config` map is passed to the plugin at load time. How plugins use it depends on their implementation.

## Plugin ABI

Plugins are Rust dynamic libraries compiled against `vyp-api`. They must:

1. Export `vyp_plugin_init() -> PluginRegistration`
2. Use the same ABI version as the host (`VYP_ABI_VERSION`)
3. Be built with a compatible Rust toolchain

```rust
#[no_mangle]
pub unsafe fn vyp_plugin_init() -> PluginRegistration {
    PluginRegistration {
        abi_version: VYP_ABI_VERSION,
        name: "my-plugin".to_string(),
        version: "0.1.0".to_string(),
        strategies: vec![Box::new(MyStrategy)],
        metadata_providers: Vec::new(),
        filters: Vec::new(),
    }
}
```

!!! warning "ABI compatibility"
    ABI mismatches cause load failure. Rebuild plugins when upgrading vyp.

## Sample Plugin

The `examples/sample-plugin` crate demonstrates a minimal ConflictStrategy:

```rust
struct LoggingStrategy;

impl ConflictStrategy for LoggingStrategy {
    fn name(&self) -> &str { "sample-logging" }
    fn priority(&self) -> i32 { 5 }
    fn evaluate(&self, context: &ConflictContext) -> StrategyVerdict {
        eprintln!("Conflict on {}", context.contested_package.name());
        StrategyVerdict::Abstain
    }
}
```

Build and load:

```bash
cd examples/sample-plugin && cargo build --release
```

```toml
[tool.vyp.plugins]
search-paths = ["examples/sample-plugin/target/release/"]
```

## Next Steps

- [Plugin API](../reference/api.md) — Full trait and type reference
- [Overrides & Conflicts](overrides.md) -- Dependency overrides and transitive conflict declarations
