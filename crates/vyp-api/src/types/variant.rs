//! PEP 825 Wheel Variant types.
//!
//! These types model the `variant.json` metadata that PEP 825 proposes
//! embedding alongside wheel files. The implementation follows the draft
//! spec and will be updated as the PEP is finalized.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single variant property describing a machine capability,
/// e.g. `(x86, microarchitecture, x86_64_v3)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VariantProperty {
    pub namespace: String,
    pub feature: String,
    pub value: String,
}

impl std::fmt::Display for VariantProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.namespace, self.feature, self.value)
    }
}

/// Full `variant.json` metadata for a package on an index.
///
/// Maps variant labels to the properties they require. The ordering
/// of `default_priorities` determines which variant is preferred when
/// multiple are compatible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantMetadata {
    /// Schema version, e.g. `"1.0"`.
    #[serde(rename = "schema-version")]
    pub schema: String,

    /// Default priority order for selecting among compatible variants.
    #[serde(rename = "default-priorities")]
    pub default_priorities: VariantPriorities,

    /// Variant label -> namespace -> feature -> list of acceptable values.
    ///
    /// `{ "x86_64_v3": { "x86": { "microarchitecture": ["x86_64_v3", "x86_64_v4"] } } }`
    #[serde(default)]
    pub variants: HashMap<String, HashMap<String, HashMap<String, Vec<String>>>>,
}

/// Priority ordering for variant selection.
///
/// When multiple variant labels are compatible with the current system,
/// these priorities determine which one is chosen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantPriorities {
    /// Ordered namespace preferences (higher index = higher priority).
    #[serde(default)]
    pub namespace: Vec<String>,

    /// Per-namespace feature ordering.
    #[serde(default)]
    pub feature: HashMap<String, Vec<String>>,

    /// Per-namespace, per-feature property value ordering.
    #[serde(default)]
    pub property: HashMap<String, HashMap<String, Vec<String>>>,
}

/// Describes the variant data that may be embedded in a `pylock.toml`
/// package entry per PEP 825's lockfile integration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantDescriptor {
    /// The variant label this wheel was built for (e.g. `"x86_64_v3"`).
    pub label: String,

    /// The properties this variant requires.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<VariantProperty>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variant_property_display() {
        let prop = VariantProperty {
            namespace: "x86".into(),
            feature: "microarchitecture".into(),
            value: "x86_64_v3".into(),
        };
        assert_eq!(prop.to_string(), "x86.microarchitecture.x86_64_v3");
    }

    #[test]
    fn test_variant_metadata_roundtrip() {
        let json = r#"{
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
                }
            }
        }"#;

        let meta: VariantMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.schema, "1.0");
        assert!(meta.variants.contains_key("x86_64_v3"));

        let serialized = serde_json::to_string(&meta).unwrap();
        let roundtrip: VariantMetadata = serde_json::from_str(&serialized).unwrap();
        assert_eq!(roundtrip.schema, meta.schema);
    }
}
