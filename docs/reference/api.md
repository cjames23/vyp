# Plugin API Reference

vyp exposes a plugin system for extending conflict resolution, metadata sources, and version filtering. Plugins implement traits from `vyp-api` and are loaded as dynamic libraries.

---

## Plugin ABI Version

Plugins must be compiled against the same ABI version as the host.

```rust
pub const VYP_ABI_VERSION: u32 = 1;
```

Mismatched ABI versions cause load failure.

---

## Plugin Registration

Plugins export an init function:

```rust
#[no_mangle]
pub unsafe fn vyp_plugin_init() -> PluginRegistration
```

### PluginRegistration

| Field | Type | Description |
|-------|------|-------------|
| `abi_version` | u32 | Must equal `VYP_ABI_VERSION`. |
| `name` | String | Human-readable plugin name. |
| `version` | String | Plugin version. |
| `strategies` | Vec<Box<dyn ConflictStrategy>> | Conflict strategies. |
| `metadata_providers` | Vec<Box<dyn MetadataProvider>> | Metadata providers. |
| `filters` | Vec<Box<dyn ResolutionFilter>> | Resolution filters. |

Builder methods: `with_strategy`, `with_metadata_provider`, `with_filter`.

---

## MetadataProvider Trait

Controls where package metadata comes from. Multiple providers compose via priority; the first that returns `Some` wins.

| Method | Signature | Description |
|--------|-----------|-------------|
| `name()` | `&str` | Human-readable provider name. |
| `priority()` | `i32` | Higher = consulted first. |
| `can_provide()` | `(&VypPackage) -> bool` | Whether this provider can supply metadata for the package. |
| `available_versions()` | `(&VypPackage) -> Result<Option<PackageVersions>>` | List available versions. |
| `get_metadata()` | `(&VypPackage, &VypVersion) -> Result<Option<PackageMetadata>>` | Get metadata for a specific version. |
| `index_url()` | `() -> Option<&str>` | Index URL for provenance (default: `None`). |
| `prefetch()` | `(&[String])` | Hint that these packages will be needed soon; implementations may start fetching in the background (default: no-op). Reduces blocking when the solver later requests versions. |
| `prefetch_metadata()` | `(&str, &[VypVersion])` | Hint that metadata for these package+versions will be needed (e.g. for speculative fetch); default: no-op. |
| `try_available_versions()` | `(&VypPackage) -> Option<PackageVersions>` | Non-blocking: return versions if already available (e.g. from prefetch); default: `None`. Lets the solver prefer packages whose data has already arrived. |
| `wheel_url()` | `(&str, &VypVersion) -> Option<(String, String)>` | Best wheel URL for lockfile/install (default: `None`). |
| `profile_data()` | `() -> HashMap<String, usize>` | Profiling counters when `VYP_PROFILE=1` (default: empty). |

Implementing `prefetch` and `try_available_versions` improves performance by overlapping I/O with solver work and avoiding unnecessary blocking.

### PackageMetadata

| Field | Type |
|-------|------|
| `package` | VypPackage |
| `version` | VypVersion |
| `dependencies` | Vec<Requirement> |
| `conflict_declarations` | ConflictSet |
| `source` | String |

### PackageVersions

| Field | Type |
|-------|------|
| `package` | VypPackage |
| `versions` | Vec<VypVersion> |

---

## ConflictStrategy Trait

Evaluates conflicts and returns a verdict. Built-in strategies (transitive-fork, override, substitution) implement this trait.

| Method | Signature | Description |
|--------|-----------|-------------|
| `name()` | `&str` | Human-readable strategy name. |
| `priority()` | `i32` | Higher = consulted first. Built-ins use 0–100. |
| `evaluate()` | `(&ConflictContext) -> StrategyVerdict` | Evaluate the conflict. |
| `suggest()` | `(&ConflictContext) -> Vec<ConflictSuggestion>` | Suggestions when resolution fails (default: empty). |

### ConflictContext

| Field | Type |
|-------|------|
| `contested_package` | VypPackage |
| `requirements` | Vec<(String, String)> — (requesting_package, required_range_display) |
| `inherited_conflicts` | Vec<ConflictDeclaration> |
| `current_resolution` | HashMap<String, VypVersion> |

### StrategyVerdict

| Variant | Description |
|---------|-------------|
| `Abstain` | Pass to next strategy. |
| `Fail { message }` | Surface as error. |
| `RewriteRanges { rewrites }` | Rewrite version ranges. |
| `Fork { conflict_name, forks }` | Fork resolution by conflict side. |

### RangeRewrite

| Field | Type |
|-------|------|
| `package` | String |
| `new_lower` | Option<VypVersion> |
| `new_upper` | Option<VypVersion> |
| `upper_inclusive` | bool |

### ForkSpec

| Field | Type |
|-------|------|
| `side` | String |
| `lower` | Option<VypVersion> |
| `upper` | Option<VypVersion> |
| `upper_inclusive` | bool |

### ConflictSuggestion

| Field | Type |
|-------|------|
| `source` | String |
| `message` | String |
| `command` | Option<String> |

---

## ResolutionFilter Trait

Pre-filters or re-ranks version candidates before PubGrub processes them.

| Method | Signature | Description |
|--------|-----------|-------------|
| `name()` | `&str` | Human-readable filter name. |
| `priority()` | `i32` | Higher = applied first. |
| `filter()` | `(&mut Vec<Candidate>)` | Mark unwanted candidates as excluded (do not remove). |

### Candidate

| Field | Type |
|-------|------|
| `package` | VypPackage |
| `version` | VypVersion |
| `excluded` | bool |
| `exclusion_reason` | Option<String> |

Use `Candidate::exclude(reason)` to mark a candidate as excluded while preserving provenance.

---

## Loading Plugins

Plugins are loaded via `[tool.vyp.plugins]`:

- **search-paths**: Directories scanned for `.so`, `.dylib`, `.dll` files.
- **[[tool.vyp.plugins.load]]**: Explicit load with `name`, `path`, `config`.

Plugins use Rust ABI (not `extern "C"`); they must be compiled with the same Rust compiler version as the host.
