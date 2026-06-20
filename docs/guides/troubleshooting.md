# Troubleshooting

Common issues and their solutions when working with vyp.

## No virtual environment found

```
Error: No virtual environment found. Use --venv to specify one,
activate one, or create .venv in the current directory.
```

vyp needs a virtual environment to install packages into. It looks for one in this order:

1. The `--venv` argument
2. The `VIRTUAL_ENV` environment variable (set by `source .venv/bin/activate`)
3. A `.venv` directory in the current working directory

**Fix:** Create and activate a virtual environment:

```bash
python -m venv .venv
source .venv/bin/activate  # Linux/macOS
# or .venv\Scripts\activate  # Windows
```

## Lock file not found

```
Error: Lock file not found: pylock.toml. Run `vyp lock` first.
```

The `vyp install` command requires a lock file. Run resolution and locking first:

```bash
vyp lock
vyp install
```

## No solution found

```
Error: No solution found:
  numpy requires <incompatible versions>
```

This means the dependency constraints are unsatisfiable. Common causes:

- Two packages require mutually exclusive versions of a shared dependency
- A package specifies an overly restrictive upper bound

**Fix options:**

1. **Add an override** to force a specific version range:
   ```bash
   vyp override add numpy ">=1.26,<2"
   ```

2. **Use substitutions** if packages are interchangeable:
   ```toml
   [[tool.vyp.substitutions]]
   provides = "opencv"
   packages = ["opencv-python", "opencv-python-headless"]
   prefer = "opencv-python-headless"
   ```

3. **Run `vyp explain`** on the conflicting package for more details:
   ```bash
   vyp explain numpy
   ```

## Network / HTTP errors

If vyp fails to download package metadata or wheels:

- Check your internet connection
- Verify the index URL is correct in `pyproject.toml`
- Try setting `RUST_LOG=debug` for more information:
  ```bash
  RUST_LOG=debug vyp lock
  ```
- If behind a corporate proxy, ensure `HTTPS_PROXY` is set

## Pre-release versions

By default, vyp excludes pre-release versions. If a package only has pre-release versions available, resolution may fail.

**Fix:** Change the pre-release policy in `pyproject.toml`:

```toml
[tool.vyp]
pre-releases = "if-necessary"  # allows pre-releases as fallback
# or
pre-releases = "allow"          # always includes pre-releases
```

## PyTorch / CUDA issues

If PyTorch packages fail to resolve or install the wrong variant:

1. Explicitly set the backend:
   ```bash
   vyp lock --torch-backend cu128
   ```

2. Or configure it in `pyproject.toml`:
   ```toml
   [tool.vyp]
   torch-backend = "cu128"  # or "cpu", "rocm6", "auto"
   ```

## Hash or metadata mismatch

```
Error: Hash mismatch for numpy: expected sha256=..., got ...
```

This indicates the downloaded wheel does not match what was recorded in the lock file. Possible causes:

- Stale CDN cache at the package index
- Corrupted download
- Tampered package on a compromised mirror

**Fix:** Delete the lock file and re-lock to get fresh metadata:

```bash
rm pylock.toml
vyp lock
vyp install
```

If the issue persists, try a different package index or verify the index is trustworthy.
