# Substitutions

**Package substitution** lets you declare that multiple packages are interchangeable and that only one should be installed. vyp uses a provides/packages model with optional preference ordering.

## The provides/packages Model

A substitution set has:

| Field | Description |
|-------|-------------|
| **provides** | Virtual capability name (e.g., `"opencv"`) |
| **packages** | Concrete packages that fulfill it (e.g., `["opencv-python", "opencv-python-headless"]`) |
| **prefer** | Optional preferred package when multiple are requested |

Only **one** package from the set will be installed. If dependencies request different packages from the same set, vyp uses the preference (if configured) or fails with a clear message.

## Preference Ordering

When you specify `prefer`, the resolver:

1. Excludes non-preferred packages from the candidate set (via the substitution filter).
2. Selects only the preferred package when the capability is needed.

If no preference is set and multiple packages from the set are requested, resolution fails with a suggestion to configure preference.

## Use Cases

### opencv-python vs opencv-python-headless

Both provide OpenCV. The headless variant has no GUI dependencies and is often preferred for servers:

```toml
[[tool.vyp.substitutions]]
provides = "opencv"
packages = ["opencv-python", "opencv-python-headless"]
prefer = "opencv-python-headless"
```

!!! tip "Server deployments"
    Use `opencv-python-headless` when you don't need GUI support. It avoids pulling in Qt and other display libraries.

### Pillow vs pillow-simd

pillow-simd is a drop-in replacement for Pillow with SIMD optimizations. You might prefer it when available:

```toml
[[tool.vyp.substitutions]]
provides = "pillow"
packages = ["Pillow", "pillow-simd"]
prefer = "pillow-simd"
```

!!! warning "Compatibility"
    Ensure the preferred package is a true drop-in. pillow-simd maintains API compatibility; other substitutes may not.

## How It Works

1. **Substitution strategy** (conflict resolution): When multiple packages from a substitution set are requested, the strategy either fails with a suggestion or (if preference is set) does not trigger—the filter handles it.

2. **Substitution filter** (candidate filtering): Before the resolver considers versions, the filter marks non-preferred substitutes as excluded. Only the preferred package remains in the candidate set.

!!! abstract "Resolution flow with substitutions"
    ```
    Dependencies request: opencv-python, opencv-python-headless
         ↓
    Substitution filter: exclude opencv-python (prefer: opencv-python-headless)
         ↓
    Resolver sees only opencv-python-headless as candidate
         ↓
    Resolution succeeds with opencv-python-headless
    ```

## Configuration

Add substitutions in `pyproject.toml`:

```toml
[[tool.vyp.substitutions]]
provides = "opencv"
packages = ["opencv-python", "opencv-python-headless"]
prefer = "opencv-python-headless"
```

Multiple substitution sets are supported:

```toml
[[tool.vyp.substitutions]]
provides = "opencv"
packages = ["opencv-python", "opencv-python-headless"]
prefer = "opencv-python-headless"

[[tool.vyp.substitutions]]
provides = "pillow"
packages = ["Pillow", "pillow-simd"]
prefer = "pillow-simd"
```

## Summary

| Concept | Description |
|---------|-------------|
| **provides** | Virtual capability name |
| **packages** | Interchangeable concrete packages |
| **prefer** | Which package to select when the capability is needed |
| **Filter** | Excludes non-preferred substitutes before resolution |
| **Strategy** | Handles conflicts when multiple substitutes are requested and no preference is set |
