use std::path::{Path, PathBuf};
use std::collections::HashSet;

/// Install files from a pre-extracted archive directory into site-packages.
/// Tries reflink (CoW clone) first, then hardlink, then copy as fallback.
///
/// When `file_list` is provided, iterates it directly instead of calling
/// walkdir — this skips the filesystem traversal entirely.
///
/// When `assume_fresh` is true, skips the per-file exists/remove_file check
/// (safe when site_packages was empty at install start — we only add files).
pub fn install_from_archive(
    archive_dir: &Path,
    site_packages: &Path,
    file_list: Option<&[PathBuf]>,
    assume_fresh: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut created_dirs: HashSet<Box<Path>> = HashSet::new();

    if let Some(files) = file_list {
        for rel in files {
            let source = archive_dir.join(rel);
            let target = site_packages.join(rel);

            if let Some(parent) = target.parent() {
                if created_dirs.insert(parent.into()) {
                    std::fs::create_dir_all(parent)?;
                }
            }

            if source.is_dir() {
                if created_dirs.insert(target.as_path().into()) {
                    std::fs::create_dir_all(&target)?;
                }
                continue;
            }

            if !assume_fresh && target.exists() {
                std::fs::remove_file(&target)?;
            }
            if reflink_copy::reflink(&source, &target).is_ok() {
                continue;
            }
            if std::fs::hard_link(&source, &target).is_ok() {
                continue;
            }
            std::fs::copy(&source, &target)?;
        }
    } else {
        for entry in walkdir::WalkDir::new(archive_dir) {
            let entry = entry?;
            let rel = entry.path().strip_prefix(archive_dir)?;
            if rel.as_os_str().is_empty() {
                continue;
            }
            let target = site_packages.join(rel);

            if entry.file_type().is_dir() {
                if created_dirs.insert(target.as_path().into()) {
                    std::fs::create_dir_all(&target)?;
                }
                continue;
            }

            if let Some(parent) = target.parent() {
                if created_dirs.insert(parent.into()) {
                    std::fs::create_dir_all(parent)?;
                }
            }

            if !assume_fresh && target.exists() {
                std::fs::remove_file(&target)?;
            }
            if reflink_copy::reflink(entry.path(), &target).is_ok() {
                continue;
            }
            if std::fs::hard_link(entry.path(), &target).is_ok() {
                continue;
            }
            std::fs::copy(entry.path(), &target)?;
        }
    }

    Ok(())
}
