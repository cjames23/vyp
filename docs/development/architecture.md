# Architecture

vyp is a Rust-based Python dependency resolver organized as a Cargo workspace. This document describes the crate structure, data flow, and performance-related design.

---

## Project layout

All Rust crates live under **`crates/`**: `crates/vyp-api`, `crates/vyp-core`, `crates/vyp`, `crates/vyp-resolver`, `crates/vyp-index`. The **`examples/`** directory remains at the repository root (e.g. `examples/sample-plugin`).

---

## Crate Structure

| Crate | Purpose |
|-------|---------|
| **vyp-api** | Shared types, traits, and plugin ABI. Defines the stable interface between the resolver and plugins. |
| **vyp-resolver** | Pure PubGrub algorithm and version-set logic (solver + provider). Depends only on `vyp-api` and `version-ranges`. |
| **vyp-index** | PyPI Simple API client, InMemoryIndex, disk metadata cache (content-addressed, LRU), wheel compatibility, offline provider for tests. |
| **vyp-core** | Orchestration: ResolverBuilder, plugin loader, strategies, explain, provenance. Depends on `vyp-resolver` and `vyp-index`; wires resolution and metadata. |
| **vyp** | CLI binary: config parsing, lockfile I/O, installation, accelerator detection. |

### vyp-api

- **Types**: `Requirement`, `VypPackage`, `VypVersion`, `ConflictDeclaration`, `DependencyOverride`, `SubstitutionSet`, `Provenance`, etc.
- **Traits**: `MetadataProvider`, `ConflictStrategy`, `ResolutionFilter`
- **Plugin ABI**: `PluginRegistration`, `VYP_ABI_VERSION`, `vyp_plugin_init` signature

### vyp-resolver

- **Solver**: `SolverState`, `SolverError`, `PackageId`, `IncompatId`, `VS` (version set). Integer handles in the hot loop; unit propagation, conflict resolution, VSIDS.
- **Provider**: `requirements_to_range` (constraint → version range conversion).

### vyp-index

- **PyPI**: `PyPIMetadataProvider` — Simple API v1+JSON, multi-thread tokio runtime for HTTP; **InMemoryIndex** (thread-safe slots + Condvar) for solver ↔ fetcher handoff.
- **Cache**: Content-addressed disk metadata cache with LRU eviction.
- **Wheel compat**: `PlatformTags`, wheel filename parsing.
- **Offline**: `OfflineMetadataProvider` for tests.

### vyp-core

- **Orchestration**: `ResolverBuilder`, `ResolutionResult`; runs the solver (from `vyp-resolver`) on a **dedicated OS thread** (`vyp-solver`); uses providers (e.g. from `vyp-index`).
- **Strategies**: `OverrideStrategy`, `TransitiveForkStrategy`, `SubstitutionStrategy`, `PreReleaseFilter`
- **Plugin**: Loader, registries for strategies/providers/filters
- **Explain / Provenance**: Failure explanations, selection reasons, requested-by, conflict-with

### vyp

- **CLI**: Commands via `clap` — resolve, lock, install, add, override, conflict, explain, diff, plugin
- **Config**: Parse `[tool.vyp]` from `pyproject.toml`
- **Lock**: Read/write `pylock.toml`, `vyp-overrides.toml`
- **Install**: Download wheels, install into site-packages
- **Accelerator**: GPU detection (CUDA, ROCm, XPU), torch-backend index selection

---

## Data Flow

```
┌─────────────────┐
│ pyproject.toml   │
│ [tool.vyp]      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌──────────────────┐
│ VypConfig       │────▶│ MetadataProvider │ (PyPI, extra indexes, torch)
│ (vyp)           │     │ (vyp-core)       │
└────────┬────────┘     └────────┬─────────┘
         │                       │
         │  overrides,           │  versions, metadata
         │  substitutions,      │
         │  conflicts,           │
         ▼  plugins              ▼
┌─────────────────┐     ┌──────────────────┐
│ ResolverBuilder │────▶│ PubGrub Resolver  │
│ (vyp-core)      │     │ (vyp-core)       │
└────────┬────────┘     └────────┬─────────┘
         │                        │
         │  ConflictStrategy      │  ResolutionResult
         │  ResolutionFilter      │  (packages, provenance,
         │                        │   inherited_conflicts)
         ▼                        ▼
┌─────────────────┐     ┌──────────────────┐
│ LockFile        │     │ Install          │
│ pylock.toml     │     │ (wheels → venv)  │
└─────────────────┘     └──────────────────┘
```

### Resolution Pipeline

1. **Config** — Load `pyproject.toml`, parse `[tool.vyp]`, create providers from index-url, extra-index, torch-backend, sources.
2. **Providers** — Each provider (PyPI, extra indexes, PyTorch) fetches versions and metadata. Cache hits avoid network.
3. **Resolver** — PubGrub solves constraints. Conflict strategies and overrides influence version selection. Filters exclude candidates.
4. **Lockfile** — `ResolutionResult` → `PyLockFile` with provenance in `tool.vyp`.
5. **Install** — Read lockfile, download wheels from index URLs, extract to site-packages.

### Resolution runtime model

The solver runs on a **single dedicated OS thread** (`vyp-solver`). It never uses `block_on` or the tokio runtime. When it needs versions or metadata it blocks on **InMemoryIndex** (`wait_versions` / `wait_metadata`) using a Condvar. Tokio worker threads (in the PyPI provider) run async HTTP and fill the same InMemoryIndex. So: **solver blocks on Condvar → tokio workers do I/O → solver unblocks when data is set**.

```
┌──────────────────┐         ┌─────────────────┐         ┌──────────────────┐
│ Solver thread    │  wait   │ InMemoryIndex   │  set    │ Tokio workers   │
│ (vyp-solver)    │◀───────▶│ (Slots+Condvar) │◀───────▶│ (HTTP fetches)  │
└──────────────────┘         └─────────────────┘         └──────────────────┘
```

### Performance-related design

- **Batch prefetch** — Root dependencies are prefetched before the solver loop. Each iteration prefetches undecided packages not yet in the version cache. Providers may chain metadata prefetch for likely versions.
- **In-run caches** — `version_cache` and `deps_cache` (per resolve) avoid repeated provider calls for the same package/version.
- **Warm start** — Optional `WarmStartData` pre-populates `deps_cache` from a previous resolution to speed re-resolve.
- **PackageId / IncompatId** — The solver uses `u32` handles instead of strings or `VypPackage` in the hot loop (hashing, BTreeMap keys, unit propagation).

---

## Key Design Decisions

- **Sealed solver** — The PubGrub loop is internal; plugins influence resolution via traits, not by replacing the algorithm.
- **Content-addressed cache** — Metadata cached by `{package}=={version}` hash; LRU eviction when over size limit.
- **Transitive propagation** — Overrides with `transitive = true` propagate to consumers via `vyp-overrides.toml`.
- **PEP 751 lock files** — Standards-compliant `pylock.toml`; tool-specific data in `[tool.vyp]` only.

---

## Crate split (vyp-resolver and vyp-index)

The pure PubGrub solver lives in **vyp-resolver** (algorithm + version-set conversion only). The PyPI client, in-memory index, and disk cache live in **vyp-index**. **vyp-core** depends on both and provides orchestration (ResolverBuilder, plugin loader, strategies). The CLI (**vyp**) depends on **vyp-core** and **vyp-index** (to construct `PyPIMetadataProvider` from config).
