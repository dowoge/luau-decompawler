//! luau_decompile: Roblox game-script bytecode (wire v14) to readable Luau.
//!
//! Input is a `.wire` file: the decoded
//! RSB1 payload a script's Source hash names.
//!
//!   luau_decompile FILE.wire              readable Luau to stdout
//!   luau_decompile --disasm FILE.wire     annotated disassembly
//!   --upstream                            input is stock Luau bytecode
//!   luau_decompile --dir DIR [--out OUT]  every *.wire in DIR (index.tsv
//!                                         paths name the outputs)
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
//! LUAU_WHY=1 lists each `jump_N` flag and what stopped its helper coming back.

mod alloc;
mod analysis;
mod bytecode;
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

use bytecode::{parse, Module};
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
    passes::blank_lines(passes::more_names(passes::role_names(lines), extra))
}

fn to_text(lines: &[String]) -> String {
    let mut text = lines.join("\n");
    text.push('\n');
    text
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

fn run_dir(src: &Path, dst: &Path, upstream: bool, noflat: bool) -> i32 {
    let names = read_index(src);
    let mut files: Vec<String> = match fs::read_dir(src) {
        Ok(d) => d
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|f| f.ends_with(".wire"))
            .collect(),
        Err(e) => {
            eprintln!("{}: {e}", src.display());
            return 2;
        }
    };
    files.sort();
    let sids: Vec<&str> = files.iter().map(|f| &f[..f.len() - 5]).collect();

    let t0 = std::time::Instant::now();
    let timing = std::env::var_os("LUAU_TIMING").is_some();
    // Script by script, in parallel: the work that needs no other script.
    // Each is kept as one string until it is written: a place's text is
    // tens of megabytes, and as separate lines it costs half as much again.
    let sizes: Vec<u64> = files.iter().map(|f| fs::metadata(src.join(f)).map_or(0, |m| m.len())).collect();
    // a script's bytecode in memory costs some forty times as much while it is
    // decompiled: a few megabytes of it at once keep all cores busy
    let budget = std::env::var("LUAU_BUDGET").ok().and_then(|v| v.parse().ok()).unwrap_or(2 << 20);
    // each kept packed till the last stage: the place's text is tens of
    // megabytes, a fraction of that packed
    let core: Vec<Result<Packed, String>> = par_map_sized(&sizes, budget, |i| {
        let run = || -> Result<String, String> {
            let data = fs::read(src.join(&files[i])).map_err(|e| e.to_string())?;
            let m = parse(&data, !upstream)?;
            Ok(decompile_core(&m, names.get(sids[i]).map(String::as_str), noflat).join("\n"))
        };
        alloc::job(
            || panic::catch_unwind(AssertUnwindSafe(run)).unwrap_or_else(|e| Err(panic_text(e))).map(|t| Packed::new(&t)),
            |r| r.map(|p| p.copy()).map_err(|e| e.as_str().to_owned()),
        )
    });
    let lines_of = |text: &str| -> Vec<String> { text.split('\n').map(String::from).collect() };

    if timing {
        eprintln!("core  {:?}", t0.elapsed());
    }
    // Naming sees other scripts only through the call sites of the functions
    // a script defines. So name every script in parallel as if it were alone;
    // the result stands unless one of those functions has call sites elsewhere.
    let mut extra = Extra::default();
    let defined: Vec<Vec<String>> = par_map(core.len(), |i| {
        alloc::job(
            || match &core[i] {
                Ok(p) => passes::defined_keys(&lines_of(&p.text())),
                Err(_) => Vec::new(),
            },
            |keys| keys.iter().map(|k| k.as_str().to_owned()).collect(),
        )
    });
    extra.want(defined.iter().flatten().cloned());
    let alone = Extra::default();
    let wanted = extra.wanted();
    // the call sites a script's names give: its text itself is named again
    // at the end, which costs less than holding a second copy of every script
    let tally_of = |text: &str, seen: &Extra| {
        let named = name_round(lines_of(text), seen);
        let mut calls = CallMap::default();
        passes::collect_calls(&named, &mut calls, wanted);
        passes::tally(&calls)
    };
    let draft: Vec<Option<Tally>> = par_map(core.len(), |i| {
        let packed = core[i].as_ref().ok()?;
        alloc::job(
            || panic::catch_unwind(AssertUnwindSafe(|| tally_of(&packed.text(), &alone))).ok(),
            |d| d.map(|calls| calls.clone()),
        )
    });
    if timing {
        eprintln!("draft {:?}", t0.elapsed());
    }

    // Call sites across the place: each script sees the calls of those before
    // it, and of those only the functions it defines (a rename cannot change
    // those keys: it leaves words after `.` and `:` alone). What it calls in
    // turn depends only on scripts further back, so there is one answer however
    // it is reached: in rounds, redo what saw something change until nothing
    // does, which costs one pass in file order only if each waits on the last.
    let mut sites: Vec<Tally> = draft.into_iter().map(Option::unwrap_or_default).collect();
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
            let Ok(packed) = &core[i] else { return Tally::default() };
            panic::catch_unwind(AssertUnwindSafe(|| tally_of(&packed.text(), &seen))).unwrap_or_default()
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

    // a script none of whose functions is called elsewhere is named as it was
    // alone: what the place knows is about functions it does not define
    // each text is read once more, here, and goes as soon as it is written
    let core: Vec<Mutex<Option<Result<Packed, String>>>> = core.into_iter().map(|c| Mutex::new(Some(c))).collect();
    let finish = |i: usize| -> Result<String, String> {
        let taken = core[i].lock().unwrap().take().ok_or_else(|| "named twice".to_string())?;
        let text = &taken.as_ref().map_err(Clone::clone)?.text();
        let seen = if defined[i].iter().any(|k| extra.knows(k)) { &extra } else { &alone };
        panic::catch_unwind(AssertUnwindSafe(|| to_text(&name_round(lines_of(text), seen)))).map_err(panic_text)
    };

    // Paths in order (a repeated game path decides who gets the suffix), then
    // the writes in parallel.
    let mut written: HashSet<PathBuf> = HashSet::default();
    let mut dirs: HashSet<PathBuf> = HashSet::default();
    let mut targets: Vec<(PathBuf, String)> = Vec::with_capacity(files.len());
    for sid in &sids {
        let rel = sanitize(names.get(*sid).map_or(sid, String::as_str), sid);
        let mut path = dst.join(format!("{rel}.lua"));
        if written.contains(&path) {
            path = dst.join(format!("{rel} ({sid}).lua")); // two scripts at the same game path
        }
        written.insert(path.clone());
        if let Some(dir) = path.parent() {
            dirs.insert(dir.to_path_buf());
        }
        targets.push((path, format!("-- {rel} (script {sid})\n")));
    }
    for dir in &dirs {
        let _ = fs::create_dir_all(dir);
    }
    // each written as soon as its text is final: none waits on the others
    let written: Vec<[usize; 5]> = par_map(files.len(), |i| alloc::job(|| {
        let finished = finish(i);
        let (path, head) = &targets[i];
        let result = match &finished {
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
    }, |counts| counts));
    if timing {
        eprintln!("names and write {:?}", t0.elapsed());
    }
    let total = |k: usize| written.iter().map(|w| w[k]).sum::<usize>();
    let ok = total(4);
    let failures = written.len() - ok;
    let bad = failures + total(0);
    let (flags, blocks) = (total(1).min(total(2)), total(3));
    println!("{ok} decompiled, {bad} failed, {flags} flags, {blocks} dispatchers -> {}", dst.display());
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
fn run_stream(upstream: bool, noflat: bool) -> i32 {
    let mut all = Vec::new();
    if let Err(e) = std::io::stdin().read_to_end(&mut all) {
        eprintln!("stdin: {e}");
        return 2;
    }
    let items = read_records(&all);
    let out = Mutex::new(std::io::stdout());
    let written = par_map(items.len(), |i| {
        let one = || -> Result<String, String> {
            let m = parse(items[i], !upstream)?;
            Ok(to_text(&name_round(decompile_core(&m, None, noflat), &Extra::default())))
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
         \x20 --upstream   input is stock Luau bytecode (luau_compile output)"
    );
    2
}

fn run(args: Vec<String>) -> i32 {
    let noflat = std::env::var_os("LUAU_NOFLAT").is_some_and(|v| !v.is_empty());
    let upstream = args.iter().any(|a| a == "--upstream");
    let args: Vec<String> = args.into_iter().filter(|a| a != "--upstream").collect();
    if args.len() >= 2 && args[0] == "--dir" {
        let src = PathBuf::from(&args[1]);
        let dst = match args.iter().position(|a| a == "--out") {
            Some(i) if i + 1 < args.len() => PathBuf::from(&args[i + 1]),
            _ => src.join("lua"),
        };
        return run_dir(&src, &dst, upstream, noflat);
    }
    if args.first().is_some_and(|a| a == "--stream") {
        return run_stream(upstream, noflat);
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
    let text = match parse(&data, !upstream) {
        Ok(m) if disasm => decomp::disassemble(&m),
        Ok(m) => to_text(&name_round(decompile_core(&m, None, noflat), &Extra::default())),
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

    #[test]
    fn output_paths_stay_inside_the_output_folder() {
        assert_eq!(sanitize("../../etc/passwd", "7"), "__/__/etc/passwd");
        assert_eq!(sanitize("a/../b", "7"), "a/__/b");
        assert_eq!(sanitize("/Workspace/Car/", "7"), "Workspace/Car");
        assert_eq!(sanitize("a:b\\c", "7"), "a_b_c");
        assert_eq!(sanitize("///", "7"), "7");
    }
}
