# Configuring Package Indexes

vyp fetches package metadata and wheels from configurable indexes. By default it uses PyPI, but you can add extra indexes, restrict which packages use them, and route specific packages (like PyTorch) to custom URLs.

## Primary Index: index-url

The primary index is PyPI by default:

```toml
[tool.vyp]
index-url = "https://pypi.org/simple"
```

Override it for a private or mirror index:

```toml
[tool.vyp]
index-url = "https://pypi.my-company.com/simple"
```

!!! note "Simple repository format"
    vyp expects [PEP 503](https://peps.python.org/pep-0503/) simple repository format (the standard PyPI layout).

## Extra Indexes

Add additional indexes with `[[tool.vyp.extra-index]]`:

```toml
[[tool.vyp.extra-index]]
name = "pytorch-cu128"
url = "https://download.pytorch.org/whl/cu128"
explicit = false
```

### Extra index fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique identifier for this index |
| `url` | Yes | Base URL of the simple repository |
| `explicit` | No | If `true`, only used for packages in `[tool.vyp.sources]` (default: `false`) |

### explicit = true

When `explicit = true`, the index is **scoped**: vyp queries it only for the packages routed to it via `[tool.vyp.sources]` **and their direct and transitive dependencies**. It is never consulted for unrelated dependencies. This prevents "index mixing" where a malicious or out-of-date package on a secondary index could shadow a legitimate PyPI package.

```toml
[[tool.vyp.extra-index]]
name = "internal"
url = "https://artifacts.my-company.com/pypi/simple"
explicit = true

[tool.vyp.sources]
my-internal-pkg = [{ index = "internal" }]
```

`my-internal-pkg` and everything only it needs resolve from `internal`; the rest of your tree stays on PyPI.

!!! note "Transitive scoping"
    The scope is computed dynamically as the dependency graph is walked, not from a fixed list. Routing `torch` pulls in `torch`'s whole closure (`sympy`, `filelock`, the matching `nvidia-*` builds, …) from the named index — without you having to enumerate them.

!!! info "Overlap policy — the default index wins"
    If a package is reachable from both a scoped root (e.g. `torch`) **and** an ordinary dependency, it resolves from the **default index (PyPI)**, not the named index. Only packages reachable *exclusively* through the named index's roots are routed there. The declared root packages themselves are always authoritative to their index. In rare diamond-shaped graphs, route the shared package explicitly in `[tool.vyp.sources]` to force a decision.

!!! tip "Security best practice"
    Use `explicit = true` for any non-PyPI index. Only PyPI is trusted by default for all packages.

## Per-Package Routing: sources

Route specific packages to specific indexes with `[tool.vyp.sources]`:

```toml
[tool.vyp.sources]
torch = [{ index = "pytorch-cu128" }]
torchvision = [{ index = "pytorch-cu128" }]
my-private-pkg = [{ index = "internal" }]
```

### With environment markers

Use PEP 508 environment markers to route per platform:

```toml
[tool.vyp.sources]
torch = [
  { index = "pytorch-cu128", marker = "sys_platform == 'linux'" },
  { index = "pytorch-cpu", marker = "sys_platform == 'darwin'" },
]
```

!!! note "Marker evaluation"
    Marker support in sources may be refined in future versions. For now, the first matching entry is typically used.

## Environment Markers in Sources

Markers use standard PEP 508 syntax:

| Marker | Example |
|--------|---------|
| `sys_platform` | `sys_platform == 'linux'`, `sys_platform == 'darwin'`, `sys_platform == 'win32'` |
| `platform_machine` | `platform_machine == 'x86_64'`, `platform_machine == 'aarch64'` |
| `python_version` | `python_version >= '3.10'` |

```toml
[tool.vyp.sources]
some-pkg = [
  { index = "linux-builds", marker = "sys_platform == 'linux'" },
  { index = "mac-builds", marker = "sys_platform == 'darwin'" },
]
```

## Private and Corporate Indexes

### Basic auth (future)

vyp may support index credentials via environment variables or keyring. For now, use URLs with embedded credentials only in secure contexts:

```toml
index-url = "https://user:token@pypi.my-company.com/simple"
```

!!! warning "Credentials in pyproject.toml"
    Avoid committing credentials. Prefer environment variables or a secrets manager when available.

### Artifactory / Nexus / DevPI

Point `url` to your artifact server's simple repository endpoint:

```toml
[[tool.vyp.extra-index]]
name = "corporate"
url = "https://artifactory.my-company.com/artifactory/api/pypi/pypi-virtual/simple"
explicit = true

[tool.vyp.sources]
internal-lib = [{ index = "corporate" }]
```

## Index Priority

When multiple indexes provide the same package:

1. **Scoped indexes win for packages in their scope**: an `explicit` index is consulted first for any package within its routed roots' transitive closure
2. **Primary index** (`index-url`) serves everything else
3. **Extra non-explicit indexes** are queried in order as additional sources
4. **Overlap goes to the default index**: a package reachable from both a scoped root and an ordinary dependency resolves from PyPI, not the named index (declared roots themselves always use their index)

The resolver merges version information from all providers and picks a single version according to the resolution strategy.

## PyTorch Index Pattern

A common pattern for PyTorch:

```toml
[tool.vyp]
torch-backend = "auto"

[[tool.vyp.extra-index]]
name = "pytorch-cu128"
url = "https://download.pytorch.org/whl/cu128"
explicit = true

[tool.vyp.sources]
torch = [{ index = "pytorch-cu128" }]
torchvision = [{ index = "pytorch-cu128" }]
torchaudio = [{ index = "pytorch-cu128" }]
```

Or use `torch-backend` alone—vyp injects the PyTorch index automatically. See [PyTorch](pytorch.md) for details.

## Next Steps

- [PyTorch](pytorch.md) — PyTorch index and backend selection
- [Configuration](../configuration/pyproject.md) — Full `[tool.vyp]` reference
