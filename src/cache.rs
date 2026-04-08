//! On-disk pack cache.
//!
//! A deterministic cache that short-circuits [`crate::packer::pack`] when
//! the inputs that would produce a pack are byte-for-byte identical to a
//! previous run. A cache hit skips every phase of the packer (tree
//! construction, signature extraction via tree-sitter, content reads, and
//! tokenization), which can turn a multi-second pack on a large repo into
//! a sub-10 ms deserialization.
//!
//! # Cache key
//!
//! The cache key is a SHA-256 hash over a JSON-serialized [`CacheKeyInputs`]
//! containing:
//!
//! - `dirpack_version` - the crate version. Bumping the dirpack version
//!   invalidates every entry, which is intentional: the output format or
//!   packing algorithm may have changed.
//! - `canonical_root` - the absolute canonicalized path being packed, so
//!   two different directories with identical contents do not share an
//!   entry.
//! - `budget_target` - the tokens/bytes budget. Different budgets produce
//!   different outputs.
//! - `format` - the output format enum.
//! - `use_git` / `include_signatures` - the two boolean feature flags
//!   that affect which scan path is used and whether tree-sitter runs.
//! - `root_label` - the user-supplied root label override, since it
//!   appears verbatim in the output.
//! - `config_digest` - a SHA-256 of the TOML-serialized [`Config`]. Any
//!   change to config — priorities, content caps, excludes, signature
//!   languages, etc. — produces a new key.
//! - `files_manifest` - a sorted list of `(relative_path, size,
//!   mtime_secs)` for every scanned file. Modifying, adding, or removing
//!   any file invalidates the cache because the manifest hash changes.
//!
//! Invalidation is implicit: there is no TTL, no background cleanup, no
//! rebuild on mtime drift. Every semantic change shows up in one of the
//! hashed inputs.
//!
//! # Storage
//!
//! Entries live under the user cache directory
//! (`${XDG_CACHE_HOME:-~/.cache}/dirpack/packs/`). Each entry is a single
//! JSON file named after its cache key. Entries are self-describing — a
//! new dirpack version will compute different keys and simply stop reading
//! old files, which can then be pruned by the user with `rm -rf` at any
//! time.
//!
//! # Opt-out
//!
//! - `CacheConfig::enabled = false` in dirpack.toml
//! - `--no-cache` CLI flag (threads through `CacheOptions::disabled()`)
//! - `DIRPACK_NO_CACHE=1` env var

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::budget::BudgetTarget;
use crate::config::{Config, OutputFormat};
use crate::packer::{PackResult, TruncationInfo};
use crate::scanner::entry::FileEntry;

/// Environment variable to force-disable the pack cache for one invocation.
pub const NO_CACHE_ENV: &str = "DIRPACK_NO_CACHE";

/// Per-call caching options. Built from `CacheConfig` + flag/env overrides.
#[derive(Debug, Clone, Copy)]
pub struct CacheOptions {
    pub enabled: bool,
}

impl CacheOptions {
    /// Resolve final cache options from the config, a CLI flag, and the
    /// `DIRPACK_NO_CACHE` env var. Any disabling signal wins.
    pub fn resolve(config: &Config, cli_no_cache: bool) -> Self {
        let env_disabled = std::env::var(NO_CACHE_ENV)
            .ok()
            .map(|v| !matches!(v.as_str(), "" | "0" | "false" | "no" | "off"))
            .unwrap_or(false);
        CacheOptions {
            enabled: config.cache.enabled && !cli_no_cache && !env_disabled,
        }
    }

    pub fn disabled() -> Self {
        CacheOptions { enabled: false }
    }
}

/// Serializable manifest of a single scanned file used in the cache key.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestEntry {
    path: String,
    size: u64,
    mtime_secs: i64,
    is_dir: bool,
}

/// All inputs that determine the output of a pack. Hashed to produce the
/// cache key.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheKeyInputs {
    dirpack_version: String,
    canonical_root: String,
    budget_target: String,
    format: String,
    use_git: bool,
    include_signatures: bool,
    root_label: Option<String>,
    config_digest: String,
    files_manifest: Vec<ManifestEntry>,
}

/// Cache key: lowercase hex of the SHA-256 of the inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheKey(String);

impl CacheKey {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// On-disk entry schema. Bumped if the layout changes.
const ENTRY_SCHEMA_VERSION: u32 = 1;

/// Cached pack result, serialized to disk as one JSON file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedEntry {
    pub schema_version: u32,
    pub dirpack_version: String,
    pub key: String,
    pub generated_at_unix: i64,
    pub output: String,
    pub budget_used: usize,
    pub budget_limit: usize,
    pub files_included: usize,
    pub truncation: TruncationSnapshot,
}

/// Serializable mirror of [`TruncationInfo`] so we don't have to touch the
/// public type with serde attributes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TruncationSnapshot {
    pub files_scanned: usize,
    pub files_in_tree: usize,
    pub files_with_signatures: usize,
    pub dirs_truncated: usize,
}

impl From<&TruncationInfo> for TruncationSnapshot {
    fn from(t: &TruncationInfo) -> Self {
        TruncationSnapshot {
            files_scanned: t.files_scanned,
            files_in_tree: t.files_in_tree,
            files_with_signatures: t.files_with_signatures,
            dirs_truncated: t.dirs_truncated,
        }
    }
}

impl From<TruncationSnapshot> for TruncationInfo {
    fn from(s: TruncationSnapshot) -> Self {
        TruncationInfo {
            files_scanned: s.files_scanned,
            files_in_tree: s.files_in_tree,
            files_with_signatures: s.files_with_signatures,
            dirs_truncated: s.dirs_truncated,
        }
    }
}

impl CachedEntry {
    pub fn into_pack_result(self) -> PackResult {
        PackResult {
            output: self.output,
            budget_used: self.budget_used,
            budget_limit: self.budget_limit,
            files_included: self.files_included,
            truncation: self.truncation.into(),
        }
    }

    pub fn from_pack_result(key: &CacheKey, result: &PackResult) -> Self {
        CachedEntry {
            schema_version: ENTRY_SCHEMA_VERSION,
            dirpack_version: env!("CARGO_PKG_VERSION").to_string(),
            key: key.0.clone(),
            generated_at_unix: now_unix_secs(),
            output: result.output.clone(),
            budget_used: result.budget_used,
            budget_limit: result.budget_limit,
            files_included: result.files_included,
            truncation: TruncationSnapshot::from(&result.truncation),
        }
    }
}

fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Build a cache key from every input that can affect the pack output.
pub fn compute_key(
    canonical_root: &Path,
    entries: &[FileEntry],
    config: &Config,
    budget_target: BudgetTarget,
    format: OutputFormat,
    use_git: bool,
    include_signatures: bool,
    root_label: Option<&str>,
) -> CacheKey {
    let mut manifest: Vec<ManifestEntry> = entries
        .iter()
        .map(|entry| ManifestEntry {
            path: entry.relative_path.to_string_lossy().into_owned(),
            size: entry.size,
            mtime_secs: file_mtime_secs(&entry.path),
            is_dir: entry.is_dir,
        })
        .collect();
    // Normalize ordering so scanner permutations don't break the key.
    manifest.sort_by(|a, b| a.path.cmp(&b.path));

    let inputs = CacheKeyInputs {
        dirpack_version: env!("CARGO_PKG_VERSION").to_string(),
        canonical_root: canonical_root.to_string_lossy().into_owned(),
        budget_target: format!("{:?}", budget_target),
        format: format!("{:?}", format),
        use_git,
        include_signatures,
        root_label: root_label.map(|s| s.to_string()),
        config_digest: config_digest(config),
        files_manifest: manifest,
    };

    // Serialize deterministically. serde_json's default object order is
    // insertion order, which is stable for our typed struct.
    let serialized = serde_json::to_vec(&inputs).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&serialized);
    CacheKey(hex(hasher.finalize().as_slice()))
}

fn file_mtime_secs(path: &Path) -> i64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn config_digest(config: &Config) -> String {
    // toml::to_string preserves field order via serde, which gives us a
    // stable digest across runs as long as the struct layout is stable.
    let toml_repr = toml::to_string(config).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(toml_repr.as_bytes());
    hex(hasher.finalize().as_slice())
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

/// Resolve the cache file path for a given key.
fn entry_path(key: &CacheKey) -> Option<PathBuf> {
    let base = cache_base_dir()?;
    Some(base.join(format!("{}.json", key.0)))
}

/// Root directory for cache entries. Honors `XDG_CACHE_HOME`, falls back
/// to `~/.cache/dirpack/packs/`.
pub fn cache_base_dir() -> Option<PathBuf> {
    let xdg = std::env::var("XDG_CACHE_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::cache_dir())?;
    Some(xdg.join("dirpack").join("packs"))
}

/// Try to read a cached entry for this key. Returns `None` for miss or
/// any read/parse error (silently — cache misses must never fail a pack).
pub fn read(key: &CacheKey) -> Option<CachedEntry> {
    let path = entry_path(key)?;
    let bytes = fs::read(&path).ok()?;
    let entry: CachedEntry = serde_json::from_slice(&bytes).ok()?;
    if entry.schema_version != ENTRY_SCHEMA_VERSION {
        return None;
    }
    if entry.key != key.0 {
        return None;
    }
    Some(entry)
}

/// Write a cache entry. Failures are logged to stderr and then swallowed
/// — a failed cache write must never fail the pack.
pub fn write(key: &CacheKey, result: &PackResult) {
    let Some(path) = entry_path(key) else {
        return;
    };
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("dirpack: cache create_dir_all failed: {}", e);
            return;
        }
    }
    let entry = CachedEntry::from_pack_result(key, result);
    let serialized = match serde_json::to_vec(&entry) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("dirpack: cache serialize failed: {}", e);
            return;
        }
    };

    // Write atomically: tmp file + rename so a concurrent read never sees
    // a partial payload.
    let tmp = path.with_extension("json.tmp");
    let mut file = match fs::File::create(&tmp) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("dirpack: cache create failed: {}", e);
            return;
        }
    };
    if let Err(e) = file.write_all(&serialized) {
        eprintln!("dirpack: cache write failed: {}", e);
        let _ = fs::remove_file(&tmp);
        return;
    }
    drop(file);
    if let Err(e) = fs::rename(&tmp, &path) {
        eprintln!("dirpack: cache rename failed: {}", e);
        let _ = fs::remove_file(&tmp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip_length() {
        let bytes = [0x00u8, 0x0f, 0xff];
        assert_eq!(hex(&bytes), "000fff");
    }

    #[test]
    fn cache_options_resolve_respects_flag() {
        let config = Config::default();
        let opts = CacheOptions::resolve(&config, true);
        assert!(!opts.enabled);
    }

    #[test]
    fn cache_options_resolve_respects_config() {
        let mut config = Config::default();
        config.cache.enabled = false;
        let opts = CacheOptions::resolve(&config, false);
        assert!(!opts.enabled);
    }

    #[test]
    fn cache_options_default_enabled() {
        let config = Config::default();
        // Ensure env doesn't pollute test
        std::env::remove_var(NO_CACHE_ENV);
        let opts = CacheOptions::resolve(&config, false);
        assert!(opts.enabled);
    }
}
