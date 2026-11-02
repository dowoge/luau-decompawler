//! Which real functions differ, and how. For each function that does not come
//! back instruction for instruction, the opcode diff reduced to one key:
//! `+[LOADNIL]` means adding that is ALL that separates it from the original.
//! --save stores the set of differing functions; --since prints those that differ
//! now and did not then, and how many got fixed.
//!
//! Reads decompiled scripts from $LUAU_RS/<place> (default out/<place>) against
//! the dumps in $LUAU_RUN/<place> (default run/<place>).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::bytecode::{parse_as, Module, Opcodes};
use crate::diff::{Matcher, Tag};
use crate::luau::{clip, module, par_map, read_text, repo, walk, Args, Item, Tools};
use crate::shape::{pair, shape};

type Shape = Vec<&'static str>;

#[derive(Clone)]
pub struct Row {
    place: String,
    path: String,
    script: String,
    proto: usize,
    name: Option<String>,
    a: Shape,
    b: Shape,
}

impl Row {
    fn id(&self) -> (String, String, usize) {
        (self.place.clone(), self.script.clone(), self.proto)
    }
}

pub struct Place {
    pub rows: Vec<Row>,
    pub functions: usize,
}

pub fn dumps_root() -> PathBuf {
    std::env::var("LUAU_RUN").map_or_else(|_| repo().join("run"), PathBuf::from)
}

fn output_root() -> PathBuf {
    std::env::var("LUAU_RS").map_or_else(|_| repo().join("out"), PathBuf::from)
}

/// Folders of dumped scripts.
pub fn dump_places(dumps: &Path) -> Vec<String> {
    let has_wire = |p: &Path| {
        std::fs::read_dir(p).is_ok_and(|d| d.flatten().any(|e| e.path().extension().is_some_and(|x| x == "wire")))
    };
    let mut places: Vec<String> = std::fs::read_dir(dumps)
        .map(|d| {
            d.flatten()
                .filter(|e| has_wire(&e.path()))
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| !n.ends_with("_lua"))
                .collect()
        })
        .unwrap_or_default();
    places.sort();
    places
}

/// Each decompiled script and the script number its first line gives.
fn scripts(dir: &Path) -> BTreeMap<String, String> {
    let mut files = Vec::new();
    walk(dir, &mut files);
    let mut found = BTreeMap::new();
    for f in files.iter().filter(|f| f.extension().is_some_and(|x| x == "lua")) {
        let path = f.to_string_lossy().into_owned();
        let text = read_text(&path);
        let first = text.lines().next().unwrap_or("");
        let id = first.split_once("(script ").and_then(|(_, rest)| rest.split_once(')')).map(|p| p.0);
        if let Some(id) = id.filter(|i| !i.is_empty() && i.bytes().all(|c| c.is_ascii_digit())) {
            found.insert(path, id.to_string());
        }
    }
    found
}

pub fn compare_place(tools: &Tools, dumps: &Path, output: &Path, place: &str) -> Place {
    let wire = dumps.join(place);
    let files = scripts(&output.join(place));
    let paths: Vec<String> = files.keys().cloned().collect();
    let texts: Vec<Item> = par_map(&paths, |p| Item::new(p.clone(), std::fs::read(p).unwrap_or_default()));
    let done = tools.compile(&texts);
    let both: Vec<(&String, &Result<Vec<u8>, String>)> = paths.iter().zip(&done).collect();
    let parts = par_map(&both, |(path, bc)| {
        let Ok(bc) = bc else { return (Vec::new(), 0) };
        let script = &files[*path];
        let data = std::fs::read(wire.join(format!("{script}.wire"))).expect("the original script");
        let orig: Module = parse_as(&data, Opcodes::Wire).expect("the original script parses");
        let back = module(bc);
        let paired = pair(&orig, &back);
        let mut rows = Vec::new();
        for (i, pa) in orig.protos.iter().enumerate() {
            let row = |a: Shape, b: Shape| Row {
                place: place.to_string(),
                path: (*path).clone(),
                script: script.clone(),
                proto: i,
                name: pa.name.clone(),
                a,
                b,
            };
            let Some(j) = paired[i] else {
                rows.push(row(shape(&orig, pa), Vec::new()));
                continue;
            };
            let (sa, sb) = (shape(&orig, pa), shape(&back, &back.protos[j]));
            if sa != sb {
                rows.push(row(sa, sb));
            }
        }
        (rows, orig.protos.len())
    });
    let functions = parts.iter().map(|p| p.1).sum();
    Place { rows: parts.into_iter().flat_map(|p| p.0).collect(), functions }
}

fn edit_key(a: &[&'static str], b: &[&'static str]) -> String {
    let mut i = 0;
    while i < a.len() && i < b.len() && a[i] == b[i] {
        i += 1;
    }
    let mut j = 0;
    while j < a.len() - i && j < b.len() - i && a[a.len() - 1 - j] == b[b.len() - 1 - j] {
        j += 1;
    }
    let (x, y) = (&a[i..a.len() - j], &b[i..b.len() - j]);
    if x.len() * y.len() > 40000 {
        return "(long)".to_string();
    }
    let mut words = Vec::new();
    for (tag, i1, i2, j1, j2) in Matcher::new(x, y).opcodes() {
        match tag {
            Tag::Delete => words.push(format!("-[{}]", x[i1..i2].join(" "))),
            Tag::Insert => words.push(format!("+[{}]", y[j1..j2].join(" "))),
            Tag::Replace => words.push(format!("[{}]>[{}]", x[i1..i2].join(" "), y[j1..j2].join(" "))),
            Tag::Equal => {}
        }
    }
    words.sort();
    words.dedup();
    words.join(" ")
}

fn short_key(r: &Row) -> String {
    clip(&edit_key(&r.a, &r.b), 96)
}

/// Keys by how often they occur; ties in the order they first appeared.
fn commonest(keys: impl Iterator<Item = String>) -> Vec<(String, usize)> {
    let mut count: HashMap<String, usize> = HashMap::new();
    let mut order = Vec::new();
    for k in keys {
        let n = count.entry(k.clone()).or_insert(0);
        if *n == 0 {
            order.push(k);
        }
        *n += 1;
    }
    let mut all: Vec<(String, usize)> = order.into_iter().map(|k| { let n = count[&k]; (k, n) }).collect();
    all.sort_by_key(|e| std::cmp::Reverse(e.1));
    all
}

fn examples(rows: &[&Row], count: usize, width: usize, depth: usize, output: &Path) {
    let mut sorted = rows.to_vec();
    sorted.sort_by_key(|r| (r.name.is_none(), r.a.len()));
    for r in sorted.iter().take(count) {
        let rel = Path::new(&r.path).strip_prefix(output).unwrap_or(Path::new(&r.path)).display().to_string();
        let name = r.name.as_deref().unwrap_or("None");
        println!("\n--- {rel} script {} proto {} name {name}", r.script, r.proto);
        println!("  orig: {}", r.a[..r.a.len().min(width)].join(" "));
        println!("  back: {}", r.b[..r.b.len().min(width)].join(" "));
        if let Some(wanted) = r.name.as_deref().filter(|n| !n.is_empty()) {
            let text = read_text(&r.path);
            let lines: Vec<&str> = text.lines().collect();
            if let Some(at) = lines.iter().position(|l| l.contains("function") && l.contains(wanted)) {
                let ind = lines[at].len() - lines[at].trim_start_matches(' ').len();
                for l in lines[at..].iter().take(depth) {
                    println!("     {}", clip(l, 100));
                    if l.trim_end() == format!("{}end", " ".repeat(ind)) {
                        break;
                    }
                }
            }
        }
    }
}

fn snapshot(name: &str) -> PathBuf {
    repo().join("out").join(format!("{name}.snap"))
}

fn save(name: &str, rows: &[Row]) {
    let lines: Vec<String> = rows.iter().map(|r| format!("{}\t{}\t{}", r.place, r.script, r.proto)).collect();
    let _ = std::fs::create_dir_all(repo().join("out"));
    std::fs::write(snapshot(name), lines.join("\n") + "\n").expect("the snapshot");
}

fn load(name: &str) -> HashSet<(String, String, usize)> {
    let text = std::fs::read_to_string(snapshot(name)).expect("no such snapshot");
    text.lines()
        .filter_map(|l| {
            let mut p = l.split('\t');
            Some((p.next()?.to_string(), p.next()?.to_string(), p.next()?.parse().ok()?))
        })
        .collect()
}

pub fn main(args: &Args) -> u8 {
    let tools = Tools::find();
    let (dumps, output) = (dumps_root(), output_root());
    let places: Vec<String> = match args.opt("--places") {
        Some(p) => p.split(',').map(String::from).collect(),
        None => dump_places(&dumps).into_iter().filter(|p| output.join(p).is_dir()).collect(),
    };
    let top = args.num("--top", 30);
    let count = args.num("--count", 2);
    let done: Vec<Place> = places.iter().map(|p| compare_place(&tools, &dumps, &output, p)).collect();
    let rows: Vec<Row> = done.into_iter().flat_map(|p| p.rows).collect();
    if let Some(name) = args.opt("--save") {
        save(name, &rows);
    }
    println!("{} functions differ", rows.len());
    if let Some(name) = args.opt("--since") {
        let then = load(name);
        let now: HashSet<_> = rows.iter().map(Row::id).collect();
        let scope: HashSet<_> = then.into_iter().filter(|k| places.contains(&k.0)).collect();
        let worse: Vec<&Row> = rows.iter().filter(|r| !scope.contains(&r.id())).collect();
        println!("{} fixed since {name} and {} newly different", scope.difference(&now).count(), worse.len());
        for (k, n) in commonest(worse.iter().map(|r| short_key(r))).iter().take(12) {
            println!("  worse {n:>4}  {k}");
        }
        let pick = args.opt("--worse").map(|k| clip(k, 96));
        let chosen: Vec<&Row> =
            worse.iter().filter(|r| pick.as_ref().is_none_or(|k| short_key(r) == *k)).copied().collect();
        examples(&chosen, count, 50, 16, &output);
    }
    let keys = par_map(&rows, short_key);
    let hist = commonest(keys.iter().cloned());
    if !args.flag("--since") || top > 0 {
        println!("{} kinds of edit; the commonest:", hist.len());
        for (k, n) in hist.iter().take(top) {
            println!("{n:>5}  {k}");
        }
    }
    if let Some(show) = args.opt("--show") {
        let wanted = clip(show, 96);
        let chosen: Vec<&Row> = rows.iter().zip(&keys).filter(|(_, k)| **k == wanted).map(|(r, _)| r).collect();
        examples(&chosen, count, 60, 26, &output);
    }
    0
}
