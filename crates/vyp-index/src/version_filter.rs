//! Resolution-time version viability filtering.
//!
//! A version is only worth offering to the solver if it can actually be
//! installed on the target environment: it must have at least one
//! distribution that is (a) not yanked, (b) platform-compatible, and
//! (c) allowed by its `Requires-Python`. Filtering here means the solver
//! backtracks to an installable version instead of selecting one whose only
//! wheels can't be used — matching pip/uv behaviour.

use vyp_api::{Requirement, VypVersion};

use crate::in_memory_index::WheelInfo;
use crate::wheel_compat::PlatformTags;

/// Whether resolution-time wheel filtering is disabled via `VYP_NO_WHEEL_FILTER=1`.
/// Read once (the value is fixed for the process) to keep the hot path free of
/// per-call environment lookups.
pub fn wheel_filter_disabled() -> bool {
    static DISABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *DISABLED.get_or_init(|| std::env::var("VYP_NO_WHEEL_FILTER").is_ok_and(|v| v == "1"))
}

/// Decide whether a version is installable on the target environment given the
/// wheel files the index reported for it.
///
/// Returns `true` (keep the version) when:
/// - the index reported no file info for the version (we can't judge — don't
///   over-filter, e.g. HTML indexes or sdist-only listings), or
/// - at least one non-yanked wheel is platform-compatible and satisfies its
///   `Requires-Python`.
pub fn version_is_viable(
    wheels: Option<&[WheelInfo]>,
    tags: &PlatformTags,
    target_python: &VypVersion,
) -> bool {
    let Some(wheels) = wheels else {
        return true;
    };
    if wheels.is_empty() {
        return true;
    }

    // If the listing contains no actual `.whl` files (sdist-only), we can't
    // judge platform compatibility — keep it and let install surface any error.
    let has_any_wheel = wheels.iter().any(|w| w.filename.ends_with(".whl"));
    if !has_any_wheel {
        return true;
    }

    wheels.iter().any(|w| {
        w.filename.ends_with(".whl")
            && !w.yanked
            && tags.is_compatible(&w.filename)
            && requires_python_ok(w.requires_python.as_deref(), target_python)
    })
}

/// Filter a version list in place, dropping versions with no installable
/// distribution for the target environment.
///
/// `tags` and `target_python` are passed in pre-built (they are fixed for a
/// provider's lifetime) so this stays off the hot path's allocation budget.
/// `rp_cache` memoizes `Requires-Python` specifier evaluations, which would
/// otherwise re-parse the same specifier string for every wheel of every
/// version.
pub fn filter_versions_with(
    versions: &mut Vec<VypVersion>,
    wheel_info: &std::collections::HashMap<VypVersion, Vec<WheelInfo>>,
    tags: &PlatformTags,
    target_python: &VypVersion,
    rp_cache: &mut std::collections::HashMap<String, bool>,
) {
    if wheel_filter_disabled() {
        return;
    }
    versions.retain(|v| {
        version_is_viable_cached(
            wheel_info.get(v).map(|w| w.as_slice()),
            tags,
            target_python,
            rp_cache,
        )
    });
}

/// Like [`version_is_viable`] but memoizes `Requires-Python` checks.
fn version_is_viable_cached(
    wheels: Option<&[WheelInfo]>,
    tags: &PlatformTags,
    target_python: &VypVersion,
    rp_cache: &mut std::collections::HashMap<String, bool>,
) -> bool {
    let Some(wheels) = wheels else { return true };
    if wheels.is_empty() {
        return true;
    }
    let has_any_wheel = wheels.iter().any(|w| w.filename.ends_with(".whl"));
    if !has_any_wheel {
        return true;
    }
    wheels.iter().any(|w| {
        if !w.filename.ends_with(".whl") || w.yanked || !tags.is_compatible(&w.filename) {
            return false;
        }
        match &w.requires_python {
            None => true,
            Some(spec) => *rp_cache
                .entry(spec.clone())
                .or_insert_with(|| requires_python_ok(Some(spec), target_python)),
        }
    })
}

/// Evaluate a PEP 440 `Requires-Python` specifier set against the target
/// interpreter version. Absent or unparseable specifiers are treated as
/// "matches" (permissive) so we never wrongly drop a version on a parse miss.
pub fn requires_python_ok(requires_python: Option<&str>, target: &VypVersion) -> bool {
    let Some(spec) = requires_python else {
        return true;
    };
    let spec = spec.trim();
    if spec.is_empty() {
        return true;
    }
    // Reuse the requirement specifier parser by prefixing a dummy name.
    match format!("python{}", spec).parse::<Requirement>() {
        Ok(req) => {
            // Compare against the full version (e.g. 3.10.4). Requires-Python
            // constraints like ">=3.8" must hold for the target interpreter.
            req.constraints.iter().all(|c| c.satisfied_by(target))
        }
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyp_api::MarkerEnvironment;

    fn v(s: &str) -> VypVersion {
        s.parse().unwrap()
    }

    #[test]
    fn requires_python_basic() {
        assert!(requires_python_ok(Some(">=3.8"), &v("3.10.4")));
        assert!(!requires_python_ok(Some(">=3.11"), &v("3.10.4")));
        assert!(requires_python_ok(Some(">=3.8,<4"), &v("3.12.0")));
        assert!(!requires_python_ok(Some(">=3.8,<3.12"), &v("3.12.1")));
    }

    #[test]
    fn requires_python_absent_or_empty_matches() {
        assert!(requires_python_ok(None, &v("3.10.0")));
        assert!(requires_python_ok(Some(""), &v("3.10.0")));
        assert!(requires_python_ok(Some("  "), &v("3.10.0")));
    }

    #[test]
    fn requires_python_unparseable_is_permissive() {
        assert!(requires_python_ok(Some("not-a-spec"), &v("3.10.0")));
    }

    fn wheel(name: &str, rp: Option<&str>, yanked: bool) -> WheelInfo {
        WheelInfo {
            filename: name.to_string(),
            url: format!("https://example.com/{}", name),
            has_metadata: true,
            requires_python: rp.map(String::from),
            yanked,
            hashes: Default::default(),
        }
    }

    fn arm64_mac() -> MarkerEnvironment {
        let mut e = MarkerEnvironment::current();
        e.sys_platform = "darwin".into();
        e.platform_machine = "arm64".into();
        e.implementation_name = "cpython".into();
        e.python_version = "3.10".into();
        e.python_full_version = "3.10.4".into();
        e
    }

    #[test]
    fn viable_with_compatible_wheel() {
        let tags = PlatformTags::from_env(&arm64_mac());
        let ws = vec![wheel(
            "pkg-1.0-cp310-cp310-macosx_11_0_arm64.whl",
            None,
            false,
        )];
        assert!(version_is_viable(Some(&ws), &tags, &v("3.10.4")));
    }

    #[test]
    fn not_viable_when_only_incompatible_wheel() {
        let tags = PlatformTags::from_env(&arm64_mac());
        let ws = vec![wheel(
            "pkg-1.0-cp310-cp310-manylinux_2_17_x86_64.whl",
            None,
            false,
        )];
        assert!(!version_is_viable(Some(&ws), &tags, &v("3.10.4")));
    }

    #[test]
    fn not_viable_when_requires_python_excludes() {
        let tags = PlatformTags::from_env(&arm64_mac());
        // Pure-python wheel matches platform tags but Requires-Python rules it out.
        let ws = vec![wheel("pkg-1.0-py3-none-any.whl", Some(">=3.11"), false)];
        assert!(!version_is_viable(Some(&ws), &tags, &v("3.10.4")));
    }

    #[test]
    fn not_viable_when_all_yanked() {
        let tags = PlatformTags::from_env(&arm64_mac());
        let ws = vec![wheel("pkg-1.0-cp310-cp310-macosx_11_0_arm64.whl", None, true)];
        assert!(!version_is_viable(Some(&ws), &tags, &v("3.10.4")));
    }

    #[test]
    fn viable_when_no_file_info() {
        let tags = PlatformTags::from_env(&arm64_mac());
        assert!(version_is_viable(None, &tags, &v("3.10.4")));
        assert!(version_is_viable(Some(&[]), &tags, &v("3.10.4")));
    }

    #[test]
    fn viable_when_sdist_only() {
        let tags = PlatformTags::from_env(&arm64_mac());
        let ws = vec![WheelInfo {
            filename: "pkg-1.0.tar.gz".to_string(),
            url: "https://example.com/pkg-1.0.tar.gz".to_string(),
            has_metadata: false,
            requires_python: None,
            yanked: false,
            hashes: Default::default(),
        }];
        assert!(version_is_viable(Some(&ws), &tags, &v("3.10.4")));
    }
}
