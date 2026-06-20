# Plan: Universal Resolution and UV-Style Fork Strategy

Implement universal resolution (one lockfile for multiple Python versions and/or platforms) and a `fork-strategy` option (`requires-python` vs `fewest`) to control how versions are chosen when the same package is resolved for multiple environments. This plan is actionable for implementation now.

---

## Goals

1. **Universal resolution**: Resolve for a set of environments (e.g. Python 3.8, 3.9, 3.10) so the lockfile is portable. A package may appear multiple times with different versions and markers.
2. **Fork strategy**: When multiple environments are used, allow the user to choose:
   - **`requires-python`** (default): Prefer the latest version per environment (one version per Python band).
   - **`fewest`**: Minimize the number of distinct versions; prefer one version for all environments when possible.
3. **Backward compatible**: When no multiple environments are configured, behavior remains single-environment resolution (current behavior).

---

## 1. Config and CLI

### 1.1 New config keys in `[tool.vyp]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `environments` | array of strings | `[]` | List of PEP 508 environment marker strings. Must be disjoint (e.g. `python_version == "3.8"`, `python_version == "3.9"`). When empty, single-environment resolution (current behavior). |
| `fork-strategy` | string | `"requires-python"` | When resolving for multiple environments: `"requires-python"` = latest version per environment; `"fewest"` = minimize distinct versions. Ignored when `environments` is empty. |

**Deriving environments from `requires-python` (optional Phase 1):** If `environments` is empty and project has `requires-python` (e.g. `>=3.8`), we could auto-expand to a set of python_version markers (e.g. 3.8, 3.9, 3.10, … up to a cap). This plan keeps Phase 1 explicit: user sets `environments` in config. Auto-derivation can be a follow-up.

**Files to change:**

- **vyp-api** or **vyp** config types: Add `environments: Vec<String>` and `fork_strategy: ForkStrategy` (enum: RequiresPython, Fewest) to the struct that holds `[tool.vyp]` (e.g. in `vyp/src/config/settings.rs` or equivalent).
- **docs/configuration/pyproject.md**: Document `environments` and `fork-strategy`.

### 1.2 Parsing

- Parse `tool.vyp.environments` (array of strings).
- Parse `tool.vyp.fork-strategy` (string `"requires-python"` | `"fewest"`); default `"requires-python"`.
- Validate: if non-empty, `environments` entries should be disjoint (e.g. check that no two markers can both be true for the same interpreter). Optional first version: accept any list and document that they must be disjoint.

---

## 2. Marker environments per target

To run the resolver once per environment, we need a **MarkerEnvironment** (or equivalent) for each entry in `environments`. Today `MarkerEnvironment` is detected from a real interpreter or built from compile-time defaults.

**Add:**

- **Synthetic MarkerEnvironment from a marker string or Python version**: Either (a) add `MarkerEnvironment::for_python_version(version: &str)` that sets `python_version` / `python_full_version` and keeps other fields from `current()`, or (b) add a small helper that, given a marker like `python_version == "3.8"`, returns a `MarkerEnvironment` with that Python version. Option (a) is enough if we restrict `environments` to python_version-only at first (e.g. `["python_version == \"3.8\"", "python_version == \"3.9\""]`).

**Files:**

- **vyp-api** `MarkerEnvironment`: Add `pub fn for_python_version(major: u8, minor: u8) -> Self` (or from string "3.8") that fills in python_version / python_full_version, rest from `current()`.
- **vyp** (or core): When building the list of environments, map each `environments[i]` to a `MarkerEnvironment`. For a first version, support only markers of the form `python_version == "X.Y"` and parse X.Y to construct `MarkerEnvironment::for_python_version(X, Y)`. More complex markers can be a later extension.

---

## 3. Resolution flow (multiple environments)

### 3.1 Entry point

- **Current**: `resolve_and_build()` (or equivalent) builds one `ResolverBuilder`, calls `resolve()`, returns one `ResolutionResult`.
- **New**: If `config.environments` is empty, keep current behavior (one resolution, one result). If non-empty, for each environment:
  1. Build a `MarkerEnvironment` for that environment.
  2. Build providers with that `MarkerEnvironment` (same index URLs, etc.).
  3. Build `ResolverBuilder` with the same root dependencies and overrides, add those providers, call `resolve()`.
  4. Collect `ResolutionResult` for that environment.

So we get `Vec<(String, ResolutionResult)>` where the String is the marker for that environment (or an env id). Root dependencies are the same for all; only marker evaluation during resolution differs (so different versions can be chosen per environment).

**Files:**

- **vyp** (e.g. `cli/common.rs` or wherever resolution is triggered): If `config.environments.is_empty()` { current path }. Else: loop over `config.environments`, for each build marker env, create providers with that env, run `builder.resolve()`, push `(marker, result)`.

### 3.2 Shared provider setup

- Reuse existing logic to build the list of providers (index, extra-index, torch, etc.). The only change is passing the per-environment `MarkerEnvironment` when constructing `PyPIMetadataProvider` (and any other provider that uses marker env). So each resolution run gets its own set of providers with one specific marker env.

---

## 4. Merge and fork strategy

After collecting `Vec<(marker, ResolutionResult)>`:

- **Input**: N resolution results, each with `packages: HashMap<String, VypVersion>`, plus provenance/wheel_urls.
- **Output**: A single structure that the lockfile writer can use: a list of “package entries” where each entry has name, version, optional marker, and optional provenance/wheel info.

**Merge logic:**

- **If fork-strategy is `requires-python`**: For each (marker, result), for each (name, version) in result.packages, emit one lockfile package entry: (name, version, marker = this environment’s marker). So we get one entry per package per environment (same package name can appear multiple times with different version and/or marker).
- **If fork-strategy is `fewest`**: For each package name that appears in any result:
  - Collect the set of (version, marker) that were chosen (one per environment).
  - If there is a single version that appears for all environments, emit one package entry (name, that version, marker = None).
  - Else, try to find one version V such that for every environment, V was chosen or V is compatible with that environment’s requires_python (if we have that data). If found, emit one entry (name, V, marker = None).
  - Otherwise, emit one entry per environment: (name, version chosen for that env, marker = that env’s marker).

**Data structure for merged result:**

- Introduce a type that the lockfile writer accepts, e.g. `UniversalResolutionResult` or extend the type used by `PyLockFile::from_resolution`. It should represent a list of package entries, each with: name, version, marker (optional), provenance (optional), wheel_urls (optional). Option A: `Vec<PackageLockEntry>` where `PackageLockEntry { name, version, marker, provenance, wheel_url }`. Option B: Keep `ResolutionResult` for single-env; add `UniversalResolutionResult { entries: Vec<PackageLockEntry>, ... }` for multi-env. Lockfile `from_resolution` then has an overload or a new method that takes this merged list and fills `packages` (and top-level `requires_python`, `environments`).

**Files:**

- **vyp-core** or **vyp**: Define `PackageLockEntry` (or reuse/adapt existing) and a function `merge_universal_results(env_results: Vec<(String, ResolutionResult)>, fork_strategy: ForkStrategy) -> Vec<PackageLockEntry>` (or a struct holding that plus top-level requires_python).
- **vyp** lockfile: Add `PyLockFile::from_universal_resolution(merged_entries, project_requires_python, environment_markers)` that sets `packages` from the merged list (each entry → one `PyLockPackage` with `marker` set when present), and sets top-level `requires_python` and `environments`.

---

## 5. Lockfile format and install

### 5.1 Lockfile

- PEP 751 already allows `packages[].marker`. Use it: each entry in the merged list becomes one element of `packages`; if the entry has a marker, set `packages[i].marker = Some(marker_string)`.
- Set top-level `requires_python` from project (from pyproject.toml).
- Set top-level `environments` to the list of marker strings we resolved for (so install and tooling know which environments are in the lockfile).

### 5.2 Install

- When installing from a lockfile, **filter** `lockfile.packages`: only install a package if its `marker` is `None` or the marker evaluates to true for the **current** `MarkerEnvironment` (detected or from `--python`). This way a universal lockfile still installs only the subset relevant to the current environment.
- **Files**: In the install path that iterates over `lockfile.packages`, skip packages whose `marker` is present and evaluates to false for the current env. Use `vyp_api::MarkerTree::parse` and `evaluate(&env)` (or equivalent) if the lockfile stores marker as string.

---

## 6. Wheel URLs and provenance in merged result

- Per-environment `ResolutionResult` has `wheel_urls` and `provenance`. When merging:
  - For each package entry in the merged list, attach the wheel_url and provenance from the resolution that contributed that (name, version, marker). If the same (name, version) is used for multiple markers (fewest), use one of them (e.g. first environment) for wheel_url and provenance.
- Store in `PackageLockEntry` (or equivalent) so the lockfile writer can set `wheels` and `tool.vyp` per package.

---

## 7. Implementation order (phased)

### Phase 1: Config and single resolution unchanged

1. Add config parsing for `environments` and `fork-strategy` in the vyp config layer; default `environments = []`, `fork-strategy = "requires-python"`. No behavior change when `environments` is empty.
2. Add `MarkerEnvironment::for_python_version(major, minor)` (or from string) in vyp-api.

### Phase 2: Multi-environment resolution and merge

3. In the resolution entry point (vyp CLI): when `environments` is non-empty, loop over environments, build `MarkerEnvironment` per env (support `python_version == "X.Y"` only at first), create providers per env, run resolve per env, collect `Vec<(marker, ResolutionResult)>`.
4. Implement `merge_universal_results(env_results, fork_strategy)` with both strategies (requires-python = one entry per env with marker; fewest = single version when possible, else per-env with marker). Define `PackageLockEntry` (or equivalent) and the merged list type.
5. Add `PyLockFile::from_universal_resolution(...)` (or extend `from_resolution` to accept merged entries) and wire lock/lockfile write to use it when we have multiple environments. Set top-level `requires_python` and `environments` on the lockfile.

### Phase 3: Install and docs

6. Install: filter packages by current marker env (skip packages with marker that evaluates to false). Ensure install still works for single-env lockfiles (no marker) and for universal lockfiles (marker set).
7. Docs: Update pyproject config reference and resolution/concepts docs to describe universal resolution and fork-strategy. Add a short “Universal resolution” section in resolution.md.

### Phase 4 (optional): Relax environment format

8. Support more than `python_version == "X.Y"` (e.g. `python_version >= "3.8" and python_version < "3.9"`) by deriving a representative MarkerEnvironment or a small set of them. Can be deferred.

---

## 8. Testing

- **Unit**: `merge_universal_results` with 2–3 environments; requires-python produces N entries per package; fewest produces 1 when all envs agree, else N.
- **Integration**: Resolve with `environments = ["python_version == \"3.8\"", "python_version == \"3.9\""]` and a dependency that has different compatible versions (e.g. numpy); assert lockfile has two entries for that package with different markers (for requires-python) or one when fewest applies. Install with a 3.8 venv and assert only 3.8-marked (or unmarked) packages are installed.
- **Backward compat**: With `environments = []`, resolution and lock output unchanged; install unchanged.

---

## 9. Summary

| Area | Change |
|------|--------|
| Config | `environments: Vec<String>`, `fork-strategy: requires-python \| fewest` |
| Marker env | `MarkerEnvironment::for_python_version` (or from marker string for `python_version == "X.Y"`) |
| Resolution | When environments non-empty: resolve once per environment with per-env MarkerEnvironment and providers |
| Merge | `merge_universal_results` implements requires-python (one entry per env) and fewest (single version when possible) |
| Lockfile | New/updated constructor from merged entries; set `packages[].marker`, top-level `environments`, `requires_python` |
| Install | Filter packages by current marker env (install only if marker is None or evaluates to true) |
| Docs | Config reference, resolution concept (universal + fork strategy) |

This plan is implementable as-is; optional Phase 4 can follow once basic universal resolution and fork-strategy work end-to-end.
