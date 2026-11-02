//! Running things: the real Luau compiler and VM, the decompiler, in parallel.
//!
//! Each tool runs as a process of its own, so that a crash or a hang can be
//! seen and survived, but one process does a whole step on every core, and the
//! programs go there and back through its standard input and output: written
//! to files, they cost more than the work done on them.

use std::collections::HashMap;
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

/// Folders on their way out: nothing has to wait for them to go.
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

/// A program or its bytecode, in memory, and the name results come back under.
#[derive(Clone)]
pub struct Item {
    pub name: String,
    pub data: Vec<u8>,
}

impl Item {
    pub fn new(name: String, data: impl Into<Vec<u8>>) -> Item {
        Item { name, data: data.into() }
    }
}

/// What a process said. No status if it was still running after the limit:
/// it was killed, and this is what it had said by then.
pub struct Ran {
    pub status: Option<ExitStatus>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// With `input` for its standard input, which it may read while it writes.
pub fn run_limited(cmd: &mut Command, input: Option<Vec<u8>>, seconds: u64) -> Ran {
    let stdin = if input.is_some() { Stdio::piped() } else { Stdio::null() };
    let mut child = cmd.stdin(stdin).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("a tool to run");
    if let Some(bytes) = input {
        let mut pipe = child.stdin.take().expect("piped");
        // closed when done: that is the end of the input
        thread::spawn(move || pipe.write_all(&bytes));
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

/// Items for a tool's standard input: `<length> <name>` and the bytes, each.
fn records<'a>(items: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, data) in items {
        out.extend_from_slice(format!("{} {name}\n", data.len()).as_bytes());
        out.extend_from_slice(data);
    }
    out
}

/// What a tool sent back, by index: `<index> ok <length>` and the result, or
/// `<index> error <length>` and why, in whatever order they were done. None
/// where nothing came.
fn results(out: &[u8], count: usize) -> Vec<Option<Result<Vec<u8>, String>>> {
    let mut found = vec![None; count];
    let mut at = 0;
    while let Some(eol) = out[at..].iter().position(|b| *b == b'\n').map(|p| at + p) {
        let head = String::from_utf8_lossy(&out[at..eol]).into_owned();
        let parts: Vec<&str> = head.split(' ').collect();
        let [index, kind, size] = parts[..] else { break };
        let (Ok(index), Ok(size)) = (index.parse::<usize>(), size.parse::<usize>()) else { break };
        let Some(body) = out.get(eol + 1..eol + 1 + size) else { break };
        if let Some(slot) = found.get_mut(index) {
            *slot = Some(if kind == "ok" { Ok(body.to_vec()) } else { Err(text_of(body)) });
        }
        at = eol + 1 + size;
    }
    found
}

fn outdated(tool: &str) -> ! {
    panic!("{tool} is older than this harness: rebuild the tools (see CMakeLists.txt)")
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

    /// Compiles each source at -O2 like Roblox: its bytecode, or why there is none.
    pub fn compile(&self, sources: &[Item]) -> Vec<Result<Vec<u8>, String>> {
        if sources.is_empty() {
            return Vec::new();
        }
        let mut check = command(&self.check);
        check.arg("--stream");
        let input = records(sources.iter().map(|s| (s.name.as_str(), s.data.as_slice())));
        let ran = run_limited(&mut check, Some(input), COMPILE_LIMIT);
        if text_of(&ran.stderr).contains("--stream: cannot open") {
            outdated(&self.check);
        }
        let found = results(&ran.stdout, sources.len());
        found.into_iter().map(|r| r.unwrap_or_else(|| Err("the compiler gave no answer".to_string()))).collect()
    }

    /// What each program prints, ending with its result line, under its name. A
    /// program still going after `seconds` has none: luau_run leaves it behind
    /// and finishes the rest.
    pub fn outputs(&self, programs: &[Item], seconds: u64, budget: u64) -> HashMap<String, String> {
        if programs.is_empty() {
            return HashMap::new();
        }
        let run = |part: &[Item]| {
            let mut cmd = command(&self.run);
            cmd.arg("--budget").arg(budget.to_string()).arg("--seconds").arg(seconds.to_string()).arg("--stream");
            let input = records(part.iter().map(|p| (p.name.as_str(), p.data.as_slice())));
            run_limited(&mut cmd, Some(input), seconds * 4 + 60)
        };
        let ran = run(programs);
        let mut found = parse_outputs(&text_of(&ran.stdout));
        if found.contains_key("--stream") {
            // it would have every program never finish, and tests pass
            outdated(&self.run);
        }
        if !ran.status.is_some_and(|s| s.success()) {
            // a crash took every unfinished program with it: each goes again by itself
            let rest: Vec<&Item> = programs.iter().filter(|p| !found.contains_key(&p.name)).collect();
            let again = par_map(&rest, |p| parse_outputs(&text_of(&run(std::slice::from_ref(*p)).stdout)));
            found.extend(again.into_iter().flatten());
        }
        found
    }
}

/// Where a value lives differs between any two runs: `table: 0x...` and its
/// kind all read `table: 0x?`.
fn mask_addresses(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(": 0x") {
        let kind = ["table", "function", "userdata", "thread", "buffer"].iter().any(|k| rest[..at].ends_with(k));
        out.push_str(&rest[..at + 4]);
        rest = &rest[at + 4..];
        if kind {
            out.push('?');
            rest = &rest[rest.bytes().take_while(u8::is_ascii_hexdigit).count()..];
        }
    }
    out.push_str(rest);
    out
}

/// `=== NAME`, what it printed, then `=== ok` or `=== error: MESSAGE`.
pub fn parse_outputs(text: &str) -> HashMap<String, String> {
    let text = mask_addresses(text);
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

/// Decompiles each bytecode by itself, in one process: its text, or why there
/// is none. What that process finished stands even if it later hung or
/// crashed; what it did not goes again one file at a time, and so does all of
/// it for a binary under test that predates --stream.
pub fn decompile_many(binary: &str, bytecode: &[Vec<u8>], seconds: u64) -> Vec<Result<String, String>> {
    if bytecode.is_empty() {
        return Vec::new();
    }
    let names: Vec<String> = (0..bytecode.len()).map(|i| i.to_string()).collect();
    let input = records(names.iter().map(String::as_str).zip(bytecode.iter().map(Vec::as_slice)));
    let mut stream = command(binary);
    stream.arg("--upstream").arg("--stream");
    let ran = run_limited(&mut stream, Some(input), seconds);
    let mut found = results(&ran.stdout, bytecode.len());
    let missing: Vec<usize> = (0..bytecode.len()).filter(|i| found[*i].is_none()).collect();
    if !missing.is_empty() {
        let tmp = Temp::new("dc");
        let again = par_map(&missing, |i| {
            let path = tmp.path(&format!("{i}.bc"));
            std::fs::write(&path, &bytecode[*i]).map_err(|e| e.to_string())?;
            decompile(binary, &path, seconds).map(String::into_bytes)
        });
        for (i, result) in missing.iter().zip(again) {
            found[*i] = Some(result);
        }
    }
    found.into_iter().map(|r| r.expect("an answer for each").map(|text| text_of(&text))).collect()
}

/// Parses bytecode the compiler just wrote.
pub fn module(bc: &[u8]) -> crate::bytecode::Module {
    crate::bytecode::parse(bc, false).expect("the compiler's own bytecode parses")
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
