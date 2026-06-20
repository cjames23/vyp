# vyp

[![CI](https://github.com/vyp-lang/vyp/actions/workflows/ci.yml/badge.svg)](https://github.com/vyp-lang/vyp/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A transitive-conflict-aware Python dependency resolver built in Rust, using the PubGrub algorithm.

**[Documentation](https://vyp.dev)**

## Overview

vyp solves dependency resolution for Python projects with features not found in existing tools:

- **Transitive Conflict Declarations** — Conflict resolution rules propagate through the entire dependency graph, not just within a single project
- **Scoped Named Indexes** — A named index attached to a dependency serves that package *and its transitive closure*, and is never consulted for unrelated dependencies (avoids stale-mirror cross-contamination, e.g. PyTorch's index)
- **Plugin System** — Extensible conflict strategies, metadata providers, and resolution filters without modifying the core solver
- **Package Substitution** — Declare interchangeable packages (e.g., `opencv-python` and `opencv-python-headless`)
- **Dependency Provenance** — Full causal chain explaining why each version was selected
- **Unified Dependency Overrides** — Range constraints and exact pins in a single `[[tool.vyp.overrides]]` array, with optional transitive propagation
- **PEP 751 Lock Files** — Standards-compliant `pylock.toml` output
- **Resolution Explain & Diff** — Trace version selections and compare lock files
- **Space-efficient Cache** — Content-addressed metadata cache with LRU eviction

## Installation

```bash
# From PyPI (prebuilt binary)
pip install vyp

# From source
cargo install --path crates/vyp
```

## Usage

### Add dependencies

```bash
# Add to [project].dependencies, re-lock, and install
vyp add numpy "pandas>=2.0" "requests[security]>=2.28"

# Add to an optional dependency group
vyp add --optional dev pytest pytest-cov

# Add to a PEP 735 dependency group
vyp add --group test pytest

# Add + lock but skip install
vyp add --no-install numpy

# Just edit pyproject.toml
vyp add --no-lock numpy
```

### Resolve dependencies

```bash
# Resolve from pyproject.toml (preview without writing lock file)
vyp resolve

# Resolve with additional requirements
vyp resolve -r "numpy>=1.20" -r "pandas>=2.0"
```

### Generate a lock file

```bash
vyp lock
```

### Install from lock file

```bash
vyp install                     # installs into the active or local .venv
vyp install --venv .venv        # install into a specific venv
vyp install --dry-run           # preview what would be installed
```

### Explain a package selection

```bash
vyp explain numpy
```

### Diff lock files

```bash
vyp diff old.pylock.toml pylock.toml
```

### Manage dependency overrides

```bash
vyp override add numpy ">=1.26,<2"
vyp override add numpy "==1.24.0" --transitive --reason "GPU/CPU fix"
vyp override list                 # lists all overrides, marks transitive ones
vyp override remove numpy
vyp override export               # export transitive overrides for library consumers
```

### View inherited conflicts

```bash
vyp conflict list     # list conflicts inherited from resolved dependencies
```

### Inspect plugins

```bash
vyp plugin list
```

## Configuration

Add to your `pyproject.toml`:

```toml
[tool.vyp]
resolution-strategy = "highest"  # highest | lowest | lowest-direct
pre-releases = "disallow"        # allow | disallow | if-necessary

# Dependency overrides (range constraint, local only by default)
[[tool.vyp.overrides]]
package = "numpy"
constraint = ">=1.26,<2"
reason = "Pin numpy to 1.x for CUDA compatibility"

# Transitive override propagates to consumers
[[tool.vyp.overrides]]
package = "scipy"
constraint = ">=1.11,<2"
transitive = true
reason = "Ensure stable scipy across all transitive deps"

# Package substitutions
[[tool.vyp.substitutions]]
provides = "opencv"
packages = ["opencv-python", "opencv-python-headless"]
prefer = "opencv-python-headless"

# Plugin system
[tool.vyp.plugins]
search-paths = ["./vyp-plugins/"]
```

### Scoped named indexes

Declare an explicit named index and route a dependency to it. The index then
serves that package **and its direct and transitive dependencies** — but it is
**not** consulted for any other top-level dependency or its closure:

```toml
[[tool.vyp.extra-index]]
name = "torch"
url = "https://download.pytorch.org/whl/cu128"
explicit = true

# `torch` and everything only `torch` needs resolve from the torch index.
# Unrelated dependencies (and shared packages) keep resolving from PyPI.
[tool.vyp.sources]
torch = [{ index = "torch" }]
```

This solves the common PyTorch problem where an accelerator index mirrors a
subset of PyPI with out-of-date copies: those mirrored packages are only used
when they are genuinely (and exclusively) part of `torch`'s dependency tree.

**Overlap policy — the default index wins.** If a package is reachable from
both a scoped root (e.g. `torch`) and an ordinary dependency, it resolves from
the **default index (PyPI)**, not the named index. Only packages reachable
*exclusively* through the named index's roots are routed there; the declared
root packages themselves are always authoritative to their index. Membership is
discovered as the dependency graph is walked; in rare diamond-shaped graphs you
can force a decision by routing the shared package explicitly in
`[tool.vyp.sources]`.

The `torch-backend` setting (auto/cpu/cuXYZ/rocm/xpu) uses the same scoped
machinery automatically, so the matching CUDA/ROCm builds of torch's
dependencies come from the accelerator index while the rest of your project
stays on PyPI.

## Architecture

vyp is organized as a Cargo workspace with five crates under **`crates/`**:

| Crate | Location | Purpose |
|-------|----------|---------|
| `vyp-api` | `crates/vyp-api/` | Stable plugin interface: trait definitions, shared types, ABI contract |
| `vyp-resolver` | `crates/vyp-resolver/` | Pure PubGrub solver and version-set logic |
| `vyp-index` | `crates/vyp-index/` | PyPI client, in-memory index, disk cache, wheel compat |
| `vyp-core` | `crates/vyp-core/` | Orchestration: ResolverBuilder, conflict propagation, built-in strategies |
| `vyp` | `crates/vyp/` | CLI binary: config parsing, lock file I/O, plugin loading |

### Plugin System

Plugins implement traits from `vyp-api`:

- **`ConflictStrategy`** — Custom conflict resolution logic
- **`MetadataProvider`** — Alternative package metadata sources
- **`ResolutionFilter`** — Pre-filter or re-rank version candidates

The PubGrub solver loop itself is sealed — plugins influence resolution through well-defined hook points but cannot bypass the core algorithm.

## Development

```bash
# Build
cargo build

# Test
cargo test

# Run with tracing
RUST_LOG=debug cargo run -- resolve
```

## License

MIT
