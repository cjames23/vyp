use vyp_api::MarkerEnvironment;

/// Platform tags describing the current environment, used to filter
/// incompatible wheels before selection.
#[derive(Debug, Clone)]
pub struct PlatformTags {
    python_tags: Vec<String>,
    abi_tags: Vec<String>,
    platform_tags: Vec<String>,
}

impl PlatformTags {
    /// Build platform tags from a detected marker environment.
    pub fn from_env(env: &MarkerEnvironment) -> Self {
        let (major, minor) = parse_python_version(&env.python_version);
        let impl_name = env.implementation_name.to_lowercase();

        let python_tags = build_python_tags(&impl_name, major, minor);
        let abi_tags = build_abi_tags(&impl_name, major, minor);
        let platform_tags = build_platform_tags(
            &env.sys_platform,
            &env.platform_machine,
            major,
            minor,
        );

        Self { python_tags, abi_tags, platform_tags }
    }

    /// Check whether a wheel filename is compatible with this platform.
    /// Returns `true` for compatible wheels and for filenames that can't be parsed.
    pub fn is_compatible(&self, filename: &str) -> bool {
        let Some(stem) = filename.strip_suffix(".whl") else {
            return false;
        };
        let parts: Vec<&str> = stem.split('-').collect();
        let (py_tag, abi_tag, plat_tag) = match parts.len() {
            5 => (parts[2], parts[3], parts[4]),
            6 => {
                if parts[2].starts_with(|c: char| c.is_ascii_digit()) {
                    (parts[3], parts[4], parts[5])
                } else {
                    (parts[2], parts[3], parts[4])
                }
            }
            7 => (parts[3], parts[4], parts[5]),
            _ => return true,
        };

        let py_ok = any_tag_matches(py_tag, &self.python_tags);
        let abi_ok = any_tag_matches(abi_tag, &self.abi_tags);
        let plat_ok = any_tag_matches(plat_tag, &self.platform_tags);

        py_ok && abi_ok && plat_ok
    }

    /// Score a wheel for preference ordering. Higher is better.
    /// Prefers native > universal > pure-python, and specific ABI > generic.
    pub fn compatibility_score(&self, filename: &str) -> u32 {
        let Some(stem) = filename.strip_suffix(".whl") else {
            return 0;
        };
        let parts: Vec<&str> = stem.split('-').collect();
        let (py_tag, abi_tag, plat_tag) = match parts.len() {
            5 => (parts[2], parts[3], parts[4]),
            6 => {
                if parts[2].starts_with(|c: char| c.is_ascii_digit()) {
                    (parts[3], parts[4], parts[5])
                } else {
                    (parts[2], parts[3], parts[4])
                }
            }
            7 => (parts[3], parts[4], parts[5]),
            _ => return 0,
        };

        let mut score: u32 = 0;

        if py_tag.starts_with("cp") {
            score += 100;
        } else if py_tag.starts_with("py") {
            score += 50;
        }

        if abi_tag.starts_with("cp") {
            score += 50;
        } else if abi_tag == "abi3" {
            score += 30;
        } else if abi_tag == "none" {
            score += 10;
        }

        for sub in plat_tag.split('.') {
            if sub == "any" {
                score += 5;
            } else if sub.contains("universal2") {
                score += 20;
            } else if !sub.contains("x86_64") && !sub.contains("i686") {
                score += 30;
            }
        }

        score
    }
}

fn any_tag_matches(wheel_tag: &str, supported: &[String]) -> bool {
    for sub in wheel_tag.split('.') {
        if supported.iter().any(|s| s == sub) {
            return true;
        }
    }
    false
}

fn parse_python_version(version_str: &str) -> (u32, u32) {
    let parts: Vec<&str> = version_str.split('.').collect();
    let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(3);
    let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(12);
    (major, minor)
}

fn build_python_tags(impl_name: &str, major: u32, minor: u32) -> Vec<String> {
    let mut tags = Vec::new();

    let cp_prefix = if impl_name == "cpython" { "cp" } else { impl_name };

    tags.push(format!("{}{}{}", cp_prefix, major, minor));

    if impl_name == "cpython" {
        tags.push("abi3".to_string());
    }

    for m in (0..=minor).rev() {
        tags.push(format!("py{}{}", major, m));
    }
    tags.push(format!("py{}", major));

    tags
}

fn build_abi_tags(impl_name: &str, major: u32, minor: u32) -> Vec<String> {
    let mut tags = Vec::new();

    let cp_prefix = if impl_name == "cpython" { "cp" } else { impl_name };
    tags.push(format!("{}{}{}", cp_prefix, major, minor));

    if impl_name == "cpython" {
        tags.push("abi3".to_string());
    }

    tags.push("none".to_string());
    tags
}

fn build_platform_tags(sys_platform: &str, machine: &str, _major: u32, _minor: u32) -> Vec<String> {
    let mut tags = Vec::new();

    match sys_platform {
        "darwin" => {
            let arch = match machine {
                "arm64" | "aarch64" => "arm64",
                _ => machine,
            };

            for macos_minor in (0..=0).chain((0..=16).rev()) {
                tags.push(format!("macosx_11_{}_universal2", macos_minor));
                tags.push(format!("macosx_10_{}_universal2", macos_minor));
            }

            for macos_major in [14, 13, 12, 11] {
                for macos_minor in (0..=4).rev() {
                    tags.push(format!("macosx_{}_{}_universal2", macos_major, macos_minor));
                    tags.push(format!("macosx_{}_{}_{}", macos_major, macos_minor, arch));
                }
            }
            for macos_minor in (0..=16).rev() {
                tags.push(format!("macosx_10_{}_universal2", macos_minor));
                if arch == "x86_64" || arch == "arm64" {
                    tags.push(format!("macosx_10_{}_{}", macos_minor, arch));
                }
                tags.push(format!("macosx_10_{}_universal", macos_minor));
            }
        }
        "linux" => {
            let linux_arch = match machine {
                "arm64" => "aarch64",
                _ => machine,
            };

            for glibc_minor in (17..=50).rev() {
                tags.push(format!("manylinux_2_{}_{}", glibc_minor, linux_arch));
            }
            tags.push(format!("manylinux2014_{}", linux_arch));
            tags.push(format!("manylinux2010_{}", linux_arch));
            tags.push(format!("manylinux1_{}", linux_arch));

            for musllinux_minor in (1..=10).rev() {
                tags.push(format!("musllinux_1_{}_{}", musllinux_minor, linux_arch));
            }

            tags.push(format!("linux_{}", linux_arch));
        }
        "win32" => {
            let win_arch = match machine {
                "x86_64" | "AMD64" => "amd64",
                "arm64" | "aarch64" => "arm64",
                _ => machine,
            };
            tags.push(format!("win_{}", win_arch));
            if win_arch == "amd64" {
                tags.push("win32".to_string());
            }
        }
        _ => {}
    }

    tags.push("any".to_string());
    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arm64_mac_env() -> MarkerEnvironment {
        MarkerEnvironment {
            os_name: "posix".to_string(),
            sys_platform: "darwin".to_string(),
            platform_system: "Darwin".to_string(),
            platform_machine: "arm64".to_string(),
            platform_release: String::new(),
            platform_version: String::new(),
            platform_python_implementation: "CPython".to_string(),
            implementation_name: "cpython".to_string(),
            python_version: "3.10".to_string(),
            python_full_version: "3.10.0".to_string(),
            implementation_version: "3.10.0".to_string(),
        }
    }

    #[test]
    fn pure_python_wheel_compatible() {
        let tags = PlatformTags::from_env(&arm64_mac_env());
        assert!(tags.is_compatible("requests-2.32.5-py3-none-any.whl"));
    }

    #[test]
    fn arm64_mac_wheel_compatible() {
        let tags = PlatformTags::from_env(&arm64_mac_env());
        assert!(tags.is_compatible(
            "pydantic_core-2.41.5-cp310-cp310-macosx_11_0_arm64.whl"
        ));
    }

    #[test]
    fn x86_64_mac_wheel_incompatible_on_arm64() {
        let tags = PlatformTags::from_env(&arm64_mac_env());
        assert!(!tags.is_compatible(
            "pydantic_core-2.41.5-cp310-cp310-macosx_10_12_x86_64.whl"
        ));
    }

    #[test]
    fn universal2_wheel_compatible_on_arm64() {
        let tags = PlatformTags::from_env(&arm64_mac_env());
        assert!(tags.is_compatible(
            "pydantic_core-2.41.5-cp310-cp310-macosx_11_0_universal2.whl"
        ));
    }

    #[test]
    fn linux_wheel_incompatible_on_mac() {
        let tags = PlatformTags::from_env(&arm64_mac_env());
        assert!(!tags.is_compatible(
            "pydantic_core-2.41.5-cp310-cp310-manylinux_2_17_x86_64.manylinux2014_x86_64.whl"
        ));
    }

    #[test]
    fn wrong_python_version_incompatible() {
        let tags = PlatformTags::from_env(&arm64_mac_env());
        assert!(!tags.is_compatible(
            "pydantic_core-2.41.5-cp312-cp312-macosx_11_0_arm64.whl"
        ));
    }

    #[test]
    fn abi3_wheel_compatible() {
        let tags = PlatformTags::from_env(&arm64_mac_env());
        assert!(tags.is_compatible(
            "some_package-1.0.0-cp310-abi3-macosx_11_0_arm64.whl"
        ));
    }

    #[test]
    fn py3_none_any_compatible() {
        let tags = PlatformTags::from_env(&arm64_mac_env());
        assert!(tags.is_compatible("click-8.3.1-py3-none-any.whl"));
    }
}
