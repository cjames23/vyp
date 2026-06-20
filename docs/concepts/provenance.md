# Provenance

**Provenance** is the "why" behind each selected package version. vyp tracks why each package was chosen, who requested it, and what alternatives were rejected. This data is stored in the lock file and used for debugging and auditing.

## What Provenance Means

For every resolved package, provenance answers:

- **Why** was this version selected?
- **Who** requested it (directly or transitively)?
- **What** did it conflict with?
- **How** did the resolver reach this decision?

## Selection Reasons

Each package has a `selected_by` field indicating how it was chosen:

| Reason | Description |
|--------|-------------|
| **normal** | Standard resolution; no conflict or override involved |
| **conflict-resolution** | Chosen via a conflict strategy (e.g., transitive fork, override) |
| **override** | An explicit override constrained this package |
| **substitution** | Selected as the preferred substitute in a substitution set |
| **plugin:&lt;name&gt;** | A plugin strategy influenced the selection |

!!! example "Selection reasons in practice"
    ```toml
    # In pylock.toml [packages.<pkg>.tool.vyp]
    selected_by = "normal"           # Resolved without special handling
    selected_by = "override"         # User added [[tool.vyp.overrides]]
    selected_by = "conflict-resolution"  # Transitive fork or override strategy
    selected_by = "substitution"     # Chosen from opencv-python vs opencv-python-headless
    selected_by = "plugin:transitive-fork"  # TransitiveForkStrategy selected a side
    ```

## The requested_by Chain

`requested_by` lists every package that directly depends on this one. It forms a chain from the root down.

```
root
  ├── requests (requested_by: root)
  │     └── certifi (requested_by: root, requests)
  └── numpy (requested_by: root)
```

In the lock file:

```toml
[[packages]]
name = "certifi"
version = "2024.2.2"

[packages.tool.vyp]
selected_by = "normal"
requested_by = ["root==0", "requests==2.31.0"]
```

!!! tip "Debugging dependency chains"
    Use `requested_by` to trace why a package was pulled in. If you see an unexpected package, follow the chain to find which direct dependency introduced it.

## conflict_with Records

When a package was selected in the context of a conflict, `conflict_with` can record the other packages or versions that were in conflict. This helps explain why a particular version was chosen over alternatives.

```toml
[packages.tool.vyp]
selected_by = "conflict-resolution"
conflict_with = ["other-package==1.0.0"]
```

## resolution_path for Debugging

`resolution_path` is an optional human-readable string describing the path the resolver took to reach this package. It is useful for understanding complex resolution scenarios and debugging failures.

!!! abstract "Resolution path example"
    ```
    root → requests → certifi
    root → numpy
    root → pandas → numpy  (merged with above)
    ```

## Storage in pylock.toml

Provenance is stored under `[packages.<name>.tool.vyp]` per PEP 751. The `[tool]` section at the package level is reserved for tool-specific metadata and must not affect installation behavior.

```toml
[[packages]]
name = "numpy"
version = "1.26.4"

[packages.tool.vyp]
selected_by = "normal"
requested_by = ["root==0", "pandas==2.2.0"]
conflict_with = []
resolution_path = "root → pandas → numpy"
```

!!! note "PEP 751 compliance"
    PEP 751 allows `[tool]` at the package level and states it MUST NOT affect installation. vyp uses this for provenance only; installers ignore it.

## Inspecting Provenance

Use the lock file's `explain_package` facility (or equivalent tooling) to see why a package was selected:

```
numpy == 1.26.4
  Selected by: normal
  Requested by: root==0, pandas==2.2.0
  Resolution path: root → pandas → numpy
```

## Summary

| Field | Purpose |
|-------|---------|
| **selected_by** | How the version was chosen (normal, override, conflict-resolution, substitution, plugin) |
| **requested_by** | Packages that directly depend on this one |
| **conflict_with** | Packages/versions that conflicted during resolution |
| **resolution_path** | Human-readable resolution path for debugging |
