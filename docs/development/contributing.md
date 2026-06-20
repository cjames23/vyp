# Contributing to vyp

Thank you for your interest in contributing to vyp. This guide covers how to set up a development environment and submit changes.

---

## Getting Started

### Prerequisites

- **Rust** — Latest stable toolchain (`rustup` recommended)
- **Git** — For version control

### Fork and Clone

1. Fork the repository on GitHub.
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/vyp.git
   cd vyp
   ```
3. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/vyp-lang/vyp.git
   ```

### Branch

Create a feature branch from `main`:

```bash
git checkout -b feature/your-feature-name
```

---

## Build and Test

### Build

```bash
cargo build
```

Release build:

```bash
cargo build --release
```

### Test

```bash
cargo test
```

Run tests for a specific crate:

```bash
cargo test -p vyp-core
cargo test -p vyp
```

### Run with Logging

```bash
RUST_LOG=debug cargo run -- resolve
```

For resolve timing and provider counters (version/metadata wait, solver time, fetch counts), use:

```bash
VYP_PROFILE=1 cargo run -- resolve
```

---

## Code Style

- Follow standard Rust formatting: `cargo fmt`
- Run the linter: `cargo clippy`
- Fix warnings before submitting

---

## Pull Request Process

1. **Ensure tests pass** — `cargo test`
2. **Format and lint** — `cargo fmt && cargo clippy`
3. **Write clear commit messages** — Describe what and why
4. **Open a PR** — Reference any related issues
5. **Address review feedback** — Maintainers will review and may request changes

### PR Checklist

- [ ] Tests pass
- [ ] Code is formatted (`cargo fmt`)
- [ ] No new clippy warnings
- [ ] Documentation updated if needed
- [ ] Changelog updated for user-facing changes (if applicable)

---

## Project Structure

All Rust crates live under **`crates/`**. Examples remain at the repository root.

| Crate | Purpose |
|-------|---------|
| `vyp-api` | Shared types, traits, plugin ABI |
| `vyp-resolver` | Pure PubGrub solver and version-set logic |
| `vyp-index` | PyPI client, in-memory index, disk cache, wheel compat |
| `vyp-core` | Orchestration: ResolverBuilder, strategies, plugins, explain |
| `vyp` | CLI binary, config, lockfile, installation |
| `examples/sample-plugin` | Example plugin implementation |

---

## Questions?

Open an issue on GitHub for questions, bug reports, or feature requests.
