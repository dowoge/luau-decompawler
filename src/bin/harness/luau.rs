//! Running things: the real Luau compiler and VM, the decompiler, in parallel.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

/// Compiling or running this many files at once keeps within the Windows
/// limit on command lines.
pub const BATCH: usize = 40;

pub struct Args(pub Vec<String>);

impl Args {
    pub fn flag(&self, name: &str) -> bool {
        self.0.iter().any(|a| a == name)
    }

    pub fn opt(&self, name: &str) -> Option<&str> {
        let at = self.0.iter().position(|a| a == name)?;
        self.0.get(at + 1).map(String::as_str)
    }

    pub fn num<T: FromStr>(&self, name: &str, default: T) -> T {
        self.opt(name).and_then(|v| v.parse().ok()).unwrap_or(default)
    }
}

pub fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn workers() -> usize {
    thread::available_parallelism().map_or(4, |n| n.get())
}

/// f over items, on all cores, the results in order.
pub fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let done: Mutex<Vec<Option<R>>> = Mutex::new(items.iter().map(|_| None).collect());
    thread::scope(|s| {
        for _ in 0..workers().min(items.len()) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(item) = items.get(i) else { break };
                let r = f(item);
                done.lock().unwrap()[i] = Some(r);
            });
        }
    });
    done.into_inner().unwrap().into_iter().map(|r| r.expect("a worker died")).collect()
}

/// A folder that goes away with this value.
pub struct Temp(pub PathBuf);

impl Temp {
    pub fn new(tag: &str) -> Temp {
        static SERIAL: AtomicUsize = AtomicUsize::new(0);
        let n = SERIAL.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("luau_{tag}_{}_{n}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temporary folder");
        Temp(dir)
    }

    pub fn path(&self, name: &str) -> String {
        self.0.join(name).to_string_lossy().into_owned()
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A process with no console of its own: nothing here may open a window.
pub fn command(program: &str) -> Command {
    let mut c = Command::new(program);
    c.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // no console at all: a hidden one for every process would cost milliseconds
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        c.creation_flags(DETACHED_PROCESS);
    }
    c
}

/// Text as it came, with Windows line ends gone.
pub fn text_of(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

pub fn read_text(path: &str) -> String {
    text_of(&std::fs::read(path).unwrap_or_default())
}

/// None if it was still running after this long.
pub fn run_limited(cmd: &mut Command, seconds: u64) -> Option<Output> {
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().ok()?;
    let mut out = child.stdout.take()?;
    let mut err = child.stderr.take()?;
    let out = thread::spawn(move || {
        let mut b = Vec::new();
        let _ = out.read_to_end(&mut b);
        b
    });
    let err = thread::spawn(move || {
        let mut b = Vec::new();
        let _ = err.read_to_end(&mut b);
        b
    });
    let stop = Instant::now() + Duration::from_secs(seconds);
    let mut nap = Duration::from_millis(1);
    let status = loop {
        if let Some(s) = child.try_wait().ok()? {
            break s;
        }
        if Instant::now() >= stop {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        thread::sleep(nap);
        nap = (nap * 2).min(Duration::from_millis(20));
    };
    Some(Output { status, stdout: out.join().ok()?, stderr: err.join().ok()? })
}

pub struct Tools {
    pub check: String,
    pub run: String,
    pub decompiler: String,
}

impl Tools {
    pub fn find() -> Tools {
        let suffix = std::env::consts::EXE_SUFFIX;
        let bin = std::env::var("LUAU_BIN").map_or_else(|_| repo().join("build").join("Release"), PathBuf::from);
        let at = |dir: &Path, name: &str| dir.join(format!("{name}{suffix}")).to_string_lossy().into_owned();
        Tools {
            check: at(&bin, "luau_check"),
            run: at(&bin, "luau_run"),
            decompiler: at(&repo().join("target").join("release"), "luau_decompile"),
        }
    }

    /// Compiles at -O2 like Roblox: {source path: bytecode path}.
    pub fn emit(&self, files: &[String], outdir: &str, batch: usize) -> HashMap<String, String> {
        let wanted: HashSet<&String> = files.iter().collect();
        let batches: Vec<(usize, &[String])> = files.chunks(batch).enumerate().collect();
        let done = par_map(&batches, |(n, part)| {
            let sub = Path::new(outdir).join(n.to_string());
            std::fs::create_dir_all(&sub).expect("a folder for bytecode");
            let sub = sub.to_string_lossy().into_owned();
            let out = command(&self.check).arg("--emit").arg(&sub).args(*part).output().expect("luau_check");
            let mut found = Vec::new();
            for line in text_of(&out.stdout).lines() {
                if let Some((n, path)) = line.split_once(' ') {
                    if n.bytes().all(|c| c.is_ascii_digit()) && wanted.contains(&path.to_string()) {
                        found.push((path.to_string(), Path::new(&sub).join(format!("{n}.bc")).to_string_lossy().into_owned()));
                    }
                }
            }
            found
        });
        done.into_iter().flatten().collect()
    }

    /// The files that do not compile, each with its error.
    pub fn errors(&self, files: &[String]) -> Vec<String> {
        let batches: Vec<&[String]> = files.chunks(BATCH).collect();
        let done = par_map(&batches, |part| {
            let out = command(&self.check).args(*part).output().expect("luau_check");
            text_of(&out.stdout)
                .lines()
                .filter(|l| l.contains(": ") && !l.ends_with("fail"))
                .map(String::from)
                .collect::<Vec<String>>()
        });
        done.into_iter().flatten().collect()
    }

    /// What each file prints, ending with its result line.
    pub fn outputs(&self, files: &[String]) -> HashMap<String, String> {
        let batches: Vec<&[String]> = files.chunks(BATCH).collect();
        let done = par_map(&batches, |part| {
            let out = command(&self.run).args(*part).output().expect("luau_run");
            parse_outputs(&text_of(&out.stdout))
        });
        done.into_iter().flatten().collect()
    }
}

/// `=== FILE`, what it printed, then `=== ok` or `=== error: MESSAGE`.
pub fn parse_outputs(text: &str) -> HashMap<String, String> {
    let mut res = HashMap::new();
    let mut cur: Option<String> = None;
    let mut buf: Vec<&str> = Vec::new();
    for line in text.lines() {
        let tag = line.strip_prefix("=== ").filter(|t| !t.is_empty());
        match (tag, &cur) {
            (Some(t), None) => {
                cur = Some(t.to_string());
                buf.clear();
            }
            (Some(t), Some(_)) if t == "ok" || t.starts_with("error:") => {
                buf.push(line);
                res.insert(cur.take().unwrap_or_default(), buf.join("\n"));
                buf.clear();
            }
            _ => buf.push(line),
        }
    }
    res
}

/// The decompiled text of a bytecode file, or why there is none.
pub fn decompile(binary: &str, bc: &str) -> Result<String, String> {
    let Some(p) = run_limited(command(binary).arg("--upstream").arg(bc), 300) else {
        return Err("timed out".to_string());
    };
    if !p.status.success() {
        let err = text_of(&p.stderr);
        let tail = err.trim().lines().last().unwrap_or("").to_string();
        return Err(format!("exit status {}: {tail}", p.status.code().unwrap_or(-1)));
    }
    Ok(text_of(&p.stdout))
}

/// Parses bytecode the compiler just wrote.
pub fn module(bc: &str) -> crate::bytecode::Module {
    crate::bytecode::parse(&std::fs::read(bc).expect("bytecode"), false).expect("the compiler's own bytecode parses")
}

pub fn percent(part: usize, whole: usize) -> String {
    format!("{:.1}%", part as f64 / whole.max(1) as f64 * 100.0)
}

/// Every file under dir.
pub fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, found);
        } else {
            found.push(p);
        }
    }
}

/// The first `count` characters of a line.
pub fn clip(line: &str, count: usize) -> String {
    line.chars().take(count).collect()
}
