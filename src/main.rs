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
//!
//! Output is Luau that compiles. `harness difftest` is the ground-truth test:
//! generated programs must behave the same after a trip through here.
//!
//! LUAU_NOFLAT=1 prints leftover jumps as `goto` instead of restructuring them.
//! LUAU_THREADS=N limits the workers; LUAU_TIMING=1 prints stage times;
//! LUAU_WHY=1 lists each `jump_N` flag and what stopped its helper coming back.

mod analysis;
mod bytecode;
mod decomp;
mod expr;
mod inline;
mod passes;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use bytecode::{parse, Module};
use passes::{CallMap, Extra};

/// Deeply nested scripts recurse deeply; a worker's stack is reserved, not committed.
const STACK: usize = 1 << 30;

/// Everything up to the point where other scripts' call sites matter.
fn decompile_core(m: &Module, path: Option<&str>, noflat: bool) -> Vec<String> {
    let body = decomp::decompile_lines(m, noflat);
    let split: Vec<String> = body.join("\n").split('\n').map(String::from).collect();
    let lines = passes::if_expressions(split);
    let lines = passes::uninline(lines);
    let lines = passes::module_table(lines, path);
    passes::tidy_before_names(lines)
}

/// The naming rounds that can use other scripts' call sites.
fn name_round(lines: Vec<String>, extra: &Extra) -> Vec<String> {
    let lines = passes::more_names(lines, extra);
    let lines = passes::more_names(lines, extra);
    passes::blank_lines(lines)
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
    let mut names = HashMap::new();
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
    let draft: Vec<Option<(Vec<String>, CallMap)>> = par_map(core.len(), |i| {
        let lines = core[i].as_ref().ok()?;
        let run = || {
            let named = name_round(lines.clone(), &alone);
            let mut calls = CallMap::new();
            passes::collect_calls(&named, &mut calls, wanted);
            (named, calls)
        };
        panic::catch_unwind(AssertUnwindSafe(run)).ok()
    });
    if timing {
        eprintln!("draft {:?}", t0.elapsed());
    }

    // Call sites across the place. Each script sees the calls of those before
    // it, so this one step is sequential and in order.
    let mut redone = 0usize;
    for (i, d) in draft.iter().enumerate() {
        let Ok(lines) = &core[i] else { continue };
        if defined[i].iter().any(|k| extra.knows(k)) {
            redone += 1;
            let run = || {
                let named = name_round(lines.clone(), &extra);
                let mut calls = CallMap::new();
                passes::collect_calls(&named, &mut calls, extra.wanted());
                calls
            };
            if let Ok(calls) = panic::catch_unwind(AssertUnwindSafe(run)) {
                extra.add(calls);
            }
        } else if let Some((_, calls)) = d {
            extra.add(calls.clone());
        }
    }
    if timing {
        eprintln!("calls {:?} ({redone} of {} redone)", t0.elapsed(), core.len());
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
    let mut written: HashSet<PathBuf> = HashSet::new();
    let mut dirs: HashSet<PathBuf> = HashSet::new();
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
    let write_errors: usize = par_map(files.len(), |i| {
        let (path, head) = &targets[i];
        let result = match &finished[i] {
            Ok(t) => fs::File::create(path)
                .and_then(|mut f| f.write_all(head.as_bytes()).and_then(|_| f.write_all(t.as_bytes()))),
            Err(e) => fs::write(path, format!("{head}-- decompile failed: {e}\n")),
        };
        match result {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{}: {e}", path.display());
                1
            }
        }
    })
    .into_iter()
    .sum();
    let bad = failures + write_errors;
    let count = |needle: &str| -> usize {
        finished.iter().flatten().map(|text| text.matches(needle).count()).sum()
    };
    let (flags, blocks) = (count(" = false\n").min(count("local jump_")), count("local block = 0"));
    println!("{ok} decompiled, {bad} failed, {flags} flags, {blocks} dispatchers -> {}", dst.display());
    if bad == 0 {
        0
    } else {
        1
    }
}

fn usage() -> i32 {
    println!(
        "luau_decompile FILE.wire | --disasm FILE.wire | --dir DIR [--out OUT]\n\
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
