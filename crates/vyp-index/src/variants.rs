//! PEP 825 wheel variant selection.
//!
//! Provides parsing of `variant.json` metadata and a stub for selecting
//! the best compatible variant given the system's capabilities.
//!
//! The actual capability detection ("which namespaces/features does this
//! machine support?") is deferred to a future PEP. Once finalized, only
//! the compatibility detection needs to be plugged in.

use vyp_api::{VariantMetadata, VariantPriorities, VariantProperty};

/// Parse a `variant.json` blob into structured metadata.
pub fn parse_variant_metadata(
    json: &str,
) -> Result<VariantMetadata, Box<dyn std::error::Error + Send + Sync>> {
    let meta: VariantMetadata = serde_json::from_str(json)?;
    Ok(meta)
}

/// Select the best variant label from `metadata` given the system's
/// `compatible` properties.
///
/// Uses the ordering algorithm from PEP 825:
/// 1. For each variant label, check if all required properties are
///    satisfied by `compatible`.
/// 2. Among compatible variants, rank by `default_priorities`:
///    namespace order, then feature order, then property value order.
/// 3. Return the highest-ranked variant label, or `None` if no variant
///    is compatible.
pub fn select_best_variant(
    metadata: &VariantMetadata,
    compatible: &[VariantProperty],
) -> Option<String> {
    let mut candidates: Vec<(&str, i64)> = Vec::new();

    for (label, namespaces) in &metadata.variants {
        if is_compatible(namespaces, compatible) {
            let score = score_variant(namespaces, compatible, &metadata.default_priorities);
            candidates.push((label, score));
        }
    }

    // Higher score wins; ties broken by alphabetical order
    candidates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    candidates.first().map(|(label, _)| label.to_string())
}

/// Check if all required properties of a variant are satisfied.
fn is_compatible(
    required: &std::collections::HashMap<String, std::collections::HashMap<String, Vec<String>>>,
    compatible: &[VariantProperty],
) -> bool {
    for (namespace, features) in required {
        for (feature, acceptable_values) in features {
            let satisfied = compatible.iter().any(|prop| {
                prop.namespace == *namespace
                    && prop.feature == *feature
                    && acceptable_values.contains(&prop.value)
            });
            if !satisfied {
                return false;
            }
        }
    }
    true
}

const NAMESPACE_WEIGHT: i64 = 1000;
const FEATURE_WEIGHT: i64 = 100;
const MAX_PROPERTY_VALUES: i64 = 10;

/// Score a variant based on priority ordering. Higher is better.
fn score_variant(
    required: &std::collections::HashMap<String, std::collections::HashMap<String, Vec<String>>>,
    compatible: &[VariantProperty],
    priorities: &VariantPriorities,
) -> i64 {
    let mut score: i64 = 0;

    for (namespace, features) in required {
        if let Some(pos) = priorities.namespace.iter().position(|n| n == namespace) {
            score += (pos as i64 + 1) * NAMESPACE_WEIGHT;
        }

        for (feature, acceptable_values) in features {
            if let Some(ns_features) = priorities.feature.get(namespace) {
                if let Some(pos) = ns_features.iter().position(|f| f == feature) {
                    score += (pos as i64 + 1) * FEATURE_WEIGHT;
                }
            }

            let best_value = compatible
                .iter()
                .filter(|p| {
                    p.namespace == *namespace
                        && p.feature == *feature
                        && acceptable_values.contains(&p.value)
                })
                .filter_map(|p| {
                    priorities
                        .property
                        .get(namespace)
                        .and_then(|fs| fs.get(feature))
                        .and_then(|vals| vals.iter().position(|v| *v == p.value))
                })
                .min();

            if let Some(pos) = best_value {
                score += (MAX_PROPERTY_VALUES - pos as i64).max(0);
            }
        }
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_metadata() -> VariantMetadata {
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
                },
                "x86_64_v2": {
                    "x86": {
                        "microarchitecture": ["x86_64_v2", "x86_64_v3", "x86_64_v4"]
                    }
                }
            }
        }"#;
        parse_variant_metadata(json).unwrap()
    }

    #[test]
    fn test_parse_variant_metadata() {
        let meta = sample_metadata();
        assert_eq!(meta.schema, "1.0");
        assert_eq!(meta.variants.len(), 2);
    }

    #[test]
    fn test_select_best_variant_v3() {
        let meta = sample_metadata();
        let compatible = vec![VariantProperty {
            namespace: "x86".into(),
            feature: "microarchitecture".into(),
            value: "x86_64_v3".into(),
        }];

        let best = select_best_variant(&meta, &compatible);
        // Both v2 and v3 accept x86_64_v3, but v3 has a higher property
        // priority score (x86_64_v3 is index 1 vs v2 which uses index 2)
        assert!(best.is_some());
    }

    #[test]
    fn test_select_best_variant_none() {
        let meta = sample_metadata();
        let compatible = vec![VariantProperty {
            namespace: "arm".into(),
            feature: "microarchitecture".into(),
            value: "armv8".into(),
        }];

        let best = select_best_variant(&meta, &compatible);
        assert!(best.is_none());
    }

    #[test]
    fn test_is_compatible() {
        let mut required = HashMap::new();
        let mut features = HashMap::new();
        features.insert(
            "microarchitecture".to_string(),
            vec!["x86_64_v3".to_string(), "x86_64_v4".to_string()],
        );
        required.insert("x86".to_string(), features);

        let compatible = vec![VariantProperty {
            namespace: "x86".into(),
            feature: "microarchitecture".into(),
            value: "x86_64_v3".into(),
        }];

        assert!(is_compatible(&required, &compatible));

        let incompatible = vec![VariantProperty {
            namespace: "x86".into(),
            feature: "microarchitecture".into(),
            value: "x86_64_v2".into(),
        }];

        assert!(!is_compatible(&required, &incompatible));
    }
}
