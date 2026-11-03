//! `--cache DIR`: what a script's work comes to before other scripts' call
//! sites name it, kept between runs under a key of this build, the options
//! and the script, so a run over scripts seen before decompiles only those
//! that changed. An entry that cannot be read is no entry.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

pub struct Cache {
    dir: PathBuf,
    /// This build and the options, hashed two ways.
    base: [u64; 2],
    /// Most bytes its entries may take: after a run the least recently
    /// used go until they fit (`--cache-max-mib`, 512 unless said).
    max_bytes: u64,
}

pub struct Entry {
    /// Compiled with `--!native`.
    pub native: bool,
    /// The text before naming.
    pub core: String,
    /// Its call sites, as passes::tally_bytes writes them.
    pub calls: Vec<u8>,
    /// Named alone: a script that defines nothing another may call is done.
    pub named: Option<String>,
}

/// SipHash with fixed keys (the same every run), each part after its length.
fn hash(seed: u64, parts: &[&[u8]]) -> u64 {
    let mut h = DefaultHasher::new();
    h.write_u64(seed);
    for p in parts {
        h.write_usize(p.len());
        h.write(p);
    }
    h.finish()
}

const MAGIC: &str = "luau-decompile-cache-1";

impl Cache {
    /// None when the directory cannot be made or this program not read.
    pub fn open(dir: &Path, options: &str, max_mib: u64) -> Option<Cache> {
        fs::create_dir_all(dir).ok()?;
        let exe = fs::read(std::env::current_exe().ok()?).ok()?;
        let base = [1, 2].map(|seed| hash(seed, &[&exe, options.as_bytes()]));
        Some(Cache { dir: dir.to_path_buf(), base, max_bytes: max_mib.saturating_mul(1 << 20) })
    }

    /// The least recently used entries go until the rest fit. Only files
    /// named as entries are (32 hex digits); nothing else in the folder.
    pub fn prune(&self) {
        let Ok(list) = fs::read_dir(&self.dir) else { return };
        let mut entries: Vec<(std::time::SystemTime, u64, PathBuf)> = list
            .flatten()
            .filter(|e| {
                let n = e.file_name();
                let n = n.to_string_lossy();
                n.len() == 32 && n.bytes().all(|b| b.is_ascii_hexdigit())
            })
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                Some((meta.modified().ok()?, meta.len(), e.path()))
            })
            .collect();
        let mut total: u64 = entries.iter().map(|e| e.1).sum();
        entries.sort_by_key(|e| e.0);
        for (_, size, path) in entries {
            if total <= self.max_bytes {
                break;
            }
            if fs::remove_file(&path).is_ok() {
                total -= size;
            }
        }
    }

    /// The entry name for a script: its name where it is, and its bytes.
    pub fn key(&self, name: &str, data: &[u8]) -> String {
        let [a, b] = self.base.map(|seed| hash(seed, &[name.as_bytes(), data]));
        format!("{a:016x}{b:016x}")
    }

    pub fn get(&self, key: &str) -> Option<Entry> {
        let path = self.dir.join(key);
        let raw = fs::read(&path).ok()?;
        // used now: the last to go when the cache is pruned
        if let Ok(f) = fs::File::options().write(true).open(&path) {
            let _ = f.set_modified(std::time::SystemTime::now());
        }
        let eol = raw.iter().position(|b| *b == b'\n')?;
        let head = std::str::from_utf8(&raw[..eol]).ok()?;
        let [magic, native, core, calls, named] = head.split(' ').collect::<Vec<_>>()[..] else { return None };
        if magic != MAGIC {
            return None;
        }
        let (core, calls): (usize, usize) = (core.parse().ok()?, calls.parse().ok()?);
        let named: Option<usize> = if named == "-" { None } else { Some(named.parse().ok()?) };
        let body = &raw[eol + 1..];
        if body.len() != core + calls + named.unwrap_or(0) {
            return None;
        }
        let text = |b: &[u8]| String::from_utf8(b.to_vec()).ok();
        Some(Entry {
            native: native == "1",
            core: text(&body[..core])?,
            calls: body[core..core + calls].to_vec(),
            named: match named {
                Some(n) => Some(text(&body[core + calls..core + calls + n])?),
                None => None,
            },
        })
    }

    /// Kept whole or not at all: written aside, then put in place.
    pub fn put(&self, key: &str, e: &Entry) {
        let named = e.named.as_ref().map_or("-".to_string(), |n| n.len().to_string());
        let mut out = format!("{MAGIC} {} {} {} {named}\n", u8::from(e.native), e.core.len(), e.calls.len()).into_bytes();
        out.extend_from_slice(e.core.as_bytes());
        out.extend_from_slice(&e.calls);
        if let Some(n) = &e.named {
            out.extend_from_slice(n.as_bytes());
        }
        let aside = self.dir.join(format!("{key}.{}", std::process::id()));
        if fs::write(&aside, &out).is_ok() && fs::rename(&aside, self.dir.join(key)).is_err() {
            let _ = fs::remove_file(&aside);
        }
    }
}
