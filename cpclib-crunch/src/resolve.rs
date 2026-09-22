//! Resolve a cruncher by the name a caller types (`"shrinkler"`,
//! `"zx0_backward"`, ...) and run it bounded and cached.
//!
//! Split out from the rest of this crate's `CrunchArgs`/`Cruncher`
//! (file-in, file-out CLI) because this is the piece other tools actually
//! want to reuse directly: [`crate::sizemap`] in this crate, and
//! `cpclib-mcp`'s `compare_crunchers`/`search_reorderings`/`measure_variants`/
//! `size_map` tools, all of which just want "compress these bytes with the
//! format named `X`, bounded, without redoing identical work".

use std::collections::HashMap;
use std::time::Duration;

use cpclib_crunchers::{CompressMethod, CrunchersError};

/// How long a single cruncher gets before a caller gives up on it and
/// reports a timeout for that one format rather than hanging forever.
/// Confirmed live: `pucrunch` (a vendored native tool) hangs indefinitely -
/// "Optimizing LZ77 and RLE lengths..." then nothing - on a degenerate,
/// maximally-repetitive input (256 zero bytes). Since `compress()` is a
/// synchronous, unbounded call with no timeout of its own, and a naive
/// caller might run it directly on an async handler or a single-threaded
/// tool, a hang here can stall far more than just this one call. Run on its
/// own thread with a bounded `recv_timeout` instead, so a native cruncher
/// that hangs only costs one leaked thread (it cannot be force-killed from
/// safe Rust) and a clean per-format timeout error, never the whole caller.
///
/// 90s: calibrated against a real, non-degenerate 16KB CPC screen asset,
/// where `upkr` alone took longer than an earlier, tighter 15s bound while
/// still making genuine progress (unlike `pucrunch`, which never budges at
/// all on its degenerate case) - a false "appears to hang" report for a
/// format that was simply still working.
pub const CRUNCHER_TIMEOUT: Duration = Duration::from_secs(90);

/// Every format name [`resolve_cruncher`] accepts. Matches
/// `cpclib_crunchers::CompressMethod`'s variants one-to-one, except
/// `lzsa1`/`lzsa2` (the single `Lzsa(version, _)` variant, named for its two
/// real on-wire formats the way the asm-facing `CrunchType` directive
/// already does) and `zx0`/`zx0_backward`. No `aplib` - the crate has no
/// separate implementation, only `apultra` (LZAPU).
pub const ALL_FORMATS: &[&str] = &[
    "apultra", "exomizer", "lz4", "lz48", "lz49", "lzsa1", "lzsa2", "pucrunch", "shrinkler", "upkr",
    "zx0", "zx0_backward", "zx7"
];

/// A format name (see [`ALL_FORMATS`]) into the method that implements it,
/// or `Err` naming every valid choice.
pub fn resolve_cruncher(name: &str) -> Result<CompressMethod, String> {
    match name {
        "none" => Ok(CompressMethod::None),
        "apultra" => Ok(CompressMethod::Apultra),
        "exomizer" => Ok(CompressMethod::Exomizer),
        "lz4" => Ok(CompressMethod::Lz4),
        "lz48" => Ok(CompressMethod::Lz48),
        "lz49" => Ok(CompressMethod::Lz49),
        "lzsa1" => {
            let version = cpclib_crunchers::lzsa::LzsaVersion::V1;
            Ok(CompressMethod::Lzsa(version, Some(version.default_minmatch())))
        },
        "lzsa2" => {
            let version = cpclib_crunchers::lzsa::LzsaVersion::V2;
            Ok(CompressMethod::Lzsa(version, Some(version.default_minmatch())))
        },
        // `log: false` trims two of Shrinkler's `printf` lines - unrelated
        // to correctness, just less noise on stderr for a caller that
        // doesn't want it.
        "shrinkler" => {
            Ok(CompressMethod::Shrinkler(cpclib_crunchers::shrinkler::ShrinklerConfiguration {
                iterations: 9,
                log: false
            }))
        },
        "pucrunch" => Ok(CompressMethod::Pucrunch),
        "upkr" => Ok(CompressMethod::Upkr),
        "zx0" => Ok(CompressMethod::Zx0),
        "zx0_backward" => Ok(CompressMethod::BackwardZx0),
        "zx7" => Ok(CompressMethod::Zx7),
        other => {
            Err(format!(
                "unknown cruncher '{other}' - expected one of: none, {}",
                ALL_FORMATS.join(", ")
            ))
        }
    }
}

/// Runs `resolve_cruncher(name).compress(&data)` on a dedicated thread and
/// waits up to `timeout` for it, answered from a small in-process cache when
/// the same `(name, data)` was already compressed - a search that tries many
/// candidates differing in only one region (`size_map`'s leave-one-out,
/// `search_reorderings`, `measure_variants`' proxy metric) keeps asking for
/// the same block, and a real cruncher run is seconds.
///
/// `name`/`data` are owned (not borrowed) so they can move into the spawned
/// thread; a hung cruncher leaks that one thread (unavoidable - safe Rust
/// has no way to force-kill a thread) but this call still returns on time,
/// with a clear per-format error instead of hanging the caller.
pub fn compress_with_timeout(name: String, data: Vec<u8>, timeout: Duration) -> Result<cpclib_crunchers::CompressionResult, String> {
    let key = CacheKey::new(&name, &data);
    if let Some(hit) = cache_lookup(&key) {
        return Ok(hit);
    }
    let result = run_with_timeout(timeout, move || {
        match resolve_cruncher(&name) {
            Ok(method) => {
                method
                    .compress(&data)
                    .map_err(|CrunchersError::CompressionFailed| "compression failed".to_string())
            },
            Err(message) => Err(message)
        }
    });
    if let Ok(done) = &result {
        cache_store(key, done);
    }
    result
}

/// What a crunch was asked to do: cruncher name and content. The content is
/// identified by its length plus two independent 64-bit hashes rather than
/// kept, so the cache stays small.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    name: String,
    len: usize,
    hashes: (u64, u64)
}

impl CacheKey {
    fn new(name: &str, data: &[u8]) -> Self {
        use std::hash::{Hash, Hasher};
        let mut a = std::collections::hash_map::DefaultHasher::new();
        data.hash(&mut a);
        let mut b = std::collections::hash_map::DefaultHasher::new();
        0x9e37_79b9_7f4a_7c15u64.hash(&mut b);
        data.hash(&mut b);
        Self { name: name.to_string(), len: data.len(), hashes: (a.finish(), b.finish()) }
    }
}

const CACHE_LIMIT: usize = 512;

fn cache() -> &'static std::sync::Mutex<HashMap<CacheKey, cpclib_crunchers::CompressionResult>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<HashMap<CacheKey, cpclib_crunchers::CompressionResult>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn cache_lookup(key: &CacheKey) -> Option<cpclib_crunchers::CompressionResult> {
    cache().lock().ok()?.get(key).cloned()
}

fn cache_store(key: CacheKey, result: &cpclib_crunchers::CompressionResult) {
    if let Ok(mut cache) = cache().lock() {
        if cache.len() >= CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, result.clone());
    }
}

/// Runs `work` on its own thread and waits up to `timeout` for its result;
/// on expiry the thread is leaked (unavoidable in safe Rust) and a clear
/// timeout error is returned instead. Generic so the bound itself can be
/// tested without needing a cruncher that really hangs.
fn run_with_timeout<T: Send + 'static>(
    timeout: Duration,
    work: impl FnOnce() -> Result<T, String> + Send + 'static
) -> Result<T, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        // The receiver may already be gone if `recv_timeout` below already
        // gave up - that's fine, nothing left to deliver to.
        let _ = tx.send(work());
    });
    rx.recv_timeout(timeout).unwrap_or_else(|_| {
        Err(format!(
            "timed out after {timeout:?} - this cruncher did not finish in time (a genuine hang, \
             or simply more work than the bound allows for this input)"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_identical_crunch_is_answered_from_the_cache() {
        let data: Vec<u8> = (0..2000u32).map(|i| (i * 7 % 251) as u8).collect();
        let first = compress_with_timeout("zx0".into(), data.clone(), CRUNCHER_TIMEOUT).unwrap();
        assert!(cache_lookup(&CacheKey::new("zx0", &data)).is_some());
        let second = compress_with_timeout("zx0".into(), data.clone(), CRUNCHER_TIMEOUT).unwrap();
        assert_eq!(first.stream, second.stream);
        // another cruncher or other content is a different entry
        assert!(cache_lookup(&CacheKey::new("zx7", &data)).is_none());
        let mut other = data;
        other[0] ^= 1;
        assert!(cache_lookup(&CacheKey::new("zx0", &other)).is_none());
    }

    #[test]
    fn run_with_timeout_gives_up_instead_of_hanging() {
        let start = std::time::Instant::now();
        let result: Result<u32, String> = run_with_timeout(Duration::from_millis(200), || {
            std::thread::sleep(Duration::from_secs(30));
            Ok(1)
        });
        assert!(start.elapsed() < Duration::from_secs(2), "{:?}", start.elapsed());
        assert!(result.unwrap_err().contains("timed out"));
    }

    #[test]
    fn run_with_timeout_returns_the_result_when_in_time() {
        assert_eq!(run_with_timeout(Duration::from_secs(5), || Ok::<_, String>(7)), Ok(7));
    }

    /// Regression for a real bug: a long run of one byte used to hang
    /// pucrunch forever (uninitialised `maxrlelen` in the vendored C - see
    /// `cpclib-crunchers/extra/pucrunch.c`).
    #[test]
    fn pucrunch_finishes_on_a_maximally_repetitive_block() {
        let result = compress_with_timeout("pucrunch".to_string(), vec![0u8; 256], Duration::from_secs(20))
            .expect("pucrunch must finish, not time out");
        assert!(result.stream.len() < 32, "{}", result.stream.len());
    }
}
