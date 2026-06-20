# Plan: Install performance (without reverting refactors)

Address identified bottlenecks by aligning with uv’s approach where we differ, while keeping: conflict resolution, universal resolution (optional), and bounded cache with eviction.

---

## 1. What we know from profiling

**Cold path (70 packages, empty cache)**  

- Pipeline wall: ~4.4s (download + extract + link).  
- download (sum) ~42s, extract (sum) ~5s, link (sum) ~3.5s.  
- Eviction once at end: ~100ms.  
- marker_detect, site_packages, cache_check, runtime_client negligible.

**Hot path (70 packages, all from cache)**  

- Pipeline wall: ~~3.4s, almost all in cached_link_wall (~~3.37s).  
- link (sum) ~19s over 70 packages.  
- cache_check ~14ms (70 × get_archive_with_file_list).  
- uv does the same workload in ~1.3s, so we’re ~3× slower on hot.

---

## 2. What uv does (from docs and issues)

- **Install concurrency**: `UV_CONCURRENT_INSTALLS` controls install/unzip parallelism. Default = CPU count; discussion and testing favor **capping at `min(cpu_count, 32)`** to avoid fs contention and kernel overhead on high-core machines (e.g. 128+ cores).  
Ref: Issue #10570, PR #3646, #3493.
- **Download concurrency**: `UV_CONCURRENT_DOWNLOADS` default 50; some tuning suggests 20 can be more stable without losing speed. We use 50 (`MAX_CONCURRENT_DOWNLOADS`).
- **Linking**: Reflink (clonefile) first, then hardlink, then copy. Same order we use. Link mode is configurable (`UV_LINK_MODE`).
- **Cache**: Extract wheels to cache (archive), then link into site-packages. Same idea as our archive + link. They do not run a full cache scan after every package; we fixed that by deferring eviction and skipping when nothing was downloaded.

---

## 3. Root causes and actions

### 3.1 Hot path: cap install parallelism (cached link)

- **Cause**: We use rayon’s default pool for `cached_owned.par_iter()` (cached link), so we use all cores. On many-core or I/O-heavy systems this can cause excessive concurrent fs work and hurt throughput.  
- **Evidence**: uv caps install concurrency (e.g. min(cores, 32)); we do not.  
- **Action**: Run the cached-link phase in a **dedicated rayon pool** with `num_threads = min(available_parallelism(), 32)` (or an env override like `VYP_CONCURRENT_INSTALLS`). Use `rayon::ThreadPoolBuilder` and `pool.install(|| { ... par_iter() ... })` so the existing `par_iter()` only uses that pool.  
- **Files**: `crates/vyp/src/cli/common.rs` (build pool, use it for the `spawn_blocking` that does cached link).  
- **Keep**: File list manifest (no WalkDir), eviction only when download_count > 0, bounded cache.

### 3.2 Hot path: avoid unnecessary `remove_file` on fresh install

- **Cause**: We call `std::fs::remove_file(&target)` before every reflink/hardlink/copy. On a fresh install into an empty site_packages, targets do not exist; we still do one unlink (or similar) per file and ignore errors. That’s a lot of syscalls.  
- **Evidence**: uv installs into a clean tree without needing to remove every file first when the tree is empty.  
- **Action**: Only remove the target when it already exists: e.g. `if target.exists() { std::fs::remove_file(&target)?; }` before reflink/hardlink/copy. Optionally, add a “fresh install” hint (e.g. when we know site_packages was just created or is empty) and skip the existence check for that run to save one stat per file on truly fresh installs; if we don’t add that, the “exists then remove” path still avoids redundant work on reinstall and keeps correctness.  
- **Files**: `crates/vyp/src/cache/linker.rs`.  
- **Keep**: Same linker API and reflink → hardlink → copy order.

**Done**: Implemented “assume_fresh”:
- `linker::install_from_archive(..., assume_fresh: bool)`. When `assume_fresh` is true, we skip the per-file `target.exists()` and `remove_file` entirely (saves one syscall per file on fresh installs).
- Caller computes `assume_fresh = site_packages is empty` once at install start; for cached link we pass it to every package; for download pipeline we pass `assume_fresh && cached_count == 0` so only all-download (cold) runs get it for downloaded packages.
- Hot path: in repeated runs, vyp can be on par or faster (e.g. 0.94× uv) or slower (e.g. 1.66×) depending on system load; assume_fresh removed a major syscall cost. Cold path: still slower than uv (e.g. 1.56×); next levers are pipeline concurrency and extract/link overlap.

### 3.3 Cold path: verify and optionally tune download concurrency

- **Cause**: Cold time is dominated by pipeline (download + extract + link). We use 50 concurrent download jobs; uv uses 50 by default, with some reports that 20 can be more stable.  
- **Action**:  
  - Keep current design (no revert): single pipeline with `buffer_unordered(MAX_CONCURRENT_DOWNLOADS)`, eviction only at end when download_count > 0.  
  - Add an env var (e.g. `VYP_CONCURRENT_DOWNLOADS`, default 50) so we can tune without code change and align with uv-style knobs.  
  - Optionally cap to 20 in the future if we see stability gains; not required for this plan.
- **Files**: `crates/vyp/src/cli/common.rs` (constant or env for concurrency).  
- **Keep**: Bounded cache and eviction behavior.

### 3.4 Eviction: keep current behavior

- **Current**: Eviction runs once after a batch when `download_count > 0`; skipped when everything is from cache. Cache limit 20 GB, overridable via `VYP_CACHE_MAX_ARCHIVE_GB`.  
- **Action**: No change. Ensures cache stays bounded without the previous per-package scan cost.

### 3.5 No changes to conflict resolution or universal resolution

- Conflict resolution (transitive fork, overrides, etc.) and optional universal resolution (environments + fork-strategy) stay as implemented.  
- This plan only touches install path (linker, concurrency, eviction already fixed).

---

## 4. Implementation order

1. **Linker: remove_file only when target exists** (small, low risk).
2. **Cached-link: dedicated rayon pool with cap** (e.g. min(cores, 32) or `VYP_CONCURRENT_INSTALLS`).
3. **Env var for download concurrency** (e.g. `VYP_CONCURRENT_DOWNLOADS`, default 50) for parity and tuning.

---

## 5. How we’ll know it worked

- **Hot**: Re-run hot benchmark (cache full, 70 packages). Target: install wall time closer to uv (~1.3s), without regressing correctness.  
- **Cold**: Re-run cold benchmark; ensure we don’t regress (eviction and pipeline structure unchanged).  
- **Features**: Conflict resolution and universal resolution behavior unchanged; cache remains bounded and eviction only when downloads occurred.

---

## 6. References (uv)

- Issue #10570: stability/performance, concurrent downloads/installs tuning.  
- PR #3646: `UV_CONCURRENT_INSTALLS`.  
- PR #3493: Consolidate concurrency limits.  
- Docs: Caching, Environment variables (`UV_CONCURRENT_DOWNLOADS`, `UV_CONCURRENT_INSTALLS`).  
- PR #1773 / Issue #1444: reflink + copy fallback.

