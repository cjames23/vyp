# PyTorch Integration

PyTorch publishes GPU-specific wheels to its own indexes, not PyPI. vyp supports automatic backend detection, manual backend selection, and per-platform configuration so you can use PyTorch with CUDA, ROCm, XPU, or CPU across different environments.

## The Problem

PyPI hosts CPU-only PyTorch wheels. GPU builds (CUDA, ROCm, XPU) are published to:

- `https://download.pytorch.org/whl/cu128`
- `https://download.pytorch.org/whl/rocm6.4`
- `https://download.pytorch.org/whl/xpu`
- `https://download.pytorch.org/whl/cpu`

vyp must know which index to use for `torch`, `torchvision`, `torchaudio`, and related packages.

## Quick Start

Add PyTorch with auto-detection:

```toml
[tool.vyp]
torch-backend = "auto"
```

```bash
vyp add torch torchvision
```

vyp detects your GPU (NVIDIA, AMD, Intel) and selects the appropriate index. On machines without a GPU, it falls back to CPU.

## torch-backend Configuration

Set the backend in `pyproject.toml`:

```toml
[tool.vyp]
torch-backend = "auto"   # Detect GPU automatically
```

Or choose explicitly:

```toml
[tool.vyp]
torch-backend = "cpu"    # CPU-only
torch-backend = "cu128"  # CUDA 12.8
torch-backend = "cu126"  # CUDA 12.6
torch-backend = "rocm6"  # AMD ROCm 6.x
torch-backend = "xpu"    # Intel XPU
```

### Supported backends

| Backend | Index URL | Use case |
|---------|------------|----------|
| `cpu` | `https://download.pytorch.org/whl/cpu` | CPU-only, no GPU |
| `cu118` | `https://download.pytorch.org/whl/cu118` | CUDA 11.8 |
| `cu121` | `https://download.pytorch.org/whl/cu121` | CUDA 12.1 |
| `cu124` | `https://download.pytorch.org/whl/cu124` | CUDA 12.4 |
| `cu126` | `https://download.pytorch.org/whl/cu126` | CUDA 12.6 |
| `cu128` | `https://download.pytorch.org/whl/cu128` | CUDA 12.8 |
| `cu130` | `https://download.pytorch.org/whl/cu130` | CUDA 13.0 |
| `rocm6` | `https://download.pytorch.org/whl/rocm6.4` | AMD ROCm 6.x |
| `xpu` | `https://download.pytorch.org/whl/xpu` | Intel XPU |
| `auto` | (detected) | Detect NVIDIA/AMD/Intel, else CPU |

!!! tip "CUDA driver mapping"
    When using `auto`, vyp maps your NVIDIA driver version to a CUDA tag. Driver 560+ → cu128, 550+ → cu126, etc.

## CLI Override: --torch-backend

Override the configured backend for a single command:

```bash
vyp add torch --torch-backend cpu
vyp lock --torch-backend cu128
```

Useful for CI (force CPU) or when generating locks for a specific GPU target.

## Auto-Detection

When `torch-backend = "auto"`:

1. **NVIDIA CUDA** — vyp runs `nvidia-smi` or reads `/proc/driver/nvidia/version` and maps driver version to a CUDA tag
2. **AMD ROCm** — vyp checks `rocm-smi` or `/opt/rocm/bin/rocm_agent_enumerator`
3. **Intel XPU** — vyp checks `xpu-smi discovery`
4. **Fallback** — If no GPU is found, uses CPU

!!! note "Detection is best-effort"
    Auto-detection works on typical Linux/Windows setups. For custom environments, set `torch-backend` explicitly.

## Per-Platform Configuration

For projects that need different PyTorch backends per platform (e.g. CUDA on Linux, CPU on macOS), use `[[tool.vyp.extra-index]]` with `explicit = true` and `[tool.vyp.sources]` with environment markers:

```toml
[tool.vyp]

[[tool.vyp.extra-index]]
name = "pytorch-cpu"
url = "https://download.pytorch.org/whl/cpu"
explicit = true

[[tool.vyp.extra-index]]
name = "pytorch-cu128"
url = "https://download.pytorch.org/whl/cu128"
explicit = true

[tool.vyp.sources]
torch = [
  { index = "pytorch-cu128", marker = "sys_platform == 'linux'" },
  { index = "pytorch-cpu", marker = "sys_platform == 'darwin'" },
]
torchvision = [
  { index = "pytorch-cu128", marker = "sys_platform == 'linux'" },
  { index = "pytorch-cpu", marker = "sys_platform == 'darwin'" },
]
torchaudio = [
  { index = "pytorch-cu128", marker = "sys_platform == 'linux'" },
  { index = "pytorch-cpu", marker = "sys_platform == 'darwin'" },
]
```

!!! warning "Marker evaluation"
    Marker evaluation in sources is a refinement. Currently, vyp may use the first matching entry. For full control, use `torch-backend` or `--torch-backend` per environment.

### Tabbed examples by platform

=== "Linux + CUDA 12.8"
    ```toml
    [tool.vyp]
    torch-backend = "cu128"
    ```

=== "macOS (CPU only)"
    ```toml
    [tool.vyp]
    torch-backend = "cpu"
    ```

=== "Linux + ROCm"
    ```toml
    [tool.vyp]
    torch-backend = "rocm6"
    ```

=== "Multi-platform (sources)"
    ```toml
    [tool.vyp]
    [[tool.vyp.extra-index]]
    name = "pytorch-cu128"
    url = "https://download.pytorch.org/whl/cu128"
    explicit = true

    [[tool.vyp.extra-index]]
    name = "pytorch-cpu"
    url = "https://download.pytorch.org/whl/cpu"
    explicit = true

    [tool.vyp.sources]
    torch = [
      { index = "pytorch-cu128", marker = "sys_platform == 'linux'" },
      { index = "pytorch-cpu", marker = "sys_platform == 'darwin'" },
    ]
    torchvision = [
      { index = "pytorch-cu128", marker = "sys_platform == 'linux'" },
      { index = "pytorch-cpu", marker = "sys_platform == 'darwin'" },
    ]
    torchaudio = [
      { index = "pytorch-cu128", marker = "sys_platform == 'linux'" },
      { index = "pytorch-cpu", marker = "sys_platform == 'darwin'" },
    ]
    ```

## Packages Routed to PyTorch Index

When a torch backend is active, vyp routes these packages to the PyTorch index:

- `torch`
- `torchvision`
- `torchaudio`
- `pytorch-triton`
- `pytorch-triton-rocm`
- `pytorch-triton-xpu`
- `torch-model-archiver`
- `torch-tb-profiler`

All other packages come from PyPI or your configured indexes.

## CPU-Only Workflow

For CI, notebooks, or machines without a GPU:

```toml
[tool.vyp]
torch-backend = "cpu"
```

```bash
vyp add torch torchvision --torch-backend cpu
```

## Next Steps

- [Package Indexes](indexes.md) — Configuring indexes and per-package sources
- [Configuration](../configuration/pyproject.md) — Full `[tool.vyp]` reference
