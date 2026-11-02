//! Running things: the real Luau compiler and VM, the decompiler, in parallel.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::Duration;

/// Compiling or running this many files at once keeps within the Windows
/// limit on command lines.
pub const BATCH: usize = 40;
/// Files for one decompiler process. It uses every core, so more would be
/// better, but this many paths always fit on a Windows command line.
const EACH: usize = 100;
/// How far luau_run lets a program go, in calls and loop passes.
pub const USUAL_BUDGET: u64 = 20_000_000;

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

/// What a process said. No status if it was still running after the limit:
/// it was killed, and this is what it had said by then.
pub struct Ran {
    pub status: Option<ExitStatus>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub fn run_limited(cmd: &mut Command, seconds: u64) -> Ran {
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("a tool to run");
    let read_all = |mut pipe: Box<dyn Read + Send>, ended: Option<mpsc::Sender<()>>| {
        thread::spawn(move || {
            let mut b = Vec::new();
            let _ = pipe.read_to_end(&mut b);
            if let Some(ended) = ended {
                let _ = ended.send(());
            }
            b
        })
    };
    let (ended, wait) = mpsc::channel();
    let out = read_all(Box::new(child.stdout.take().expect("piped")), Some(ended));
    let err = read_all(Box::new(child.stderr.take().expect("piped")), None);
    // its output ends when it does: nothing to poll
    let finished = wait.recv_timeout(Duration::from_secs(seconds)).is_ok();
    if !finished {
        let _ = child.kill();
    }
    let status = child.wait().ok().filter(|_| finished);
    Ran { status, stdout: out.join().unwrap_or_default(), stderr: err.join().unwrap_or_default() }
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

    /// Compiles at -O2 like Roblox.
    pub fn compile(&self, files: &[String], outdir: &str, batch: usize) -> Compiled {
        let wanted: HashSet<&str> = files.iter().map(String::as_str).collect();
        let batches: Vec<(usize, &[String])> = files.chunks(batch).enumerate().collect();
        let done = par_map(&batches, |(n, part)| {
            let sub = Path::new(outdir).join(n.to_string());
            std::fs::create_dir_all(&sub).expect("a folder for bytecode");
            let sub = sub.to_string_lossy().into_owned();
            let out = command(&self.check).arg("--emit").arg(&sub).args(*part).output().expect("luau_check");
            let mut good = Vec::new();
            let mut bad = Vec::new();
            for line in text_of(&out.stdout).lines() {
                match line.split_once(' ') {
                    Some((n, path)) if n.bytes().all(|c| c.is_ascii_digit()) && wanted.contains(path) => {
                        let bc = Path::new(&sub).join(format!("{n}.bc")).to_string_lossy().into_owned();
                        good.push((path.to_string(), bc));
                    }
                    _ => {
                        if let Some((path, why)) = line.split_once(": ").filter(|p| wanted.contains(p.0)) {
                            bad.push((path.to_string(), why.to_string()));
                        }
                    }
                }
            }
            (good, bad)
        });
        let mut all = Compiled { bytecode: HashMap::new(), failed: Vec::new() };
        for (good, bad) in done {
            all.bytecode.extend(good);
            all.failed.extend(bad);
        }
        all
    }

    /// What each file prints, ending with its result line. A file still going
    /// after `seconds` has none, and the rest of its batch carries on without it.
    pub fn outputs(&self, files: &[String], seconds: u64, budget: u64) -> HashMap<String, String> {
        let batches: Vec<&[String]> = files.chunks(BATCH).collect();
        let done = par_map(&batches, |part| {
            let mut found = HashMap::new();
            let mut rest: &[String] = part;
            while !rest.is_empty() {
                let run = run_limited(command(&self.run).arg("--budget").arg(budget.to_string()).args(rest), seconds);
                found.extend(parse_outputs(&text_of(&run.stdout)));
                if run.status.is_some() {
                    break;
                }
                let hung = rest.iter().position(|f| !found.contains_key(f)).unwrap_or(rest.len());
                rest = &rest[(hung + 1).min(rest.len())..];
            }
            found
        });
        done.into_iter().flatten().collect()
    }
}

pub struct Compiled {
    /// {source path: bytecode path}.
    pub bytecode: HashMap<String, String>,
    /// (source path, error).
    pub failed: Vec<(String, String)>,
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

/// One bytecode file by itself: the decompiled text, or why there is none.
fn decompile(binary: &str, bc: &str, seconds: u64) -> Result<String, String> {
    let run = run_limited(command(binary).arg("--upstream").arg(bc), seconds);
    let Some(status) = run.status else { return Err("timed out".to_string()) };
    if !status.success() {
        let err = text_of(&run.stderr);
        let tail = err.trim().lines().last().unwrap_or("").to_string();
        return Err(format!("exit status {}: {tail}", status.code().unwrap_or(-1)));
    }
    Ok(text_of(&run.stdout))
}

/// Decompiles many bytecode files through as few processes as the command line
/// allows: for each, the path of its text, or why there is none. A process that
/// hung or crashed took the rest of its files with it, and what it wrote may be
/// cut short, so its files go through again one at a time.
pub fn decompile_many(binary: &str, files: &[String], dir: &str, seconds: u64) -> Vec<Result<String, String>> {
    let mut results = Vec::with_capacity(files.len());
    for (n, part) in files.chunks(EACH).enumerate() {
        let out = format!("{dir}/{n}");
        let run = run_limited(command(binary).arg("--upstream").arg("--each").arg(&out).args(part), seconds);
        let clean = run.status.is_some_and(|s| s.success());
        for (i, bc) in part.iter().enumerate() {
            let text = format!("{out}/{i}.lua");
            results.push(if clean {
                match std::fs::read_to_string(format!("{out}/{i}.error")) {
                    Ok(why) => Err(why),
                    Err(_) => Ok(text),
                }
            } else {
                let _ = std::fs::create_dir_all(&out);
                decompile(binary, bc, seconds).and_then(|t| std::fs::write(&text, t).map(|_| text).map_err(|e| e.to_string()))
            });
        }
    }
    results
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
