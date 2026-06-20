use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering as AtomicOrdering};
use std::time::{Duration, Instant};
use sha2::{Sha256, Digest};
use vyp_api::{MarkerEnvironment, MarkerTree, Requirement};
use vyp_core::{ResolutionResult, ResolverBuilder, ResolveProgress};
use vyp_index::PlatformTags;
use futures::stream::{self, StreamExt};
use rayon::prelude::*;

use crate::cache::wheel_cache::WheelCache;
use crate::cache::linker;
use crate::config::settings::VypConfig;
use crate::lock::lockfile::{LockFile, PyLockPackage};
use crate::lock::universal::{merge_universal_results, parse_python_version_from_marker, UniversalPackageEntry};

const DEFAULT_INDEX: &str = "https://pypi.org/simple";
const DEFAULT_CONCURRENT_DOWNLOADS: usize = 50;
const MAX_CONCURRENT_INSTALLS: usize = 32;

fn concurrent_downloads() -> usize {
    std::env::var("VYP_CONCURRENT_DOWNLOADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_CONCURRENT_DOWNLOADS)
}

fn concurrent_installs() -> usize {
    let from_env = std::env::var("VYP_CONCURRENT_INSTALLS")
        .ok()
        .and_then(|s| s.parse().ok());
    if let Some(n) = from_env {
        return n;
    }
    let available = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(8);
    available.min(MAX_CONCURRENT_INSTALLS)
}

/// Result of resolution: either a single-environment result or a universal
/// (multi-environment) merged result.
pub enum ResolveOut {
    Single(ResolutionResult),
    Universal {
        entries: Vec<UniversalPackageEntry>,
        environments: Vec<String>,
    },
}

/// Run dependency resolution from a `VypConfig`, optionally including
/// extra requirements (e.g. from `vyp add`).
///
/// When `config.environments` is empty, returns `ResolveOut::Single`.
/// When non-empty, resolves once per environment and returns `ResolveOut::Universal`
/// with merged entries and the environment marker list.
pub fn resolve_from_config(
    config: &VypConfig,
    extra_reqs: &[Requirement],
    torch_backend: Option<&str>,
    progress: Option<Box<dyn Fn(ResolveProgress) + Send>>,
) -> miette::Result<(ResolveOut, reqwest::Client)> {
    if config.environments.is_empty() {
        let (result, client) = resolve_single(config, extra_reqs, torch_backend, progress)?;
        return Ok((ResolveOut::Single(result), client));
    }

    let shared_client = vyp_index::PyPIMetadataProvider::build_default_client();
    let mut env_results: Vec<(String, ResolutionResult)> = Vec::new();

    for marker_str in &config.environments {
        let marker_env = parse_python_version_from_marker(marker_str)
            .and_then(|v| MarkerEnvironment::for_python_version_str(&v))
            .ok_or_else(|| {
                miette::miette!(
                    "Unsupported environment marker '{}'; only python_version == \"X.Y\" is supported",
                    marker_str
                )
            })?;

        let mut builder = ResolverBuilder::new()
            .with_overrides(config.overrides.clone())
            .with_substitutions(config.substitutions.clone())
            .with_resolution_strategy(config.core_resolution_strategy())
            .with_pre_release_policy(config.core_pre_release_policy());

        // Progress callback is not shared across multi-env resolutions (single use per resolve).
        let _ = progress;

        config.load_plugins(builder.plugin_loader_mut());

        for req_str in &config.dependencies {
            let req: Requirement = req_str
                .parse()
                .map_err(|e| miette::miette!("Invalid requirement '{}': {}", req_str, e))?;
            builder = builder.add_dependency(req);
        }

        for req in extra_reqs {
            builder = builder.add_dependency(req.clone());
        }

        let (providers, router) = config.create_providers_with_client(
            torch_backend,
            Some(&marker_env),
            Some(shared_client.clone()),
        )?;
        for provider in providers {
            builder = builder.with_provider(provider);
        }
        builder = builder.with_index_router(router);

        let result = builder.resolve().map_err(|e| match e {
            vyp_core::VypError::NoSolution(msg) => {
                miette::miette!("No solution for {}:\n{}", marker_str, msg)
            }
            other => miette::miette!("Resolution failed: {}", other),
        })?;
        env_results.push((marker_str.clone(), result));
    }

    let entries = merge_universal_results(env_results, config.fork_strategy);
    let environments = config.environments.clone();
    Ok((
        ResolveOut::Universal { entries, environments },
        shared_client,
    ))
}

fn resolve_single(
    config: &VypConfig,
    extra_reqs: &[Requirement],
    torch_backend: Option<&str>,
    progress: Option<Box<dyn Fn(ResolveProgress) + Send>>,
) -> miette::Result<(ResolutionResult, reqwest::Client)> {
    let marker_env = MarkerEnvironment::detect();
    let shared_client = vyp_index::PyPIMetadataProvider::build_default_client();

    let mut builder = ResolverBuilder::new()
        .with_overrides(config.overrides.clone())
        .with_substitutions(config.substitutions.clone())
        .with_resolution_strategy(config.core_resolution_strategy())
        .with_pre_release_policy(config.core_pre_release_policy());

    if let Some(cb) = progress {
        builder = builder.with_progress(cb);
    }

    config.load_plugins(builder.plugin_loader_mut());

    for req_str in &config.dependencies {
        let req: Requirement = req_str
            .parse()
            .map_err(|e| miette::miette!("Invalid requirement '{}': {}", req_str, e))?;
        builder = builder.add_dependency(req);
    }

    for req in extra_reqs {
        builder = builder.add_dependency(req.clone());
    }

    let (providers, router) = config.create_providers_with_client(
        torch_backend,
        Some(&marker_env),
        Some(shared_client.clone()),
    )?;
    for provider in providers {
        builder = builder.with_provider(provider);
    }
    builder = builder.with_index_router(router);

    let result = builder.resolve().map_err(|e| match e {
        vyp_core::VypError::NoSolution(msg) => miette::miette!("No solution found:\n{}", msg),
        other => miette::miette!("Resolution failed: {}", other),
    })?;

    Ok((result, shared_client))
}

/// Timing breakdown of the install phase (populated when VYP_PROFILE=1).
/// All _ms fields are wall-clock except download/extract/link which are summed across workers.
pub struct InstallTiming {
    pub marker_detect_ms: f64,
    pub site_packages_ms: f64,
    pub cache_check_ms: f64,
    pub runtime_client_ms: f64,
    pub pipeline_wall_ms: f64,
    pub cached_link_wall_ms: f64,
    pub download_ms: f64,
    pub extract_ms: f64,
    pub link_ms: f64,
    pub eviction_ms: f64,
    pub total_ms: f64,
    pub cached_count: usize,
    pub download_count: usize,
    pub link_count: usize,
}

struct DownloadedWheel {
    #[allow(dead_code)]
    pkg_name: String,
    #[allow(dead_code)]
    pkg_version: String,
    path: PathBuf,
    sha256: String,
}

/// Install packages from a lock file into a virtual environment.
///
/// Architecture: streaming pipeline — all phases overlap:
///   1. Check wheel cache for pre-extracted archives
///   2. Link cached packages AND download missing wheels **concurrently**
///   3. As EACH download completes, immediately extract+link (no waiting
///      for all downloads to finish first)
///
/// Wall-clock time ≈ max(download) + extract_tail + link_tail, not sum(all).
/// Filter lockfile packages to those applicable for the current marker environment.
fn packages_for_env<'a>(
    packages: &'a [PyLockPackage],
    marker_env: &MarkerEnvironment,
) -> Vec<&'a PyLockPackage> {
    packages
        .iter()
        .filter(|pkg| {
            match &pkg.marker {
                None => true,
                Some(m) => MarkerTree::parse(m).evaluate(marker_env, &[]),
            }
        })
        .collect()
}

pub fn install_lockfile(
    lockfile: &LockFile,
    venv: Option<&Path>,
    dry_run: bool,
    default_index: Option<&str>,
    shared_client: Option<reqwest::Client>,
) -> miette::Result<Option<InstallTiming>> {
    let profiling = std::env::var("VYP_PROFILE").is_ok_and(|v| v == "1");
    let t_marker_start = Instant::now();
    let marker_env = MarkerEnvironment::detect();
    let marker_detect_ms = if profiling { t_marker_start.elapsed().as_secs_f64() * 1000.0 } else { 0.0 };
    let packages = packages_for_env(&lockfile.packages, &marker_env);

    if packages.is_empty() {
        println!("No packages to install for this environment.");
        return Ok(None);
    }

    let t_site_start = Instant::now();
    let site_packages = resolve_site_packages(venv)?;
    let site_packages_ms = if profiling { t_site_start.elapsed().as_secs_f64() * 1000.0 } else { 0.0 };
    let fallback_index = default_index.unwrap_or(DEFAULT_INDEX);

    if dry_run {
        println!(
            "Would install {} packages into {}:",
            packages.len(),
            site_packages.display()
        );
        for pkg in &packages {
            println!("  {} == {}", pkg.name, pkg.version);
        }
        return Ok(None);
    }

    let total = packages.len();
    println!("Installing {} packages into {}...", total, site_packages.display());

    let start = Instant::now();
    let wheel_cache = WheelCache::new();
    let platform_tags = PlatformTags::from_env(&marker_env);

    // Phase 1: Check cache, partition into cached vs needs-download.
    let phase1_start = Instant::now();
    let mut cached_archives: Vec<(&PyLockPackage, PathBuf, Option<Vec<PathBuf>>)> = Vec::new();
    let mut to_download: Vec<&PyLockPackage> = Vec::new();

    for pkg in &packages {
        let key = pkg_cache_key(pkg);
        if let Some((archive_path, file_list)) = wheel_cache.get_archive_with_file_list(&key) {
            cached_archives.push((pkg, archive_path, file_list));
            continue;
        }
        to_download.push(pkg);
    }
    let cache_check_ms = phase1_start.elapsed().as_secs_f64() * 1000.0;
    let cached_count = cached_archives.len();
    let download_count = to_download.len();

    // Skip per-file exists/remove when site_packages is empty (fresh install).
    let assume_fresh = !site_packages.exists()
        || std::fs::read_dir(&site_packages)
            .map(|d| d.count() == 0)
            .unwrap_or(false);
    let assume_fresh_dl = assume_fresh && cached_count == 0;

    let dl_ns = Arc::new(AtomicUsize::new(0));
    let ext_ns = Arc::new(AtomicUsize::new(0));
    let lnk_ns = Arc::new(AtomicUsize::new(0));
    let cached_link_wall_ns = Arc::new(AtomicU64::new(0));

    let t_runtime_start = Instant::now();
    let num_workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_workers)
        .enable_all()
        .build()
        .map_err(|e| miette::miette!("Failed to start async runtime: {}", e))?;

    let concurrent_dl = concurrent_downloads();
    let install_workers = concurrent_installs();
    let client = shared_client.unwrap_or_else(|| {
        reqwest::Client::builder()
            .user_agent(format!("vyp/{}", env!("CARGO_PKG_VERSION")))
            .pool_max_idle_per_host(concurrent_dl)
            .tcp_nodelay(true)
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("failed to create HTTP client")
    });
    let runtime_client_ms = if profiling { t_runtime_start.elapsed().as_secs_f64() * 1000.0 } else { 0.0 };

    let tmp_dir = wheel_cache.tmp_dir();
    let _ = std::fs::create_dir_all(&tmp_dir);

    // Collect owned data for the cache link task (needs 'static for spawn_blocking).
    let cached_owned: Vec<(String, PathBuf, Option<Vec<PathBuf>>)> = cached_archives
        .iter()
        .map(|(pkg, path, list)| (pkg.name.clone(), path.clone(), list.clone()))
        .collect();

    // Collect owned download data (needs 'static for async closures).
    struct DownloadJob {
        name: String,
        version: String,
        wheels: Vec<crate::lock::lockfile::PyLockWheel>,
    }
    let download_jobs: Vec<DownloadJob> = to_download
        .iter()
        .map(|pkg| DownloadJob {
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            wheels: pkg.wheels.clone(),
        })
        .collect();

    let tags_owned = platform_tags.clone();

    // Run cached link + streaming download→extract→link concurrently.
    let t_pipeline_start = Instant::now();
    let (link_cached_results, download_pipeline_results) = rt.block_on(async {
        let sp = site_packages.clone();
        let lnk_c = Arc::clone(&lnk_ns);
        let cached_link_wall = Arc::clone(&cached_link_wall_ns);
        let assume_fresh = assume_fresh;
        let install_workers = install_workers;

        let cache_link_handle = tokio::task::spawn_blocking(move || {
            let t = Instant::now();
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(install_workers)
                .build()
                .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().num_threads(1).build().expect("fallback pool"));
            let results: Vec<_> = pool.install(|| {
                cached_owned
                    .par_iter()
                    .map(|(name, archive_path, file_list)| {
                        let t = Instant::now();
                        let list_ref = file_list.as_deref();
                        let res = linker::install_from_archive(archive_path, &sp, list_ref, assume_fresh)
                            .map_err(|e| (name.clone(), format!("install failed: {}", e)));
                        lnk_c.fetch_add(t.elapsed().as_nanos() as usize, AtomicOrdering::Relaxed);
                        res
                    })
                    .collect()
            });
            cached_link_wall.store(t.elapsed().as_nanos() as u64, AtomicOrdering::Relaxed);
            results
        });

        let pipeline_results: Vec<Result<String, (String, String)>> = if download_jobs.is_empty() {
            Vec::new()
        } else {
            let assume_fresh_dl = assume_fresh_dl;
            stream::iter(download_jobs.into_iter())
                .map(|job| {
                    let cl = client.clone();
                    let fb = fallback_index.to_string();
                    let tmp = tmp_dir.clone();
                    let sp2 = site_packages.clone();
                    let tags = tags_owned.clone();
                    let dl_c = Arc::clone(&dl_ns);
                    let ext_c = Arc::clone(&ext_ns);
                    let lnk_c = Arc::clone(&lnk_ns);
                    let assume_fresh_dl = assume_fresh_dl;
                    async move {
                        let dl_start = Instant::now();
                        let dw = match download_wheel_job(&cl, &job.name, &job.version, &job.wheels, &fb, &tmp, &tags).await {
                            Ok(dw) => dw,
                            Err(e) => return Err((job.name.clone(), e.to_string())),
                        };
                        let dl_elapsed = dl_start.elapsed().as_nanos() as usize;

                        let expected_hash = job.wheels.first()
                            .and_then(|w| w.hashes.as_ref())
                            .and_then(|h| h.get("sha256"))
                            .cloned();
                        let cache_key = format!("{}-{}", job.name.to_lowercase().replace('-', "_"), job.version);
                        let pkg_name = job.name;

                        tokio::task::spawn_blocking(move || {
                            dl_c.fetch_add(dl_elapsed, AtomicOrdering::Relaxed);

                            if let Some(expected) = &expected_hash {
                                if dw.sha256 != *expected {
                                    return Err((
                                        pkg_name,
                                        format!("hash mismatch: expected {}, got {}", expected, dw.sha256),
                                    ));
                                }
                            }

                            let wc = WheelCache::new();
                            let ext_start = Instant::now();
                            let (archive_path, file_list) = match wc.store_from_file_with_list(&cache_key, &dw.path, false) {
                                Ok(r) => r,
                                Err(e) => return Err((pkg_name, format!("cache store failed: {}", e))),
                            };
                            let _ = std::fs::remove_file(&dw.path);
                            ext_c.fetch_add(ext_start.elapsed().as_nanos() as usize, AtomicOrdering::Relaxed);

                            let link_start = Instant::now();
                            let file_list_ref = if file_list.is_empty() { None } else { Some(file_list.as_slice()) };
                            if let Err(e) = linker::install_from_archive(&archive_path, &sp2, file_list_ref, assume_fresh_dl) {
                                return Err((pkg_name, format!("install failed: {}", e)));
                            }
                            lnk_c.fetch_add(link_start.elapsed().as_nanos() as usize, AtomicOrdering::Relaxed);

                            Ok(pkg_name)
                        })
                        .await
                        .unwrap_or_else(|e| Err(("unknown".into(), format!("task panicked: {}", e))))
                    }
                })
                .buffer_unordered(concurrent_dl)
                .collect::<Vec<_>>()
                .await
        };

        let cache_results = cache_link_handle.await
            .unwrap_or_else(|e| vec![Err(("cache-link".into(), format!("task panicked: {}", e)))]);

        (cache_results, pipeline_results)
    });
    let pipeline_wall_ms = t_pipeline_start.elapsed().as_secs_f64() * 1000.0;

    let mut installed = 0usize;
    let mut failed: Vec<(String, String)> = Vec::new();

    for r in link_cached_results {
        match r {
            Ok(()) => installed += 1,
            Err(e) => failed.push(e),
        }
    }
    for r in download_pipeline_results {
        match r {
            Ok(_) => installed += 1,
            Err(e) => failed.push(e),
        }
    }

    let t_evict_start = Instant::now();
    if download_count > 0 {
        wheel_cache.evict_archives_if_needed();
    }
    let eviction_ms = if profiling { t_evict_start.elapsed().as_secs_f64() * 1000.0 } else { 0.0 };

    let elapsed = start.elapsed();
    let total_ms = elapsed.as_secs_f64() * 1000.0;
    println!(
        "\nInstalled {} package(s) in {:.2}s.",
        installed,
        elapsed.as_secs_f64()
    );
    if !failed.is_empty() {
        println!("{} package(s) failed:", failed.len());
        for (name, err) in &failed {
            println!("  {}: {}", name, err);
        }
        return Err(miette::miette!(
            "{} package(s) failed to install",
            failed.len()
        ));
    }

    let timing = if profiling {
        let download_ms = dl_ns.load(AtomicOrdering::Relaxed) as f64 / 1_000_000.0;
        let extract_ms = ext_ns.load(AtomicOrdering::Relaxed) as f64 / 1_000_000.0;
        let link_ms = lnk_ns.load(AtomicOrdering::Relaxed) as f64 / 1_000_000.0;
        let cached_link_wall_ms = cached_link_wall_ns.load(AtomicOrdering::Relaxed) as f64 / 1_000_000.0;
        Some(InstallTiming {
            marker_detect_ms,
            site_packages_ms,
            cache_check_ms,
            runtime_client_ms,
            pipeline_wall_ms,
            cached_link_wall_ms,
            download_ms,
            extract_ms,
            link_ms,
            eviction_ms,
            total_ms,
            cached_count,
            download_count,
            link_count: installed,
        })
    } else {
        None
    };

    Ok(timing)
}

/// Stable cache key for a package: normalized name + version.
fn pkg_cache_key(pkg: &PyLockPackage) -> String {
    format!("{}-{}", pkg.name.to_lowercase().replace('-', "_"), pkg.version)
}

/// Download a wheel given owned job data (for the streaming pipeline).
async fn download_wheel_job(
    client: &reqwest::Client,
    name: &str,
    version: &str,
    wheels: &[crate::lock::lockfile::PyLockWheel],
    fallback_index: &str,
    tmp_dir: &Path,
    tags: &PlatformTags,
) -> Result<DownloadedWheel, Box<dyn std::error::Error + Send + Sync>> {
    let url = {
        let lockfile_url = wheels.first().and_then(|w| w.url.as_ref()).cloned();
        let compatible = lockfile_url.as_ref().is_some_and(|u| {
            let fname = u.rsplit('/').next().unwrap_or(u);
            let fname = fname.split('#').next().unwrap_or(fname);
            !fname.ends_with(".whl") || tags.is_compatible(fname)
        });
        if compatible {
            lockfile_url.unwrap()
        } else {
            let normalized = name.to_lowercase().replace('-', "_");
            resolve_wheel_url_direct(client, &normalized, version, fallback_index, tags).await?
        }
    };

    let response = client.get(&url).send().await?.error_for_status()?;

    let tmp_path = tmp_dir.join(format!(
        "{}-{}.whl.tmp",
        name.to_lowercase().replace('-', "_"),
        version
    ));
    let mut file = tokio::fs::File::create(&tmp_path).await?;
    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();

    use futures::TryStreamExt;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.try_next().await? {
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;

    let sha256 = format!("{:x}", hasher.finalize());

    Ok(DownloadedWheel {
        pkg_name: name.to_string(),
        pkg_version: version.to_string(),
        path: tmp_path,
        sha256,
    })
}

/// Resolve wheel URL using normalized name and version directly.
async fn resolve_wheel_url_direct(
    client: &reqwest::Client,
    normalized_name: &str,
    version: &str,
    index_url: &str,
    tags: &PlatformTags,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!(
        "{}/{}/",
        index_url.trim_end_matches('/'),
        normalized_name.replace('_', "-")
    );

    let response = client
        .get(&url)
        .header("Accept", "application/vnd.pypi.simple.v1+json")
        .send()
        .await?
        .error_for_status()?;

    let body = response.text().await?;

    if let Ok(index) = serde_json::from_str::<serde_json::Value>(&body) {
        find_wheel_url_json(&index, normalized_name, version, tags)
    } else {
        find_wheel_url_html(&body, version, tags)
    }
}

pub fn resolve_site_packages(venv: Option<&Path>) -> miette::Result<PathBuf> {
    if let Some(venv_path) = venv {
        return find_site_packages_in_venv(venv_path);
    }

    if let Ok(virtual_env) = std::env::var("VIRTUAL_ENV") {
        let venv_path = PathBuf::from(&virtual_env);
        return find_site_packages_in_venv(&venv_path);
    }

    let local_venv = PathBuf::from(".venv");
    if local_venv.exists() {
        return find_site_packages_in_venv(&local_venv);
    }

    Err(miette::miette!(
        "No virtual environment found. Use --venv to specify one, \
         activate one, or create .venv in the current directory."
    ))
}

fn find_site_packages_in_venv(venv: &Path) -> miette::Result<PathBuf> {
    let lib_dir = venv.join("lib");
    if !lib_dir.exists() {
        return Err(miette::miette!(
            "Invalid virtual environment: {} has no lib/ directory",
            venv.display()
        ));
    }

    let entries: Vec<_> = std::fs::read_dir(&lib_dir)
        .map_err(|e| miette::miette!("Cannot read {}: {}", lib_dir.display(), e))?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("python"))
        .collect();

    let python_dir = entries.first().ok_or_else(|| {
        miette::miette!("No python directory found in {}", lib_dir.display())
    })?;

    let site = python_dir.path().join("site-packages");
    if !site.exists() {
        return Err(miette::miette!(
            "site-packages not found at {}",
            site.display()
        ));
    }

    Ok(site)
}

fn find_wheel_url_json(
    index: &serde_json::Value,
    name: &str,
    version: &str,
    tags: &PlatformTags,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let files = index
        .get("files")
        .and_then(|f| f.as_array())
        .ok_or("No files in index response")?;

    let version_tag = format!("-{}-", version);

    let mut best: Option<(u32, &str)> = None;
    for f in files {
        let filename = match f.get("filename").and_then(|n| n.as_str()) {
            Some(n) => n,
            None => continue,
        };
        if !filename.contains(&version_tag) || !filename.ends_with(".whl") {
            continue;
        }
        if !tags.is_compatible(filename) {
            continue;
        }
        let url = match f.get("url").and_then(|u| u.as_str()) {
            Some(u) => u,
            None => continue,
        };
        let score = tags.compatibility_score(filename);
        if best.as_ref().is_none_or(|(bs, _)| score > *bs) {
            best = Some((score, url));
        }
    }

    best.map(|(_, url)| url.to_string())
        .ok_or_else(|| format!("No compatible wheel found for {}=={}", name, version).into())
}

fn find_wheel_url_html(
    body: &str,
    version: &str,
    tags: &PlatformTags,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let version_tag = format!("-{}-", version);
    let mut best: Option<(u32, String)> = None;

    for line in body.lines() {
        if let Some(href_start) = line.find("href=\"") {
            let rest = &line[href_start + 6..];
            if let Some(href_end) = rest.find('"') {
                let href = &rest[..href_end];
                let filename = href.rsplit('/').next().unwrap_or(href);
                let filename = filename.split('#').next().unwrap_or(filename);
                if filename.contains(&version_tag) && filename.ends_with(".whl") && tags.is_compatible(filename) {
                    let score = tags.compatibility_score(filename);
                    if best.as_ref().is_none_or(|(bs, _)| score > *bs) {
                        best = Some((score, href.to_string()));
                    }
                }
            }
        }
    }

    best.map(|(_, url)| url)
        .ok_or_else(|| format!("No compatible wheel found for version {} in HTML index", version).into())
}

#[allow(dead_code)]
fn verify_wheel_integrity(
    bytes: &[u8],
    pkg: &PyLockPackage,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(wheel) = pkg.wheels.first() {
        if let Some(ref hashes) = wheel.hashes {
            if let Some(expected_sha256) = hashes.get("sha256") {
                let mut hasher = Sha256::new();
                hasher.update(bytes);
                let actual = format!("{:x}", hasher.finalize());
                if actual != *expected_sha256 {
                    return Err(format!(
                        "Hash mismatch for {}: expected sha256={}, got {}",
                        pkg.name, expected_sha256, actual
                    ).into());
                }
            }
        }
    }

    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    let metadata_path = (0..archive.len())
        .find_map(|i| {
            let file = archive.by_index(i).ok()?;
            let name = file.name().to_string();
            if name.ends_with(".dist-info/METADATA") {
                Some(name)
            } else {
                None
            }
        });

    if let Some(path) = metadata_path {
        let mut file = archive.by_name(&path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let mut meta_name = None;
        let mut meta_version = None;
        for line in content.lines() {
            if let Some(val) = line.strip_prefix("Name: ") {
                meta_name = Some(val.trim().to_string());
            } else if let Some(val) = line.strip_prefix("Version: ") {
                meta_version = Some(val.trim().to_string());
            }
            if meta_name.is_some() && meta_version.is_some() {
                break;
            }
            if line.is_empty() {
                break;
            }
        }

        if let Some(ref name) = meta_name {
            let normalized_meta = name.to_lowercase().replace(['-', '.'], "_");
            let normalized_expected = pkg.name.to_lowercase().replace(['-', '.'], "_");
            if normalized_meta != normalized_expected {
                return Err(format!(
                    "Package name mismatch: lock expects '{}' but wheel contains '{}'",
                    pkg.name, name
                ).into());
            }
        }

        if let Some(ref version) = meta_version {
            if *version != pkg.version {
                return Err(format!(
                    "Version mismatch for {}: lock expects '{}' but wheel contains '{}'",
                    pkg.name, pkg.version, version
                ).into());
            }
        }
    }

    if let Some(ref expected_variant) = pkg.variant {
        let variant_path = (0..archive.len())
            .find_map(|i| {
                let file = archive.by_index(i).ok()?;
                let name = file.name().to_string();
                if name.ends_with(".dist-info/variant.json") {
                    Some(name)
                } else {
                    None
                }
            });

        if let Some(path) = variant_path {
            let mut file = archive.by_name(&path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;

            let actual_variant: vyp_api::VariantDescriptor = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse variant.json in wheel: {}", e))?;

            if actual_variant != *expected_variant {
                return Err(format!(
                    "Variant mismatch for {}: lock variant descriptor does not match wheel's variant.json",
                    pkg.name
                ).into());
            }
        } else {
            return Err(format!(
                "Lock expects variant info for {} but wheel has no variant.json",
                pkg.name
            ).into());
        }
    }

    Ok(())
}
