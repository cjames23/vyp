# Managing Python Projects with vyp

vyp reads project metadata and dependencies from `pyproject.toml`. This guide covers the structure vyp expects and how it interprets PEP 621 metadata, optional dependencies, and PEP 735 dependency groups.

## Minimal pyproject.toml

Start with a minimal project:

```toml
[project]
name = "my-project"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = [
    "requests>=2.28",
    "numpy>=1.24",
]
```

!!! tip "PEP 621 compliance"
    vyp follows [PEP 621](https://peps.python.org/pep-0621/) for project metadata. The `[project]` table and `dependencies` array are the primary source of direct dependencies.

## PEP 621 Project Metadata

vyp reads these fields from `[project]`:

| Field | Purpose |
|-------|---------|
| `name` | Project name (used for display and lock provenance) |
| `version` | Project version |
| `requires-python` | Python version constraint (e.g. `>=3.10`, `<3.13`) |
| `dependencies` | **Primary dependency list** — vyp resolves from this array |

!!! note "Resolution source"
    Currently, vyp resolves from `[project].dependencies` only. Optional dependencies and dependency groups are parsed and stored for future use (e.g. `vyp add --optional`, `vyp add --group`).

## Optional Dependencies (PEP 621)

Optional dependency groups are defined under `[project.optional-dependencies]`:

```toml
[project]
name = "my-project"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = [
    "requests>=2.28",
]

[project.optional-dependencies]
dev = [
    "pytest>=7.0",
    "mypy>=1.0",
]
docs = [
    "mkdocs>=1.5",
    "mkdocs-material",
]
```

!!! example "Add to optional group"
    Use `vyp add --optional <name>` to add packages to a specific optional group:
    ```bash
    vyp add --optional dev pytest-cov
    vyp add --optional docs mkdocs-material
    ```

## Dependency Groups (PEP 735)

PEP 735 introduces top-level `[dependency-groups]` for grouping dependencies without extras:

```toml
[project]
name = "my-project"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = ["requests>=2.28"]

[dependency-groups]
dev = [
    "pytest>=7.0",
    "mypy>=1.0",
]
lint = [
    "ruff>=0.1",
]
```

!!! tip "include-group"
    Groups can reference other groups via `{include-group: <name>}`:
    ```toml
    [dependency-groups]
    dev = [
        "pytest>=7.0",
        { include-group = "lint" },
    ]
    lint = ["ruff>=0.1"]
    ```

!!! example "Add to dependency group"
    Use `vyp add --group <name>` to add packages to a dependency group:
    ```bash
    vyp add --group dev pytest-cov
    vyp add --group lint ruff
    ```

## tool.vyp Configuration

vyp-specific settings live under `[tool.vyp]`:

```toml
[tool.vyp]
index-url = "https://pypi.org/simple"
resolution-strategy = "highest"   # highest | lowest | lowest-direct
pre-releases = "disallow"        # allow | disallow | if-necessary
torch-backend = "auto"           # auto | cpu | cu126 | cu128 | rocm6 | xpu

[[tool.vyp.extra-index]]
name = "pytorch-cu128"
url = "https://download.pytorch.org/whl/cu128"
explicit = true

[tool.vyp.sources]
torch = [
  { index = "pytorch-cu128", marker = "sys_platform == 'linux'" },
]

[[tool.vyp.overrides]]
package = "numpy"
constraint = ">=1.26,<2"
transitive = true
reason = "Security fix"
```

See the [Configuration](../configuration/pyproject.md) reference for the full `[tool.vyp]` schema.

## Project Layout

A typical vyp-managed project looks like:

```
my-project/
├── pyproject.toml      # Project metadata + dependencies
├── pylock.toml         # Lock file (commit this)
├── .venv/              # Virtual environment
├── src/
│   └── my_project/
│       └── __init__.py
└── tests/
```

!!! important "Commit pylock.toml"
    Always commit `pylock.toml` to version control for reproducible installs across machines and CI.

## Next Steps

- [Dependencies](dependencies.md) — Adding, removing, and updating dependencies
- [Lock Files](lockfiles.md) — Lock file format and workflows
- [Package Indexes](indexes.md) — Configuring PyPI and extra indexes
