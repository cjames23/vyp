# vyp

**A transitive-conflict-aware Python dependency resolver and installer**

vyp is a Rust-based dependency resolver and installer for Python, inspired by pip and uv. It brings unique features for managing complex dependency graphs: transitive conflict declarations, unified overrides, and PEP 825 wheel variant groundwork.

## Highlights

- **Transitive conflict resolution** — Conflict rules propagate through the entire dependency graph, not just within a single project
- **Unified dependency overrides** — Range constraints and exact pins in a single `[[tool.vyp.overrides]]` array, with optional transitive propagation
- **PEP 751 lock files** — Standards-compliant `pylock.toml` output
- **PyTorch support** — Built-in handling for GPU/CPU and CUDA variant selection
- **PEP 825 groundwork** — Wheel variant selection infrastructure for future platform-specific wheels
- **Plugin system** — Extensible conflict strategies, metadata providers, and resolution filters

## Installation

```bash
# From Cargo (Rust)
cargo install vyp

# From PyPI (prebuilt binary wheels)
pip install vyp
```

!!! tip "First time here?"
    See the [installation guide](getting-started/installation.md) for all installation methods and [first steps](getting-started/first-steps.md) to get started.

## Quick start

Add dependencies, lock, and install:

```bash
vyp add numpy pandas
vyp lock
vyp install
```

That's it. vyp adds packages to your `pyproject.toml`, resolves the dependency graph, writes a `pylock.toml` lock file, and installs into your active or local `.venv`.

!!! important "Virtual environment required"
    vyp never installs to system Python. It installs into the active virtual environment, `VIRTUAL_ENV`, or a local `.venv` in the project directory.

## Learn more

| | |
|---|---|
| [**Getting Started**](getting-started/installation.md) | Installation and first steps |
| [**Guides**](guides/projects.md) | Projects, dependencies, lock files, overrides, conflicts |
| [**Reference**](reference/cli.md) | CLI commands and configuration |
