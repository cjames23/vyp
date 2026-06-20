# Adding, Removing, and Updating Dependencies

This guide covers how to add, remove, and update dependencies with vyp using the CLI and by editing `pyproject.toml`.

## Adding Dependencies

### Add to main dependencies

```bash
vyp add requests numpy
```

This appends packages to `[project].dependencies`, resolves the graph, updates `pylock.toml`, and installs into the active virtual environment.

!!! example "With version constraints"
    Use PEP 508 specifiers for version constraints:
    ```bash
    vyp add "pandas>=2.0" "requests[security]>=2.28"
    ```

!!! example "With extras"
    Request extras (optional features) in brackets:
    ```bash
    vyp add "requests[security,socks]"
    ```

### Add to optional dependencies

Add to a PEP 621 optional group:

```bash
vyp add --optional dev pytest mypy
vyp add --optional docs mkdocs mkdocs-material
```

This writes to `[project.optional-dependencies.<name>]`.

### Add to dependency groups

Add to a PEP 735 dependency group:

```bash
vyp add --group dev pytest-cov
vyp add --group lint ruff
```

This writes to `[dependency-groups.<name>]`.

!!! tip "Create group if missing"
    If the group does not exist, vyp creates it. Ensure `[dependency-groups]` exists in `pyproject.toml` or vyp will add it.

### Add without locking or installing

```bash
vyp add --no-lock numpy        # Edit pyproject.toml only
vyp add --no-install pandas    # Edit and lock, but skip install
```

!!! warning "Rollback on failure"
    If resolution fails after `vyp add`, vyp rolls back the `pyproject.toml` changes automatically.

## Removing Dependencies

vyp does not provide a `remove` command. Remove dependencies by editing `pyproject.toml`:

1. Open `pyproject.toml`
2. Delete the package line from the appropriate array:
   - `[project].dependencies`
   - `[project.optional-dependencies.<name>]`
   - `[dependency-groups.<name>]`
3. Run `vyp lock` to regenerate the lock file
4. Run `vyp install` to sync the environment

!!! example "Before and after"
    **Before:**
    ```toml
    [project]
    dependencies = [
        "requests>=2.28",
        "numpy>=1.24",
        "pandas>=2.0",
    ]
    ```
    **After (removed pandas):**
    ```toml
    [project]
    dependencies = [
        "requests>=2.28",
        "numpy>=1.24",
    ]
    ```

!!! tip "Updating an existing dependency"
    If a package with the same normalized name already exists, `vyp add` replaces it:
    ```bash
    vyp add "numpy>=1.26"   # Replaces existing numpy entry
    ```

## Updating Dependencies

### Regenerate the lock file

After editing `pyproject.toml` (adding, removing, or changing constraints), run:

```bash
vyp lock
```

This resolves the dependency graph and writes `pylock.toml` with the new solution.

!!! note "What vyp lock does"
    - Reads `[project].dependencies` from `pyproject.toml`
    - Applies `[tool.vyp]` config (overrides, conflicts, indexes, torch-backend)
    - Resolves with PubGrub
    - Writes `pylock.toml` (PEP 751 format)

### Update a specific package

To bump a package to a newer version:

1. Edit the constraint in `pyproject.toml` (e.g. `numpy>=1.24` → `numpy>=1.26`)
2. Run `vyp lock`
3. Run `vyp install`

Or use `vyp add` to replace the existing entry:

```bash
vyp add "numpy>=1.26"
```

### Named lock files

Generate a lock file with a custom name:

```bash
vyp lock --name dev
```

This creates `pylock.dev.toml` instead of `pylock.toml`. See [Lock Files](lockfiles.md) for details.

## Workflow Summary

| Task | Command |
|------|---------|
| Add to main deps | `vyp add <pkg>` |
| Add to optional | `vyp add --optional <name> <pkg>` |
| Add to group | `vyp add --group <name> <pkg>` |
| Remove | Edit pyproject.toml, then `vyp lock` |
| Update lock | `vyp lock` |
| Install from lock | `vyp install` |

## Next Steps

- [Lock Files](lockfiles.md) — Lock file format, named locks, provenance
- [Overrides & Conflicts](overrides.md) -- Pinning, constraining transitive dependencies, and resolving conflicts
