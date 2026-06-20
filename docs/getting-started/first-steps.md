# First steps

This guide walks through creating a Python project, adding dependencies, locking, and installing with vyp.

## Prerequisites

- vyp [installed](installation.md)
- Python 3.8+ with `venv` available
- A virtual environment (vyp never installs to system Python)

## Create a Python project

Start with a `pyproject.toml` in your project directory:

```toml
[project]
name = "my-project"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = []
```

Create and activate a virtual environment:

```bash
python -m venv .venv
source .venv/bin/activate   # On Windows: .venv\Scripts\activate
```

## Add dependencies

Use `vyp add` to add packages to `[project].dependencies` and install them:

```bash
vyp add requests numpy
```

This will:

1. Append `requests` and `numpy` to your `pyproject.toml`
2. Resolve the dependency graph
3. Update or create `pylock.toml`
4. Install into the active venv

!!! example "With version constraints"
    ```bash
    vyp add "pandas>=2.0" "requests[security]>=2.28"
    ```

!!! tip "Add without installing"
    Use `--no-install` to add and lock without installing:
    ```bash
    vyp add --no-install numpy
    ```

## Lock dependencies

Generate or update the lock file:

```bash
vyp lock
```

This resolves all dependencies from `pyproject.toml` and writes `pylock.toml` (PEP 751 format). The lock file pins exact versions for reproducible installs.

## Install from lock file

Install the locked dependencies into your virtual environment:

```bash
vyp install
```

vyp looks for a venv in this order:

1. `--venv` path if provided
2. `VIRTUAL_ENV` (active venv)
3. `.venv` in the current directory

!!! example "Specify venv explicitly"
    ```bash
    vyp install --venv .venv
    ```

!!! example "Preview without installing"
    ```bash
    vyp install --dry-run
    ```

## Understanding pylock.toml

`pylock.toml` is a PEP 751 lock file. It records:

- Exact package versions
- Dependency relationships
- Wheel URLs and hashes
- Optional variant metadata (PEP 825)

You should commit `pylock.toml` to version control for reproducible builds.

!!! important "Do not edit by hand"
    Edit `pyproject.toml` and run `vyp lock` to update the lock file. Manual edits may be overwritten.

## Dry-run resolution

Preview the resolution without writing a lock file:

```bash
vyp resolve
```

Add extra requirements for one-off resolution:

```bash
vyp resolve -r "numpy>=1.20" -r "pandas>=2.0"
```

## Virtual environment requirements

vyp **never** installs to system Python. You must use a virtual environment:

| Scenario | Action |
|----------|--------|
| No venv | Create one: `python -m venv .venv` |
| Venv exists | Activate it or pass `--venv .venv` |
| Wrong venv | Use `vyp install --venv /path/to/venv` |

!!! warning "System Python"
    Installing into the system Python can break your environment. vyp requires an explicit venv to avoid accidental system installs.

## Next steps

- [Projects](../guides/projects.md) — Project layout and configuration
- [Dependencies](../guides/dependencies.md) — Optional groups, extras, and constraints
- [Lock Files](../guides/lockfiles.md) — Lock file format and workflows
- [Overrides & Conflicts](../guides/overrides.md) -- Dependency overrides, pins, and conflict resolution
