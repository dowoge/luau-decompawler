//! Running things: the real Luau compiler and VM, the decompiler, in parallel.

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::Duration;

/// How far luau_run lets a program go, in calls and loop passes.
pub const USUAL_BUDGET: u64 = 20_000_000;
/// Compiling does not hang; this only keeps a broken compiler from waiting forever.
const COMPILE_LIMIT: u64 = 600;

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

/// Folders on their way out. Deleting thousands of files takes a quarter of the
/// time of making and using them, and nothing has to wait for it.
static LEAVING: Mutex<Vec<thread::JoinHandle<()>>> = Mutex::new(Vec::new());

impl Drop for Temp {
    fn drop(&mut self) {
        let dir = std::mem::take(&mut self.0);
        let leaving = thread::spawn(move || {
            let _ = std::fs::remove_dir_all(dir);
        });
        LEAVING.lock().unwrap().push(leaving);
    }
}

/// Before the process ends: what is still being deleted.
pub fn clean_up() {
    let leaving = std::mem::take(&mut *LEAVING.lock().unwrap());
    for folder in leaving {
        let _ = folder.join();
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

/// With `input` for its standard input, which it may read while it writes.
pub fn run_limited(cmd: &mut Command, input: Option<String>, seconds: u64) -> Ran {
    let stdin = if input.is_some() { Stdio::piped() } else { Stdio::null() };
    let mut child = cmd.stdin(stdin).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("a tool to run");
    if let Some(text) = input {
        let mut pipe = child.stdin.take().expect("piped");
        // closed when done: that is the end of the list
        thread::spawn(move || pipe.write_all(text.as_bytes()));
    }
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

    /// Compiles at -O2 like Roblox, in one process on every core.
    pub fn compile(&self, files: &[String], outdir: &str) -> Compiled {
        let mut all = Compiled { bytecode: HashMap::new(), failed: Vec::new() };
        if files.is_empty() {
            return all;
        }
        let wanted: HashSet<&str> = files.iter().map(String::as_str).collect();
        std::fs::create_dir_all(outdir).expect("a folder for bytecode");
        let mut check = command(&self.check);
        check.arg("--emit").arg(outdir).arg("--stdin");
        let ran = run_limited(&mut check, Some(list(files)), COMPILE_LIMIT);
        if text_of(&ran.stderr).contains("--stdin: cannot open") {
            panic!("{} is older than this harness: rebuild the tools (CMakeLists.txt)", self.check);
        }
        for line in text_of(&ran.stdout).lines() {
            match line.split_once(' ') {
                Some((n, path)) if n.bytes().all(|c| c.is_ascii_digit()) && wanted.contains(path) => {
                    let bc = Path::new(outdir).join(format!("{n}.bc")).to_string_lossy().into_owned();
                    all.bytecode.insert(path.to_string(), bc);
                }
                _ => {
                    if let Some((path, why)) = line.split_once(": ").filter(|p| wanted.contains(p.0)) {
                        all.failed.push((path.to_string(), why.to_string()));
                    }
                }
            }
        }
        all
    }

    /// What each file prints, ending with its result line, from one process on
    /// every core. A file still going after `seconds` has none: luau_run leaves
    /// it behind and finishes the rest.
    pub fn outputs(&self, files: &[String], seconds: u64, budget: u64) -> HashMap<String, String> {
        if files.is_empty() {
            return HashMap::new();
        }
        let run = |part: &[String]| {
            let mut cmd = command(&self.run);
            cmd.arg("--budget").arg(budget.to_string()).arg("--seconds").arg(seconds.to_string()).arg("--stdin");
            run_limited(&mut cmd, Some(list(part)), seconds * 4 + 60)
        };
        let ran = run(files);
        let mut found = parse_outputs(&text_of(&ran.stdout));
        if found.contains_key("--stdin") {
            // it would have every program never finish, and tests pass
            panic!("{} is older than this harness: rebuild the tools (CMakeLists.txt)", self.run);
        }
        if !ran.status.is_some_and(|s| s.success()) {
            // a crash took every unfinished file with it: each goes again by itself
            let rest: Vec<String> = files.iter().filter(|f| !found.contains_key(*f)).cloned().collect();
            let again = par_map(&rest, |f| parse_outputs(&text_of(&run(std::slice::from_ref(f)).stdout)));
            found.extend(again.into_iter().flatten());
        }
        found
    }
}

/// Paths one per line, for a tool reading them from its standard input.
fn list(files: &[String]) -> String {
    let mut text = files.join("\n");
    text.push('\n');
    text
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
    let run = run_limited(command(binary).arg("--upstream").arg(bc), None, seconds);
    let Some(status) = run.status else { return Err("timed out".to_string()) };
    if !status.success() {
        let err = text_of(&run.stderr);
        let tail = err.trim().lines().last().unwrap_or("").to_string();
        return Err(format!("exit status {}: {tail}", status.code().unwrap_or(-1)));
    }
    Ok(text_of(&run.stdout))
}

/// Decompiles many bytecode files in one process: for each, the path of its
/// text, or why there is none. A process that hung or crashed took its
/// unfinished files with it, and what it wrote may be cut short, so then they
/// all go through again one at a time; so do any it left no word about. A
/// binary under test may predate the list on stdin: it would take `--stdin`
/// for a file, so only one that says how many it read is believed.
pub fn decompile_many(binary: &str, files: &[String], dir: &str, seconds: u64) -> Vec<Result<String, String>> {
    if files.is_empty() {
        return Vec::new();
    }
    let mut each = command(binary);
    each.arg("--upstream").arg("--each").arg(dir).arg("--stdin");
    let ran = run_limited(&mut each, Some(list(files)), seconds);
    let read = format!("{} files", files.len());
    let clean = ran.status.is_some_and(|s| s.success()) && text_of(&ran.stdout).lines().last() == Some(read.as_str());
    let alone = |bc: &String, text: String| {
        let _ = std::fs::create_dir_all(dir);
        decompile(binary, bc, seconds).and_then(|t| std::fs::write(&text, t).map(|_| text).map_err(|e| e.to_string()))
    };
    let results = files.iter().enumerate().map(|(i, bc)| {
        let text = format!("{dir}/{i}.lua");
        match std::fs::read_to_string(format!("{dir}/{i}.error")) {
            Ok(why) if clean => Err(why),
            _ if clean && Path::new(&text).exists() => Ok(text),
            _ => alone(bc, text),
        }
    });
    results.collect()
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
