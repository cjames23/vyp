# Future Performance Improvements

Potential optimizations that have been investigated but deferred due to complexity or risk.

---

## Overlap Resolve with Download (Streaming Mid-Resolve)

**Estimated gain**: 500–700ms on cold installs (50-package benchmark)

**Idea**: Begin downloading wheel artifacts for packages as soon as the solver selects them, before resolution is fully complete. Currently, all downloads wait until the solver finishes. Overlapping the two phases would hide download latency behind resolve time.

**Implementation sketch**:

1. Add an `mpsc` channel to the solver loop that emits `(name, version, wheel_url)` tuples each time the solver commits a package selection.
2. In `install_lockfile`, spawn a consumer task that listens on this channel and starts downloading wheels immediately.
3. When the solver finishes, drain remaining downloads and proceed with extract+link.

**Risk — Solver Backtracking**:

Backtracking investigation (2026-03-15) found that while common package sets resolve cleanly (1 iteration per package), specific conflict-prone combinations trigger significant backtracking:

| Scenario | Packages | Iterations | Overhead |
|----------|----------|------------|----------|
| Common stacks (flask, django, fastapi, etc.) | 5–10 | N | 0 |
| `old-numpy` + `new-scipy` | 2 | N+3 | 3 |
| `tensorflow-ecosystem` | 4 | N+10 | 10 |
| `boto3-old` + `botocore-new` | 7 | N+173 | 173 |

If the solver downloads a wheel for version X then backtracks to version Y, the X download is wasted bandwidth and disk I/O.

**Mitigation strategies** (to implement alongside):

- **Confidence threshold**: Only start downloads for packages where the solver's decision level is 0 (root-level, unlikely to be undone).
- **Cancellation tokens**: Attach a `CancellationToken` to each speculative download; cancel on backtrack.
- **Deferred high-risk packages**: Maintain a set of known conflict-prone package prefixes (boto3, tensorflow, torch) that skip speculative download.

**Current architecture**: The solver runs on a dedicated thread and blocks on the in-memory index when metadata is not ready; I/O is done by tokio workers. Any mid-resolve streaming would need to coordinate with this thread model (e.g. a channel from solver to an install task) and with backtracking (see above).

**Prerequisites**: All mitigation strategies above should be implemented before enabling mid-resolve streaming in production.

---

## Profiling

To see where resolve time is spent, set `VYP_PROFILE=1` and run resolve. The result includes a `ResolveTiming` breakdown: `version_wait_ms`, `metadata_wait_ms`, `solver_ms`, `wheel_url_ms`, `iterations`, `version_fetches`, `metadata_fetches`, and provider-specific counters (e.g. disk hits, 304s, network fetches). Use this to identify whether waits are dominated by version list fetches, metadata fetches, or pure solver work.
