# Lock Files with vyp

vyp uses the PEP 751 `pylock.toml` format for lock files. This guide covers the format, named lock files, provenance tracking, and how to regenerate locks.

## Generating a Lock File

```bash
vyp lock
```

This resolves dependencies from `pyproject.toml` and writes `pylock.toml` by default.

!!! tip "When to run vyp lock"
    Run `vyp lock` after:
    - Adding or removing dependencies
    - Changing version constraints
    - Modifying `[tool.vyp]` overrides or conflicts
    - Updating indexes or torch-backend

## PEP 751 pylock.toml Format

The lock file follows [PEP 751](https://peps.python.org/pep-0751/):

```toml
lock-version = "1.0"
created-by = "vyp 0.1.0"
requires-python = ">=3.10"

[[packages]]
name = "requests"
version = "2.31.0"
dependencies = ["certifi>=2017.4.17", "charset-normalizer>=2,<4", ...]

[[packages]]
name = "numpy"
version = "1.26.4"
dependencies = []
```

### Top-level fields

| Field | Description |
|-------|-------------|
| `lock-version` | Lock file schema version |
| `created-by` | Tool and version that generated the lock |
| `requires-python` | Python version constraint for the environment |
| `environments` | Environment names (if using multi-env) |
| `extras` | Resolved extras |
| `dependency-groups` | Resolved dependency groups |
| `default-groups` | Default groups to install |
| `packages` | Resolved package entries |

### Package entries

Each package in `[[packages]]` includes:

- `name` — Package name
- `version` — Exact pinned version
- `requires-python` — Python constraint (if any)
- `dependencies` — Resolved dependency specs
- `wheels` — Wheel URLs and hashes
- `sdist` — Source distribution fallback
- `marker` — Environment marker (if conditional)
- `index` — Index the package came from
- `variant` — PEP 825 variant descriptor (e.g. CUDA wheel)
- `tool` — Tool-specific metadata (see provenance)

## Named Lock Files

Create lock files with custom names:

```bash
vyp lock --name dev
vyp lock --name prod
```

This produces:

- `pylock.dev.toml`
- `pylock.prod.toml`

!!! example "Use case"
    Use named locks for different environments or dependency sets:
    ```bash
    vyp lock --name dev    # Resolves dev + main deps
    vyp lock --name prod   # Resolves prod deps only
    ```

!!! note "Default filename"
    The default lock file is `pylock.toml`. Named locks follow `pylock.<name>.toml`.

## Provenance in tool.vyp

vyp stores resolution provenance in `[packages.*.tool.vyp]`:

```toml
[[packages]]
name = "numpy"
version = "1.24.0"

  [packages.tool.vyp]
  selected_by = "override"
  requested_by = ["my-project", "pandas"]
  conflict_with = ["pandas"]
  resolution_path = "my-project -> pandas -> numpy"
```

### Provenance fields

| Field | Description |
|-------|-------------|
| `selected_by` | How this version was chosen: `normal`, `conflict-resolution`, `override`, `substitution`, or `plugin:<name>` |
| `requested_by` | Packages that depend on this one |
| `conflict_with` | Packages that conflicted during resolution |
| `resolution_path` | Dependency path from root to this package |

!!! tip "Explain a package"
    Use the lock file API or inspect `tool.vyp` to understand why a specific version was selected.

## Regenerating Lock Files

To fully regenerate the lock file:

1. Optionally delete the existing lock: `rm pylock.toml`
2. Run `vyp lock`

vyp always resolves from scratch; there is no incremental lock update. The lock file is the output of a full resolution.

!!! warning "Do not edit by hand"
    Edit `pyproject.toml` and run `vyp lock` to update the lock. Manual edits to `pylock.toml` may be overwritten or cause inconsistent installs.

## Commit the Lock File

Always commit `pylock.toml` (and any `pylock.<name>.toml`) to version control:

```bash
git add pylock.toml
git commit -m "Update lock file"
```

This ensures:

- Reproducible installs across machines
- CI uses the same dependency versions
- Teammates get identical environments

## Next Steps

- [Overrides & Conflicts](overrides.md) -- Dependency overrides, pins, and transitive conflicts
- [Lock File Format](../reference/lockfile-format.md) — Full schema reference
