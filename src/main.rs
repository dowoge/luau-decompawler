//! luau_decompile: Roblox game-script bytecode (wire v14) to readable Luau.
//!
//! Input is a `.wire` file: the decoded
//! RSB1 payload a script's Source hash names.
//!
//!   luau_decompile FILE.wire              readable Luau to stdout
//!   luau_decompile --disasm FILE.wire     annotated disassembly
//!   --upstream                            input is stock Luau bytecode
//!   --key N                               opcodes stored times N (203: a
//!                                         Roblox client's bytecode)
//!   luau_decompile --dir DIR [--out OUT]  every *.wire in DIR (index.tsv
//!                                         paths name the outputs; with
//!                                         --key also .lua .luau .txt .bin)
//!
//! Bytecode saved as text, a `--` line and then base64, is read as such.
//!   luau_decompile --stream               bytecode as records on stdin, each
//!                                         by itself as if run alone; results
//!                                         go out as records, and last a line
//!                                         saying how many came in
//!
//! A record is a line `<length> <name>` and that many bytes; a result is
//! `<index> ok <length>` and the text, or `<index> error <length>` and why.
//!
//! Output is Luau that compiles. `harness difftest` is the ground-truth test:
//! generated programs must behave the same after a trip through here.
//!
//! LUAU_NOFLAT=1 prints leftover jumps as `goto` instead of restructuring them.
//! LUAU_THREADS=N limits the workers; LUAU_TIMING=1 prints stage times;
//! LUAU_WHY=1 lists each `jump_N` flag and what stopped its helper coming back;
//! LUAU_WHY=all lists every refused helper match too (`REFUSE` lines).

mod alloc;
mod analysis;
mod bytecode;
mod cache;
mod decomp;
mod expr;
mod hash;
mod inline;
mod pack;
mod passes;

use std::fs;
use std::io::{Read, Write};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use bytecode::{Module, Opcodes};
use hash::{HashMap, HashSet};
use pack::Packed;
use passes::{CallMap, Extra, Tally};

#[global_allocator]
static ALLOC: alloc::Alloc = alloc::Alloc;

/// Deeply nested scripts recurse deeply; a worker's stack is reserved, not committed.
const STACK: usize = 1 << 30;

/// Everything up to the point where other scripts' call sites matter.
fn decompile_core(m: &Module, path: Option<&str>, noflat: bool) -> Vec<String> {
    let body = decomp::decompile_lines(m, noflat);
    let mut split = Vec::with_capacity(body.len());
    for line in body {
        if line.contains('\n') {
            split.extend(line.split('\n').map(String::from));
        } else {
            split.push(line);
        }
    }
    if split.is_empty() {
        split.push(String::new());
    }
    let lines = passes::if_expressions(split);
    let lines = passes::uninline(lines);
    let lines = passes::module_table(lines, path);
    passes::tidy_before_names(lines)
}

/// The naming rounds that can use other scripts' call sites.
fn name_round(lines: Vec<String>, extra: &Extra) -> Vec<String> {
    passes::final_forms(passes::blank_lines(passes::more_names(passes::role_names(lines), extra)))
}

/// The finished lines: with `--info` the comments on each function (from m),
/// and `--!native` first when the module was compiled so.
fn finish_lines(mut lines: Vec<String>, m: Option<&Module>, native: bool) -> Vec<String> {
    if RETURN_TYPES.load(std::sync::atomic::Ordering::Relaxed) {
        lines = passes::return_types(lines);
    }
    if let Some(m) = m.filter(|_| decomp::INFO.load(std::sync::atomic::Ordering::Relaxed)) {
        lines = decomp::info_comments(lines, m);
    }
    if let Some(line) = m.filter(|_| KEEP_LINES.load(std::sync::atomic::Ordering::Relaxed)).and_then(one_line) {
        lines = on_one_line(&lines, line, native);
        return lines;
    }
    if native {
        lines.insert(0, "--!native".to_string());
    }
    lines
}

/// A script all on one line (minified) is written on that line, unless
/// `--readable`: some check their own line numbers and run only so.
static KEEP_LINES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// The one line every instruction of the module is on, if there is one: a
/// minified script. (Some check their own line numbers through `debug`.)
fn one_line(m: &Module) -> Option<u32> {
    let mut found: Option<u32> = None;
    for (k, p) in m.protos.iter().enumerate() {
        // the main function's PREPVARARGS is on the chunk's first line, and
        // its closing `return` on the last (after the code's newline)
        let closing = (k == m.main).then(|| p.code.last()).flatten().filter(|i| i.op == bytecode::op::RETURN && i.b() == 1);
        let code = p.code.iter().filter(|i| i.op != bytecode::op::PREPVARARGS && Some(i.pc) != closing.map(|c| c.pc));
        for l in code.filter_map(|i| p.lines.get(i.pc as usize).copied()) {
            match found {
                None => found = Some(l),
                Some(f) if f == l => {}
                Some(_) => return None,
            }
        }
    }
    found.filter(|l| *l >= 1)
}

/// The lines joined on source line `line`: blank lines and whole-line comments
/// go (a comment would hide the rest); `--!native` stays on the first line.
/// Statements that a line break alone kept apart already end in `;`.
fn on_one_line(lines: &[String], line: u32, native: bool) -> Vec<String> {
    let code: Vec<&str> = lines.iter().map(|l| l.trim()).filter(|l| !l.is_empty() && !l.starts_with("--")).collect();
    let mut out: Vec<String> = Vec::new();
    if native {
        out.push("--!native".to_string());
    }
    while (out.len() as u32) + 1 < line {
        out.push(String::new());
    }
    out.push(code.join(" "));
    out
}

fn is_native(m: &Module) -> bool {
    m.protos[m.main].flags & 1 != 0
}

/// `--strict`: a script whose text needs a flag for a jump or the block
/// dispatcher is reported as failed instead of written.
static STRICT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `--return-types`: functions whose results are certainly of one kind are
/// annotated with it. Off by default: sources seldom write them.
static RETURN_TYPES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `--cache-max-mib N`: the most a `--cache` folder may hold after a run.
static CACHE_MAX_MIB: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(512);

/// `--stats-json PATH`: with --dir, what the run came to, as JSON.
static STATS_JSON: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// A JSON string of s.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A finished text as `--strict` takes it: the text, or why it is refused.
fn strict_check(text: String) -> Result<String, String> {
    if !STRICT.load(std::sync::atomic::Ordering::Relaxed) {
        return Ok(text);
    }
    // `local jump_<pc> = `, not `local jump_height`
    let flag = text.match_indices("local jump_").any(|(i, m)| {
        let rest = &text[i + m.len()..];
        let d = rest.bytes().take_while(u8::is_ascii_digit).count();
        d > 0 && rest[d..].starts_with(" = ")
    });
    if flag {
        return Err("strict: a jump is kept as a flag".to_string());
    }
    if text.contains("local block = 0") {
        return Err("strict: blocks dispatched by number".to_string());
    }
    Ok(text)
}

/// `--tabs`: indent with tabs, one for each four leading spaces.
static TABS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn to_text(lines: &[String]) -> String {
    let mut text = lines.join("\n");
    text.push('\n');
    if !TABS.load(std::sync::atomic::Ordering::Relaxed) {
        return text;
    }
    // strings print newlines escaped: leading spaces are indentation
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let n = line.len() - line.trim_start_matches(' ').len();
        out.extend(std::iter::repeat_n('\t', n / 4));
        out.push_str(&line[n / 4 * 4..]);
    }
    out
}

fn panic_text(e: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = e.downcast_ref::<&str>() {
        return (*s).to_string();
    }
    if let Some(s) = e.downcast_ref::<String>() {
        return s.clone();
    }
    "internal error".to_string()
}

/// Runs f over 0..n on every core; results come back in index order.
fn par_map<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let cores = std::thread::available_parallelism().map_or(1, |c| c.get());
    let wanted = std::env::var("LUAU_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(cores);
    let workers = wanted.clamp(1, n.max(1));
    let next = AtomicUsize::new(0);
    let slots: Vec<Mutex<Option<T>>> = (0..n).map(|_| Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..workers {
            std::thread::Builder::new()
                .stack_size(STACK)
                .spawn_scoped(s, || {
                    // the free blocks of a worker that is done, not new ones
                    alloc::adopt();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= n {
                            break;
                        }
                        let v = f(i);
                        *slots[i].lock().unwrap() = Some(v);
                    }
                    alloc::park();
                })
                .expect("cannot start a worker");
        }
    });
    slots.into_iter().map(|s| s.into_inner().unwrap().unwrap()).collect()
}

/// par_map for jobs of very different sizes: the biggest start first, so the
/// longest are not left for last on one worker. A job's memory grows with
/// its size, so jobs run only as many at once as `budget` bytes of them
/// allow (one always may). A worker takes the biggest left that fits, else
/// the smallest, else waits for one to end.
fn par_map_sized<T: Send>(sizes: &[u64], budget: u64, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let n = sizes.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|i| std::cmp::Reverse(sizes[*i]));
    // (next biggest, one past the smallest, bytes running)
    let state = Mutex::new((0usize, n, 0u64));
    let ended = std::sync::Condvar::new();
    let cores = std::thread::available_parallelism().map_or(1, |c| c.get());
    let wanted = std::env::var("LUAU_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(cores);
    let workers = wanted.clamp(1, n.max(1));
    let slots: Vec<Mutex<Option<T>>> = (0..n).map(|_| Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..workers {
            std::thread::Builder::new()
                .stack_size(STACK)
                .spawn_scoped(s, || {
                    alloc::adopt();
                    loop {
                        let i = {
                            let mut st = state.lock().unwrap();
                            loop {
                                let (lo, hi, running) = *st;
                                if lo >= hi {
                                    break None;
                                }
                                let (big, small) = (order[lo], order[hi - 1]);
                                if running == 0 || running + sizes[big] <= budget {
                                    *st = (lo + 1, hi, running + sizes[big]);
                                    break Some(big);
                                }
                                if running + sizes[small] <= budget {
                                    *st = (lo, hi - 1, running + sizes[small]);
                                    break Some(small);
                                }
                                st = ended.wait(st).unwrap();
                            }
                        };
                        let Some(i) = i else { break };
                        let v = f(i);
                        *slots[i].lock().unwrap() = Some(v);
                        state.lock().unwrap().2 -= sizes[i];
                        ended.notify_all();
                    }
                    alloc::park();
                })
                .expect("cannot start a worker");
        }
    });
    slots.into_iter().map(|s| s.into_inner().unwrap().unwrap()).collect()
}

/// What a script's first stage leaves: its text if later stages need it, the
/// functions it defines, its call sites of every function (tally_bytes), and
/// what the summary counts in it once it is written.
#[derive(Default)]
struct Core {
    text: Option<Packed>,
    defined: Vec<String>,
    calls: Vec<u8>,
    done: Option<[usize; 5]>,
    /// compiled with `--!native`
    native: bool,
}

impl Core {
    fn done(counts: [usize; 5]) -> Core {
        Core { done: Some(counts), ..Core::default() }
    }

    /// Out of a job's memory.
    fn copy(&self) -> Core {
        Core {
            text: self.text.as_ref().map(Packed::copy),
            defined: self.defined.iter().map(|k| k.as_str().to_owned()).collect(),
            calls: self.calls.as_slice().to_vec(),
            done: self.done,
            native: self.native,
        }
    }
}

/// Writing a file costs several times what reading it does, and most of what
/// a repeat run produces is already there. The length goes first: the first
/// read of a new file can cost far more than writing it, with a virus scanner
/// looking on, and asking its length does not.
fn write_text(path: &Path, head: &str, text: &str) -> std::io::Result<()> {
    let long = (head.len() + text.len()) as u64;
    if fs::metadata(path).is_ok_and(|m| m.len() == long) {
        if let Ok(old) = fs::read(path) {
            if old.starts_with(head.as_bytes()) && old.ends_with(text.as_bytes()) {
                return Ok(());
            }
        }
    }
    let mut f = fs::File::create(path)?;
    f.write_all(head.as_bytes())?;
    f.write_all(text.as_bytes())
}

fn sanitize(rel: &str, sid: &str) -> String {
    let clean: String = rel
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | ' ' | '-') { c } else { '_' })
        .collect();
    // Instance names come from the server: a `..` must not leave the output folder.
    let safe: Vec<&str> = clean.split('/').map(|p| if p == ".." { "__" } else { p }).collect();
    let joined = safe.join("/");
    let trimmed = joined.trim_matches('/');
    if trimmed.is_empty() {
        sid.to_string()
    } else {
        trimmed.to_string()
    }
}

fn read_index(src: &Path) -> HashMap<String, String> {
    let mut names = HashMap::default();
    if let Ok(data) = fs::read(src.join("index.tsv")) {
        let text = String::from_utf8_lossy(&data);
        for line in text.lines() {
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() >= 6 {
                names.insert(f[0].to_string(), f[5].to_string());
            }
        }
    }
    names
}

/// A bytecode file's module: bytecode itself, or saved as text. What no
/// compiler makes is refused here (bytecode::validate). When the opcodes
/// stored the way asked for read as nothing sound, the other ways are
/// tried (a Roblox client's `* 203`, stock Luau, a game's wire).
fn load(data: &[u8], opcodes: Opcodes) -> Result<Module, String> {
    let text = bytecode::saved_text(data);
    let bytes = text.as_deref().unwrap_or(data);
    let read = |o: Opcodes| -> Result<Module, String> {
        let m = bytecode::parse_as(bytes, o)?;
        bytecode::validate(&m, |ins, maxstack| {
            let (rd, wr) = analysis::full_effects(ins, maxstack);
            !(maxstack..256).any(|r| rd.has(r) || wr.has(r))
        })?;
        Ok(m)
    };
    let first = read(opcodes);
    if first.is_err() && bytes.first() != Some(&0) {
        for other in [Opcodes::Key(203), Opcodes::Stock, Opcodes::Wire] {
            if other != opcodes {
                if let Ok(m) = read(other) {
                    return Ok(m);
                }
            }
        }
    }
    first
}

/// What the compiler writes for a script it could not compile (version 0,
/// then its message): that message, as comments, since there is no code.
fn refused_source(data: &[u8]) -> Option<String> {
    let text = bytecode::saved_text(data);
    let bytes = text.as_deref().unwrap_or(data);
    if bytes.first() != Some(&0) {
        return None;
    }
    let mut out = String::from("-- the compiler refused this script:\n");
    for l in String::from_utf8_lossy(&bytes[1..]).lines() {
        out.push_str(&format!("-- {l}\n"));
    }
    Some(out)
}

fn run_dir(src: &Path, dst: &Path, opcodes: Opcodes, noflat: bool, cache_dir: Option<&Path>) -> i32 {
    let names = read_index(src);
    // dumps of a game's wire bytecode, or with a key other saved scripts
    let keyed = matches!(opcodes, Opcodes::Key(_));
    let input = |f: &str| {
        f.ends_with(".wire") || keyed && [".lua", ".luau", ".txt", ".bin"].iter().any(|x| f.ends_with(x))
    };
    let mut files: Vec<String> = match fs::read_dir(src) {
        Ok(d) => d
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|f| input(f))
            .collect(),
        Err(e) => {
            eprintln!("{}: {e}", src.display());
            return 2;
        }
    };
    files.sort();
    let sids: Vec<&str> = files.iter().map(|f| f.rsplit_once('.').map_or(f.as_str(), |x| x.0)).collect();

    let t0 = std::time::Instant::now();
    let timing = std::env::var_os("LUAU_TIMING").is_some();

    // Paths in order (a repeated game path decides who gets the suffix).
    let mut taken: HashSet<PathBuf> = HashSet::default();
    let mut dirs: HashSet<PathBuf> = HashSet::default();
    let mut targets: Vec<(PathBuf, String)> = Vec::with_capacity(files.len());
    for sid in &sids {
        let rel = sanitize(names.get(*sid).map_or(sid, String::as_str), sid);
        let mut path = dst.join(format!("{rel}.lua"));
        if taken.contains(&path) {
            path = dst.join(format!("{rel} ({sid}).lua")); // two scripts at the same game path
        }
        taken.insert(path.clone());
        if let Some(dir) = path.parent() {
            dirs.insert(dir.to_path_buf());
        }
        targets.push((path, format!("-- {rel} (script {sid})\n")));
    }
    for dir in &dirs {
        let _ = fs::create_dir_all(dir);
    }
    // a script's text written out, and what the summary counts in it
    let write_out = |i: usize, finished: &Result<String, String>| -> [usize; 5] {
        let (path, head) = &targets[i];
        let result = match finished {
            Ok(t) => write_text(path, head, t),
            Err(e) => fs::write(path, format!("{head}-- decompile failed: {e}\n")),
        };
        if let Err(e) = &result {
            eprintln!("{}: {e}", path.display());
        }
        let count = |needle: &str| finished.as_ref().map_or(0, |text| text.matches(needle).count());
        // a flag the dispatcher left: `local jump_<pc> = `, not `local jump_height`
        let jumps = finished.as_ref().map_or(0, |text| {
            text.match_indices("local jump_")
                .filter(|(i, m)| {
                    let rest = &text[i + m.len()..];
                    let d = rest.bytes().take_while(u8::is_ascii_digit).count();
                    d > 0 && rest[d..].starts_with(" = ")
                })
                .count()
        });
        let made = usize::from(finished.is_ok());
        [usize::from(result.is_err()), count(" = false\n"), jumps, count("local block = 0"), made]
    };
    let lines_of = |text: &str| -> Vec<String> { text.split('\n').map(String::from).collect() };
    let alone = Extra::default();
    // the finished text of script i; `--info` reads its bytecode again rather
    // than keep every script's
    let finished = |i: usize, lines: Vec<String>, native: bool| -> Result<String, String> {
        if !decomp::INFO.load(std::sync::atomic::Ordering::Relaxed) {
            return strict_check(to_text(&finish_lines(lines, None, native)));
        }
        let data = fs::read(src.join(&files[i])).map_err(|e| e.to_string())?;
        let m = load(&data, opcodes)?;
        strict_check(to_text(&finish_lines(lines, Some(&m), native)))
    };

    // Script by script, in parallel: the work that needs no other script, up
    // to naming each as if it were alone. Naming sees other scripts only
    // through the call sites of the functions a script defines, so one that
    // defines none is done then, and written. The others keep their text,
    // packed, until the place's call sites are known: a place's text is tens
    // of megabytes.
    let sizes: Vec<u64> = files.iter().map(|f| fs::metadata(src.join(f)).map_or(0, |m| m.len())).collect();
    // a script's bytecode costs up to fifty times as much while it is
    // decompiled, and the biggest come first: a megabyte and a quarter of it
    // at once keeps all cores busy, and more only costs memory
    let budget = std::env::var("LUAU_BUDGET").ok().and_then(|v| v.parse().ok()).unwrap_or(5 << 18);
    // `--cache`: an entry per script, named by its place and its bytes
    let options = format!("{opcodes:?} noflat={noflat} info={}", decomp::INFO.load(std::sync::atomic::Ordering::Relaxed));
    let cache = cache_dir.and_then(|d| cache::Cache::open(d, &options, CACHE_MAX_MIB.load(std::sync::atomic::Ordering::Relaxed)));
    let core: Vec<Core> = par_map_sized(&sizes, budget, |i| {
        let data = match fs::read(src.join(&files[i])) {
            Ok(d) => d,
            Err(e) => return Core::done(write_out(i, &Err(e.to_string()))),
        };
        if let Some(text) = refused_source(&data) {
            return Core::done(write_out(i, &Ok(text)));
        }
        let place = format!("{}{}", targets[i].1, names.get(sids[i]).map_or("", String::as_str));
        let key = cache.as_ref().map(|c| c.key(&place, &data));
        if let Some(hit) = cache.as_ref().zip(key.as_ref()).and_then(|(c, k)| c.get(k)) {
            return alloc::job(
                || match hit.named {
                    Some(named) => Core { calls: hit.calls, ..Core::done(write_out(i, &finished(i, lines_of(&named), hit.native))) },
                    None => Core {
                        defined: passes::defined_keys(&lines_of(&hit.core)),
                        text: Some(Packed::new(&hit.core)),
                        calls: hit.calls,
                        done: None,
                        native: hit.native,
                    },
                },
                |c| c.copy(),
            );
        }
        let run = || -> Result<(String, bool), String> {
            let m = load(&data, opcodes)?;
            Ok((decompile_core(&m, names.get(sids[i]).map(String::as_str), noflat).join("\n"), is_native(&m)))
        };
        alloc::job(
            || {
                let (text, native) = match panic::catch_unwind(AssertUnwindSafe(run)).unwrap_or_else(|e| Err(panic_text(e))) {
                    Ok(done) => done,
                    Err(e) => return Core::done(write_out(i, &Err(e))),
                };
                let lines = lines_of(&text);
                let defined = passes::defined_keys(&lines);
                let kept = (!defined.is_empty()).then(|| Packed::new(&text));
                let store = |calls: &[u8], core: String, named: Option<String>| {
                    if let (Some(c), Some(k)) = (&cache, &key) {
                        c.put(k, &cache::Entry { native, core, calls: calls.to_vec(), named });
                    }
                };
                let text = if cache.is_some() && kept.is_some() { text } else { String::new() };
                // its call sites of every function: which the place defines is
                // not known yet
                let named = panic::catch_unwind(AssertUnwindSafe(|| {
                    let named = name_round(lines, &alone);
                    let mut calls = CallMap::default();
                    passes::collect_calls(&named, &mut calls, None);
                    (named, passes::tally_bytes(&passes::tally(&calls)))
                }));
                match (named, kept) {
                    (Ok((named, calls)), None) => {
                        store(&calls, String::new(), Some(named.join("\n")));
                        Core { calls, ..Core::done(write_out(i, &finished(i, named, native))) }
                    }
                    (Ok((_, calls)), Some(packed)) => {
                        store(&calls, text, None);
                        Core { text: Some(packed), defined, calls, done: None, native }
                    }
                    // named the same way again, it would fail the same way
                    (Err(e), None) => Core::done(write_out(i, &Err(panic_text(e)))),
                    (Err(_), Some(packed)) => Core { text: Some(packed), defined, calls: Vec::new(), done: None, native },
                }
            },
            |c| c.copy(),
        )
    });
    if timing {
        let (text, packed) = core.iter().filter_map(|c| c.text.as_ref()).map(Packed::sizes).fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
        let written = core.iter().filter(|c| c.done.is_some()).count();
        eprintln!(
            "core  {:?} ({:.1} MB of text kept in {:.1}; {written} of {} written)",
            t0.elapsed(),
            text as f64 / 1048576.0,
            packed as f64 / 1048576.0,
            core.len()
        );
    }

    // The drafts' call sites, of the functions some script defines.
    let mut extra = Extra::default();
    extra.want(core.iter().flat_map(|c| c.defined.iter().cloned()));
    let wanted = extra.wanted();
    let mut done: Vec<Option<[usize; 5]>> = Vec::with_capacity(core.len());
    let mut defined: Vec<Vec<String>> = Vec::with_capacity(core.len());
    let mut texts: Vec<Mutex<Option<Packed>>> = Vec::with_capacity(core.len());
    let mut sites: Vec<Tally> = Vec::with_capacity(core.len());
    let mut natives: Vec<bool> = Vec::with_capacity(core.len());
    for c in core {
        sites.push(passes::tally_from(&c.calls, wanted));
        done.push(c.done);
        defined.push(c.defined);
        texts.push(Mutex::new(c.text));
        natives.push(c.native);
    }
    // the text a script kept, read in place
    let text_of = |i: usize| -> Option<String> { texts[i].lock().unwrap().as_ref().map(Packed::text) };
    // the call sites a script's names give: its text itself is named again
    // at the end, which costs less than holding a second copy of every script
    let tally_of = |text: &str, seen: &Extra| {
        let named = name_round(lines_of(text), seen);
        let mut calls = CallMap::default();
        passes::collect_calls(&named, &mut calls, Some(wanted));
        passes::tally(&calls)
    };

    // Call sites across the place: each script sees the calls of those before
    // it, and of those only the functions it defines (a rename cannot change
    // those keys: it leaves words after `.` and `:` alone). What it calls in
    // turn depends only on scripts further back, so there is one answer however
    // it is reached: in rounds, redo what saw something change until nothing
    // does, which costs one pass in file order only if each waits on the last.
    // key -> the first script whose call sites of it changed; the drafts saw nothing
    let mut changed: HashMap<String, usize> = HashMap::default();
    for (j, calls) in sites.iter().enumerate().rev() {
        changed.extend(calls.keys().map(|key| (key.clone(), j)));
    }
    let (mut redone, mut rounds) = (0usize, 0usize);
    loop {
        let todo: Vec<usize> = (0..sites.len())
            .filter(|i| defined[*i].iter().any(|k| changed.get(k).is_some_and(|j| j < i)))
            .collect();
        if todo.is_empty() {
            break;
        }
        redone += todo.len();
        rounds += 1;
        let mut callers: HashMap<&str, Vec<usize>> = HashMap::default();
        for (j, calls) in sites.iter().enumerate() {
            for key in calls.keys() {
                callers.entry(key).or_default().push(j);
            }
        }
        let fresh: Vec<Tally> = par_map(todo.len(), |t| alloc::job(|| {
            let i = todo[t];
            let mut seen = Extra::default();
            for key in &defined[i] {
                if seen.knows(key) {
                    continue; // defined twice
                }
                let before = callers.get(key.as_str()).map_or(&[][..], |list| list);
                for j in before.iter().take_while(|j| **j < i) {
                    seen.add(key, &sites[*j][key]);
                }
            }
            let Some(text) = text_of(i) else { return Tally::default() };
            panic::catch_unwind(AssertUnwindSafe(|| tally_of(&text, &seen))).unwrap_or_default()
        }, |calls| calls.clone()));
        changed.clear();
        for (i, new) in todo.iter().zip(fresh) {
            let old = std::mem::replace(&mut sites[*i], new);
            for key in old.keys().chain(sites[*i].keys()) {
                if old.get(key) != sites[*i].get(key) && !changed.contains_key(key) {
                    changed.insert(key.clone(), *i);
                }
            }
        }
    }
    for calls in &sites {
        for (key, list) in calls {
            extra.add(key, list);
        }
    }
    if timing {
        eprintln!("calls {:?} ({redone} redone over {rounds} rounds)", t0.elapsed());
    }

    // the rest, named with what the place knows; each text goes once written
    let finish = |i: usize| -> Result<String, String> {
        let text = &texts[i].lock().unwrap().take().ok_or_else(|| "named twice".to_string())?.text();
        let seen = if defined[i].iter().any(|k| extra.knows(k)) { &extra } else { &alone };
        let named = panic::catch_unwind(AssertUnwindSafe(|| name_round(lines_of(text), seen))).map_err(panic_text)?;
        panic::catch_unwind(AssertUnwindSafe(|| finished(i, named, natives[i]))).map_err(panic_text)?
    };
    let written: Vec<[usize; 5]> = par_map(files.len(), |i| match done[i] {
        Some(counts) => counts,
        None => alloc::job(|| write_out(i, &finish(i)), |counts| counts),
    });
    if timing {
        eprintln!("names and write {:?}", t0.elapsed());
    }
    let total = |k: usize| written.iter().map(|w| w[k]).sum::<usize>();
    let ok = total(4);
    let failures = written.len() - ok;
    let bad = failures + total(0);
    let (flags, blocks) = (total(1).min(total(2)), total(3));
    println!("{ok} decompiled, {bad} failed, {flags} flags, {blocks} dispatchers -> {}", dst.display());
    if let Some(c) = &cache {
        c.prune();
    }
    if let Some(path) = STATS_JSON.get() {
        let rows: Vec<String> = written
            .iter()
            .zip(&targets)
            .map(|(w, (p, _))| {
                let rel = p.strip_prefix(dst).unwrap_or(p).to_string_lossy().replace('\\', "/");
                format!(
                    "{{\"path\":{},\"decompiled\":{},\"flags\":{},\"dispatchers\":{}}}",
                    json_string(&rel),
                    w[4] == 1 && w[0] == 0,
                    w[1].min(w[2]),
                    w[3]
                )
            })
            .collect();
        let json = format!(
            "{{\"decompiled\":{ok},\"failed\":{bad},\"flags\":{flags},\"dispatchers\":{blocks},\"seconds\":{:.3},\"files\":[{}]}}\n",
            t0.elapsed().as_secs_f64(),
            rows.join(",")
        );
        if let Err(e) = fs::write(path, json) {
            eprintln!("{}: {e}", path.display());
        }
    }
    if bad == 0 {
        0
    } else {
        1
    }
}

/// Records on stdin: `<length> <name>` and that many bytes, until one does not parse.
fn read_records(all: &[u8]) -> Vec<&[u8]> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(eol) = all[at..].iter().position(|b| *b == b'\n').map(|p| at + p) {
        let head = String::from_utf8_lossy(&all[at..eol]);
        let Some(size) = head.split(' ').next().and_then(|n| n.parse::<usize>().ok()) else { break };
        let Some(data) = all.get(eol + 1..eol + 1 + size) else { break };
        found.push(data);
        at = eol + 1 + size;
    }
    found
}

/// Starting a process costs far more than decompiling a small program, so
/// many can share one, and files would only carry them there and back. Each
/// result goes out as soon as it is ready: a crash later keeps what was done.
fn run_stream(opcodes: Opcodes, noflat: bool) -> i32 {
    let mut all = Vec::new();
    if let Err(e) = std::io::stdin().read_to_end(&mut all) {
        eprintln!("stdin: {e}");
        return 2;
    }
    let items = read_records(&all);
    let out = Mutex::new(std::io::stdout());
    let written = par_map(items.len(), |i| {
        let one = || -> Result<String, String> {
            if let Some(text) = refused_source(items[i]) {
                return Ok(text);
            }
            let m = load(items[i], opcodes)?;
            let lines = name_round(decompile_core(&m, None, noflat), &Extra::default());
            strict_check(to_text(&finish_lines(lines, Some(&m), is_native(&m))))
        };
        let result = panic::catch_unwind(AssertUnwindSafe(one)).unwrap_or_else(|e| Err(panic_text(e)));
        let (kind, body) = match &result {
            Ok(text) => ("ok", text),
            Err(why) => ("error", why),
        };
        let mut o = out.lock().unwrap();
        let head = format!("{i} {kind} {}\n", body.len());
        o.write_all(head.as_bytes()).and_then(|_| o.write_all(body.as_bytes())).and_then(|_| o.flush()).is_ok()
    });
    // what tells a caller this binary understood what it was given
    println!("{} files", items.len());
    i32::from(written.contains(&false))
}

fn usage() -> i32 {
    println!(
        "luau_decompile FILE.wire | --disasm FILE.wire | --dir DIR [--out OUT] | --stream\n\
         \x20 --upstream   input is stock Luau bytecode (luau_compile output)\n\
         \x20 --key N      opcodes stored multiplied: the opcode is a byte times N mod 256\n\
         \x20              (203 for a Roblox client's bytecode); with --dir, .lua, .luau,\n\
         \x20              .txt and .bin files are read too\n\
         \x20 --tabs       indent with tabs\n\
         \x20 --readable   a script the bytecode shows all on one line (minified) is\n\
         \x20              written over many lines too; by default it is written on\n\
         \x20              its one line, as some check their own line numbers\n\
         \x20 --info       above each function, a comment with its upvalues, constants,\n\
         \x20              parameters, registers, instructions and flags\n\
         \x20 --strict     a script needing a flag for a jump, or blocks dispatched by\n\
         \x20              number, fails instead of being written\n\
         \x20 --return-types  `: T` on functions whose results are certainly booleans,\n\
         \x20              strings or numbers (`T?` with nil)\n\
         \x20 --cache DIR  with --dir, keep each script's work in DIR: a run over\n\
         \x20              scripts seen before decompiles only those that changed\n\
         \x20 --cache-max-mib N  the most DIR holds after a run, least recently\n\
         \x20              used entries going first (512)\n\
         \x20 --stats-json PATH  with --dir, the run's counts as JSON: scripts\n\
         \x20              decompiled and failed, flags, dispatchers and time, in all and\n\
         \x20              for each file\n\
         Bytecode saved as text, a `--` line and then base64, is read as such."
    );
    2
}

fn run(args: Vec<String>) -> i32 {
    let noflat = std::env::var_os("LUAU_NOFLAT").is_some_and(|v| !v.is_empty());
    let upstream = args.iter().any(|a| a == "--upstream");
    TABS.store(args.iter().any(|a| a == "--tabs"), std::sync::atomic::Ordering::Relaxed);
    KEEP_LINES.store(!args.iter().any(|a| a == "--readable"), std::sync::atomic::Ordering::Relaxed);
    decomp::INFO.store(args.iter().any(|a| a == "--info"), std::sync::atomic::Ordering::Relaxed);
    STRICT.store(args.iter().any(|a| a == "--strict"), std::sync::atomic::Ordering::Relaxed);
    RETURN_TYPES.store(args.iter().any(|a| a == "--return-types"), std::sync::atomic::Ordering::Relaxed);
    let key = match args.iter().position(|a| a == "--key") {
        Some(i) => match args.get(i + 1).and_then(|v| v.parse::<u8>().ok()) {
            Some(k) => Some(k),
            None => return usage(),
        },
        None => None,
    };
    let opcodes = match key {
        Some(k) => Opcodes::Key(k),
        None if upstream => Opcodes::Stock,
        None => Opcodes::Wire,
    };
    let flags = ["--upstream", "--tabs", "--info", "--strict", "--return-types", "--keep-lines", "--readable"];
    let mut args: Vec<String> = args.into_iter().filter(|a| !flags.contains(&a.as_str())).collect();
    if let Some(i) = args.iter().position(|a| a == "--key") {
        args.drain(i..i + 2);
    }
    let cache_dir = match args.iter().position(|a| a == "--cache") {
        Some(i) if i + 1 < args.len() => Some(PathBuf::from(args.drain(i..i + 2).nth(1).unwrap())),
        Some(_) => return usage(),
        None => None,
    };
    if let Some(i) = args.iter().position(|a| a == "--stats-json") {
        match args.get(i + 1) {
            Some(p) => {
                let _ = STATS_JSON.set(PathBuf::from(p));
            }
            None => return usage(),
        }
        args.drain(i..i + 2);
    }
    if let Some(i) = args.iter().position(|a| a == "--cache-max-mib") {
        match args.get(i + 1).and_then(|v| v.parse::<u64>().ok()) {
            Some(n) => CACHE_MAX_MIB.store(n, std::sync::atomic::Ordering::Relaxed),
            None => return usage(),
        }
        args.drain(i..i + 2);
    }
    if args.len() >= 2 && args[0] == "--dir" {
        let src = PathBuf::from(&args[1]);
        let dst = match args.iter().position(|a| a == "--out") {
            Some(i) if i + 1 < args.len() => PathBuf::from(&args[i + 1]),
            _ => src.join("lua"),
        };
        return run_dir(&src, &dst, opcodes, noflat, cache_dir.as_deref());
    }
    if args.first().is_some_and(|a| a == "--stream") {
        return run_stream(opcodes, noflat);
    }
    let disasm = args.iter().any(|a| a == "--disasm");
    let Some(file) = args.iter().find(|a| !a.starts_with("--")) else { return usage() };
    let data = match fs::read(file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{file}: {e}");
            return 2;
        }
    };
    let refused = refused_source(&data);
    let text = match load(&data, opcodes) {
        _ if refused.is_some() => refused.unwrap_or_default(),
        Ok(m) if disasm => decomp::disassemble(&m),
        Ok(m) => {
            let lines = name_round(decompile_core(&m, None, noflat), &Extra::default());
            match strict_check(to_text(&finish_lines(lines, Some(&m), is_native(&m)))) {
                Ok(text) => text,
                Err(why) => {
                    eprintln!("{file}: {why}");
                    return 1;
                }
            }
        }
        Err(e) => {
            eprintln!("{file}: {e}");
            return 1;
        }
    };
    let stdout = std::io::stdout();
    let mut o = stdout.lock();
    if o.write_all(text.as_bytes()).is_err() {
        return 1;
    }
    0
}

fn main() {
    // One bad script is reported as a failed file, not a backtrace per file.
    panic::set_hook(Box::new(|_| {}));
    // what is made once and kept must not be made inside a job, whose memory
    // goes when it ends
    decomp::warm();
    passes::warm();
    let _ = std::io::stdout();
    alloc::poison(std::env::var_os("LUAU_POISON").is_some());
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(move || run(args))
        .expect("cannot start")
        .join()
        .unwrap_or_else(|e| {
            eprintln!("internal error: {}", panic_text(e));
            1
        });
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::sanitize;
    use crate::hash::HashSet;
    use crate::passes::{self, CallMap};

    #[test]
    fn a_required_module_names_its_local_and_a_taken_name_gets_a_number() {
        let lines: Vec<String> = [
            "local lazyRequire = makeRequire(script)",
            "local v1 = lazyRequire(game.ReplicatedStorage.Shared.Settings)",
            "local v2 = require(script.Parent:WaitForChild(\"string-utils\"))",
            "local tween = TweenService:Create(a, info, {})",
            "local v4 = TweenService:Create(b, info, {})",
            "print(v1, v2, tween, v4)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let out = passes::tidy_before_names(lines);
        assert_eq!(out[1], "local Settings = lazyRequire(game.ReplicatedStorage.Shared.Settings)");
        assert_eq!(out[2], "local stringUtils = require(script.Parent:WaitForChild(\"string-utils\"))");
        assert_eq!(out[4], "local tween2 = TweenService:Create(b, info, {})");
        assert_eq!(out[5], "print(Settings, stringUtils, tween, tween2)");
    }

    #[test]
    fn call_sites_kept_as_bytes_read_back_as_those_asked_for() {
        let lines: Vec<String> = [
            "local x = M.f(alpha, beta)",
            "M:g(gamma)",
            "M:g(delta, 1, M.f(beta))",
            "print(M.h(alpha), self.player)",
            "helper(alpha)",
            "function M:g(a, b)",
        ]
        .iter()
        .map(|l| l.to_string())
        .collect();
        let tally_of = |wanted: Option<&HashSet<String>>| {
            let mut calls = CallMap::default();
            passes::collect_calls(&lines, &mut calls, wanted);
            passes::tally(&calls)
        };
        let all = tally_of(None);
        assert!(all.contains_key(".h") && !all.contains_key("helper") && !all.contains_key("print"));
        let bytes = passes::tally_bytes(&all);
        let some: HashSet<String> = [".f", ":g", ".missing"].iter().map(|k| k.to_string()).collect();
        assert!(passes::tally_from(&bytes, &some) == tally_of(Some(&some)));
        let every: HashSet<String> = all.keys().cloned().collect();
        assert!(passes::tally_from(&bytes, &every) == all);
        assert!(passes::tally_from(&[], &every).is_empty());
    }

    #[test]
    fn loop_variables_renamed_leave_what_the_loop_reads_first() {
        // one variable per register: the loops' variables take registers an
        // outer value had, which their headers still read
        let lines: Vec<String> = [
            "local v5 = 3",
            "for v5, v6 in ipairs(if v5 > 1 then t else u) do",
            "    print(v5, v6)",
            "end",
            "for v5 = v5 + 1, 10 do",
            "    print(v5)",
            "end",
            "print(v5)",
        ]
        .iter()
        .map(|l| l.to_string())
        .collect();
        let out = passes::role_names(lines);
        assert!(out[1].starts_with("for i, ") && out[1].ends_with(" in ipairs(if v5 > 1 then t else u) do"), "{}", out[1]);
        assert!(out[2].starts_with("    print(i, "), "{}", out[2]);
        assert!(out[4].ends_with(" = v5 + 1, 10 do") && !out[4].starts_with("for v5 "), "{}", out[4]);
        assert_eq!(out[7], "print(v5)");
    }

    #[test]
    fn output_paths_stay_inside_the_output_folder() {
        assert_eq!(sanitize("../../etc/passwd", "7"), "__/__/etc/passwd");
        assert_eq!(sanitize("a/../b", "7"), "a/__/b");
        assert_eq!(sanitize("/Workspace/Car/", "7"), "Workspace/Car");
        assert_eq!(sanitize("a:b\\c", "7"), "a_b_c");
        assert_eq!(sanitize("///", "7"), "7");
    }
}

#[cfg(test)]
mod keep_lines_tests {
    use super::*;

    #[test]
    fn a_minified_script_goes_back_on_its_line() {
        let lines: Vec<String> =
            ["local t = {}", "", "-- a note", "t.x = 1;", "(print)(t.x)"].iter().map(|s| s.to_string()).collect();
        let out = on_one_line(&lines, 3, false);
        assert_eq!(out, vec!["", "", "local t = {} t.x = 1; (print)(t.x)"]);
        let native = on_one_line(&lines, 2, true);
        assert_eq!(native[0], "--!native");
        assert_eq!(native.len(), 2);
    }
}
