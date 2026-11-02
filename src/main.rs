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
//!   luau_decompile --each OUT FILE...     each file by itself, as if run alone:
//!                                         OUT/<n>.lua, or OUT/<n>.error
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
mod passes;

use std::fs;
use std::io::Write;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use bytecode::{parse, Module};
use hash::{HashMap, HashSet};
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
    passes::blank_lines(passes::more_names(lines, extra))
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
                .spawn_scoped(s, || loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= n {
                        break;
                    }
                    let v = f(i);
                    *slots[i].lock().unwrap() = Some(v);
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
    let core: Vec<Result<Vec<String>, String>> = par_map(files.len(), |i| {
        let run = || -> Result<Vec<String>, String> {
            let data = fs::read(src.join(&files[i])).map_err(|e| e.to_string())?;
            let m = parse(&data, !upstream)?;
            Ok(decompile_core(&m, names.get(sids[i]).map(String::as_str), noflat))
        };
        panic::catch_unwind(AssertUnwindSafe(run)).unwrap_or_else(|e| Err(panic_text(e)))
    });

    if timing {
        eprintln!("core  {:?}", t0.elapsed());
    }
    // Naming sees other scripts only through the call sites of the functions
    // a script defines. So name every script in parallel as if it were alone;
    // the result stands unless one of those functions has call sites elsewhere.
    let mut extra = Extra::default();
    let defined: Vec<Vec<String>> = par_map(core.len(), |i| match &core[i] {
        Ok(lines) => passes::defined_keys(lines),
        Err(_) => Vec::new(),
    });
    extra.want(defined.iter().flatten().cloned());
    let alone = Extra::default();
    let wanted = extra.wanted();
    let name_and_tally = |lines: &[String], seen: &Extra| {
        let named = name_round(lines.to_vec(), seen);
        let mut calls = CallMap::default();
        passes::collect_calls(&named, &mut calls, wanted);
        (named, passes::tally(&calls))
    };
    let mut draft: Vec<Option<(Vec<String>, Tally)>> = par_map(core.len(), |i| {
        let lines = core[i].as_ref().ok()?;
        panic::catch_unwind(AssertUnwindSafe(|| name_and_tally(lines, &alone))).ok()
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
    let mut sites: Vec<Tally> =
        draft.iter_mut().map(|d| d.as_mut().map(|d| std::mem::take(&mut d.1)).unwrap_or_default()).collect();
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
        let fresh: Vec<Tally> = par_map(todo.len(), |t| {
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
            let Ok(lines) = &core[i] else { return Tally::default() };
            panic::catch_unwind(AssertUnwindSafe(|| name_and_tally(lines, &seen).1)).unwrap_or_default()
        });
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

    let finished: Vec<Result<String, String>> = par_map(files.len(), |i| {
        let lines = match &core[i] {
            Ok(lines) => lines,
            Err(e) => return Err(e.clone()),
        };
        if !defined[i].iter().any(|k| extra.knows(k)) {
            if let Some((named, _)) = &draft[i] {
                return Ok(to_text(named));
            }
        }
        panic::catch_unwind(AssertUnwindSafe(|| to_text(&name_round(lines.clone(), &extra))))
            .map_err(panic_text)
    });
    if timing {
        eprintln!("names {:?}", t0.elapsed());
    }

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
    let ok = finished.iter().filter(|r| r.is_ok()).count();
    let failures = finished.len() - ok;
    let written: Vec<[usize; 4]> = par_map(files.len(), |i| {
        let (path, head) = &targets[i];
        let result = match &finished[i] {
            Ok(t) => write_text(path, head, t),
            Err(e) => fs::write(path, format!("{head}-- decompile failed: {e}\n")),
        };
        if let Err(e) = &result {
            eprintln!("{}: {e}", path.display());
        }
        let count = |needle: &str| finished[i].as_ref().map_or(0, |text| text.matches(needle).count());
        [usize::from(result.is_err()), count(" = false\n"), count("local jump_"), count("local block = 0")]
    });
    if timing {
        eprintln!("write {:?}", t0.elapsed());
    }
    let total = |k: usize| written.iter().map(|w| w[k]).sum::<usize>();
    let bad = failures + total(0);
    let (flags, blocks) = (total(1).min(total(2)), total(3));
    println!("{ok} decompiled, {bad} failed, {flags} flags, {blocks} dispatchers -> {}", dst.display());
    if bad == 0 {
        0
    } else {
        1
    }
}

/// Starting a process costs far more than decompiling a small program, so
/// many can share one. Nothing here depends on the others.
fn run_each(out: &Path, files: &[String], upstream: bool, noflat: bool) -> i32 {
    if let Err(e) = fs::create_dir_all(out) {
        eprintln!("{}: {e}", out.display());
        return 2;
    }
    let written = par_map(files.len(), |i| {
        let one = || -> Result<String, String> {
            let data = fs::read(&files[i]).map_err(|e| e.to_string())?;
            let m = parse(&data, !upstream)?;
            Ok(to_text(&name_round(decompile_core(&m, None, noflat), &Extra::default())))
        };
        match panic::catch_unwind(AssertUnwindSafe(one)).unwrap_or_else(|e| Err(panic_text(e))) {
            Ok(text) => fs::write(out.join(format!("{i}.lua")), text),
            Err(why) => fs::write(out.join(format!("{i}.error")), why),
        }
        .is_ok()
    });
    i32::from(written.contains(&false))
}

fn usage() -> i32 {
    println!(
        "luau_decompile FILE.wire | --disasm FILE.wire | --dir DIR [--out OUT] | --each OUT FILE...\n\
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
    if args.len() >= 2 && args[0] == "--each" {
        return run_each(Path::new(&args[1]), &args[2..], upstream, noflat);
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
