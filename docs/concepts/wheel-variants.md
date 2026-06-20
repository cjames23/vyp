# Wheel Variants (PEP 825)

vyp includes **groundwork** for PEP 825 wheel variants—a proposed standard for platform-specific wheel selection (e.g., x86_64_v3 vs x86_64_v2). The types and selection logic are in place; full integration awaits PEP finalization.

## What PEP 825 Proposes

PEP 825 aims to standardize how Python packages ship multiple wheel variants for different machine capabilities. For example:

- **x86_64_v2**: Baseline x86-64
- **x86_64_v3**: AVX2, BMI1, etc.
- **x86_64_v4**: AVX-512 and more

Instead of one generic wheel, a package could ship wheels optimized for each variant. The installer would select the best compatible variant for the current machine.

## Variant Labels in Wheel Filenames

Wheel filenames would include a variant label, e.g.:

```
numpy-1.26.4-cp312-cp312-x86_64_v3.whl
                              ^^^^^^^^^
                              variant label
```

The exact format is still under discussion in the PEP.

## variant.json Metadata Structure

Each package (or index) would provide a `variant.json` that describes:

- **Schema version**
- **Default priorities** for selecting among compatible variants
- **Variants**: mapping from variant label to required properties

!!! example "variant.json (simplified)"
    ```json
    {
      "schema-version": "1.0",
      "default-priorities": {
        "namespace": ["x86"],
        "feature": {"x86": ["microarchitecture"]},
        "property": {"x86": {"microarchitecture": ["x86_64_v4", "x86_64_v3", "x86_64_v2"]}}
      },
      "variants": {
        "x86_64_v3": {
          "x86": {
            "microarchitecture": ["x86_64_v3", "x86_64_v4"]
          }
        },
        "x86_64_v2": {
          "x86": {
            "microarchitecture": ["x86_64_v2", "x86_64_v3", "x86_64_v4"]
          }
        }
      }
    }
    ```

## vyp Types

vyp defines these types for variant handling:

### VariantProperty

A single machine capability: `(namespace, feature, value)`.

```rust
// Example: x86.microarchitecture.x86_64_v3
VariantProperty {
    namespace: "x86",
    feature: "microarchitecture",
    value: "x86_64_v3"
}
```

### VariantMetadata

Full `variant.json` structure:

- **schema**: Schema version string
- **default_priorities**: Ordering for selection
- **variants**: Map from variant label to required properties (namespace → feature → acceptable values)

### VariantPriorities

Determines which variant is preferred when multiple are compatible:

- **namespace**: Ordered list (higher index = higher priority)
- **feature**: Per-namespace feature ordering
- **property**: Per-namespace, per-feature value ordering (e.g., x86_64_v4 > x86_64_v3 > x86_64_v2)

### VariantDescriptor

Stored in the lock file when a variant wheel is selected:

- **label**: The variant label (e.g., `"x86_64_v3"`)
- **requires**: The properties this variant requires

## How vyp Will Select the Best Variant

The selection algorithm (already implemented as groundwork):

1. **Compatibility**: For each variant label, check if all required properties are satisfied by the system's `compatible` properties.
2. **Scoring**: Among compatible variants, score by `default_priorities` (namespace, feature, property value order).
3. **Result**: Return the highest-ranked variant label, or `None` if no variant is compatible.

!!! abstract "Selection flow"
    ```
    System reports: compatible = [x86.microarchitecture.x86_64_v3]
         ↓
    Filter variants: x86_64_v3 ✓, x86_64_v2 ✓ (both accept x86_64_v3)
         ↓
    Score by priorities: x86_64_v3 ranks higher (more specific)
         ↓
    Select: "x86_64_v3"
    ```

## Current Status

| Component | Status |
|-----------|--------|
| **Types** (VariantProperty, VariantMetadata, VariantPriorities) | Implemented |
| **Parsing** (variant.json) | Implemented |
| **Selection logic** (select_best_variant) | Implemented |
| **Lock file** (VariantDescriptor in packages) | Structure in place |
| **Capability detection** | Deferred—awaits PEP for "how to detect system capabilities" |
| **Index integration** | Groundwork laid; full wiring pending PEP finalization |

!!! info "Awaiting PEP finalization"
    The actual capability detection ("which namespaces/features does this machine support?") is deferred to a future PEP. Once the PEP is finalized, vyp can plug in the compatibility detection and complete variant selection.

## Summary

vyp has laid the groundwork for PEP 825 wheel variants:

- **variant.json** parsing and structured types
- **Selection algorithm** for choosing the best compatible variant
- **Lock file** support for variant descriptors

Full end-to-end variant selection will be enabled once the PEP is finalized and capability detection is standardized.
