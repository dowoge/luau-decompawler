//! How close does the decompiler get on hand-written scripts? Every file in
//! tools/luau_cases is compiled at -O2, decompiled, and judged on:
//!   run     prints the same as the original (and ran without error)
//!   shape   each function recompiles to the same opcode sequence
//!   exact   also the same target registers: the variables came back as written
//! Behaving differently is a bug and fails; differing in form does not.
//!
//! The same goes for any folder of programs: `check` runs tools/semantic too,
//! where a program may end in an error, as long as its decompiled form ends in
//! the same one.

use std::collections::HashMap;
use std::path::Path;

use crate::luau::{clip, decompile_many, module, percent, repo, text_of, Args, Item, Tools, USUAL_BUDGET};
use crate::shape::{exact, pair, shape};

struct Row {
    name: Option<String>,
    why: &'static str,
    a: Vec<String>,
    b: Vec<String>,
}

pub fn main(args: &Args) -> u8 {
    u8::from(run(args, true) > 0)
}

#[derive(Clone, Copy, PartialEq)]
pub enum Print {
    All,
    Bad,
    Nothing,
}

/// A folder of programs to put through the decompiler.
pub struct Suite<'a> {
    pub dir: String,
    pub binary: String,
    pub show: Option<&'a str>,
    pub print: Print,
    /// A program may end in an error, which its decompiled form must repeat.
    pub errors_are_output: bool,
}

/// What a suite found: the programs that do not behave the same, and why.
pub struct Found {
    pub misbehaving: Vec<(String, &'static str)>,
    pub total: usize,
}

fn print_row(print: Print, bad: bool, line: String) {
    if print == Print::All || (bad && print == Print::Bad) {
        println!("{line}");
    }
}

fn base(path: &str) -> String {
    Path::new(path).file_name().map_or(String::new(), |f| f.to_string_lossy().into_owned())
}

/// How many cases misbehave. With `loud` off, only the rows that matter come out.
pub fn run(args: &Args, loud: bool) -> usize {
    let default = repo().join("tools").join("luau_cases").to_string_lossy().into_owned();
    let suite = Suite {
        dir: args.opt("--dir").map_or(default, String::from),
        binary: args.opt("--bin").map_or_else(|| Tools::find().decompiler, String::from),
        show: args.opt("--show"),
        print: if loud { Print::All } else { Print::Bad },
        errors_are_output: args.flag("--errors"),
    };
    run_suite(&suite).misbehaving.len()
}

pub fn run_suite(suite: &Suite) -> Found {
    let tools = Tools::find();
    let (cases, show, binary, print) = (&suite.dir, suite.show, &suite.binary, suite.print);
    let mut srcs: Vec<String> = std::fs::read_dir(cases)
        .expect("the cases folder")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "luau"))
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    srcs.sort();
    let sources: Vec<Item> =
        srcs.iter().map(|s| Item::new(s.clone(), std::fs::read(s).expect("a case to read"))).collect();
    let code = tools.compile(&sources);
    let mut decs: HashMap<String, Item> = HashMap::new();
    let mut crashed: HashMap<String, String> = HashMap::new();
    let compiled: Vec<usize> = (0..srcs.len()).filter(|i| code[*i].is_ok()).collect();
    let bytecode: Vec<Vec<u8>> = compiled.iter().map(|i| code[*i].clone().unwrap_or_default()).collect();
    for (s, c) in srcs.iter().zip(&code) {
        if c.is_err() {
            crashed.insert(s.clone(), "the case itself does not compile".to_string());
        }
    }
    for (i, text) in compiled.iter().zip(decompile_many(binary, &bytecode, 300)) {
        let s = &srcs[*i];
        match text {
            Ok(text) => {
                decs.insert(s.clone(), Item::new(format!("{s}.dec"), text));
            }
            Err(why) => {
                crashed.insert(s.clone(), why);
            }
        }
    }
    let dec_items: Vec<Item> = srcs.iter().filter_map(|s| decs.get(s).cloned()).collect();
    let back: HashMap<String, Vec<u8>> = dec_items
        .iter()
        .zip(tools.compile(&dec_items))
        .filter_map(|(d, b)| Some((d.name.clone(), b.ok()?)))
        .collect();
    let bc: HashMap<&String, &Vec<u8>> = srcs.iter().zip(&code).filter_map(|(s, c)| Some((s, c.as_ref().ok()?))).collect();
    let all: Vec<Item> = sources.iter().chain(&dec_items).cloned().collect();
    let out = tools.outputs(&all, 60, USUAL_BUDGET);
    let mut totals = [0usize; 3];
    let mut misbehaving: Vec<(String, &'static str)> = Vec::new();
    print_row(print, false, format!("{:<24} {:<4} {:>5} {:>6} {:>6}", "case", "run", "funcs", "shape", "exact"));
    for s in &srcs {
        let name = base(s);
        let Some(d) = decs.get(s) else {
            print_row(print, true, format!("{name:<24} crash: {}", crashed[s]));
            misbehaving.push((name, "decompiler error"));
            continue;
        };
        let Some(recompiled) = back.get(&d.name) else {
            print_row(print, true, format!("{name:<24} decompiled text does not compile"));
            misbehaving.push((name, "decompiled text does not compile"));
            continue;
        };
        let ran = out.get(s).is_some_and(|x| suite.errors_are_output || x.ends_with("=== ok"));
        let same = ran && out.get(s) == out.get(&d.name);
        if !same {
            misbehaving.push((name.clone(), if ran { "output differs" } else { "the case itself fails" }));
        }
        let (a, b) = (module(bc[s]), module(recompiled));
        let paired = pair(&a, &b);
        let (mut n, mut shp, mut ex) = (0, 0, 0);
        let mut rows = Vec::new();
        for (i, pa) in a.protos.iter().enumerate() {
            n += 1;
            let Some(j) = paired[i] else {
                rows.push(Row { name: pa.name.clone(), why: "no matching function", a: Vec::new(), b: Vec::new() });
                continue;
            };
            let (sa, sb) = (shape(&a, pa), shape(&b, &b.protos[j]));
            if sa == sb {
                shp += 1;
                let (ea, eb) = (exact(pa), exact(&b.protos[j]));
                if ea == eb {
                    ex += 1;
                } else {
                    rows.push(Row { name: pa.name.clone(), why: "registers", a: ea, b: eb });
                }
            } else {
                let text = |v: Vec<&'static str>| v.into_iter().map(String::from).collect();
                rows.push(Row { name: pa.name.clone(), why: "shape", a: text(sa), b: text(sb) });
            }
        }
        totals[0] += n;
        totals[1] += shp;
        totals[2] += ex;
        let verdict = if same { "ok" } else { "BAD" };
        print_row(print, !same, format!("{name:<24} {verdict:<4} {n:>5} {shp:>6} {ex:>6}"));
        if show.is_some_and(|w| name.contains(w)) {
            if !same {
                let shown = |p: &String| out.get(p).map_or("None", String::as_str).to_string();
                println!("--- original output\n{}\n--- decompiled output\n{}", shown(s), shown(&d.name));
            }
            print_rows(&rows, &text_of(&d.data));
        }
    }
    let [n, shp, ex] = totals;
    let line = format!(
        "{:<24} {:<4} {n:>5} {shp:>6} {ex:>6}   shape {}, exact {}, {} misbehave",
        "total",
        "",
        percent(shp, n),
        percent(ex, n),
        misbehaving.len()
    );
    print_row(print, true, line);
    Found { misbehaving, total: srcs.len() }
}

fn print_rows(rows: &[Row], text: &str) {
    let lines: Vec<&str> = text.lines().collect();
    for row in rows {
        let name = row.name.as_deref().unwrap_or("");
        println!("\n  {}: {}", row.name.as_deref().unwrap_or("None"), row.why);
        let (x, y) = (&row.a, &row.b);
        let shared = x.len().min(y.len());
        let k = (0..shared).find(|i| x[*i] != y[*i]).unwrap_or(shared);
        let around = |v: &[String]| {
            let before = v[k.saturating_sub(3)..k].join(" ");
            let after = v[k..(k + 12).min(v.len())].join(" ");
            format!("{before} | {after}")
        };
        println!("    orig: {}", around(x));
        println!("    back: {}", around(y));
        let names_it = |l: &&str| {
            !name.is_empty()
                && l.contains("function")
                && (l.contains(&format!(".{name}(")) || l.contains(&format!(" {name}(")) || l.contains(&format!(":{name}(")))
        };
        if let Some(at) = lines.iter().position(|l| names_it(l)) {
            let ind = lines[at].len() - lines[at].trim_start_matches(' ').len();
            for l in lines[at..].iter().take(24) {
                println!("      {}", clip(l, 110));
                if l.trim_end() == format!("{}end", " ".repeat(ind)) {
                    break;
                }
            }
        }
    }
}
