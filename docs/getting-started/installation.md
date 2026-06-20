# Installation

vyp can be installed from Cargo, PyPI, or built from source.

## From Cargo

If you have [Rust](https://rustup.rs/) installed:

```bash
cargo install vyp
```

This compiles vyp from crates.io. For the latest development version:

```bash
cargo install --path vyp
```

## From PyPI

Prebuilt binary wheels are available on PyPI for common platforms:

```bash
pip install vyp
```

!!! note "Binary wheels"
    The PyPI package ships a Rust-compiled binary. No Rust or compilation step is required.

## From source

Clone the repository and build:

```bash
git clone https://github.com/vyp-lang/vyp
cd vyp
cargo build --release
```

The binary will be at `target/release/vyp`. Add it to your `PATH` or copy it to a directory in your path.

## Verifying installation

Confirm vyp is installed and on your path:

```bash
vyp --version
```

You should see output like `vyp 0.1.0` (or your installed version).
