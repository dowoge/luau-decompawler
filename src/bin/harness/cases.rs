//! How close does the decompiler get on hand-written scripts? Every file in
//! tools/luau_cases is compiled at -O2, decompiled, and judged on:
//!   run     prints the same as the original (and ran without error)
//!   shape   each function recompiles to the same opcode sequence
//!   exact   also the same target registers: the variables came back as written
//! Behaving differently is a bug and fails; differing in form does not.

use std::collections::HashMap;
use std::path::Path;

use crate::luau::{clip, decompile_many, module, percent, read_text, repo, Args, Temp, Tools, USUAL_BUDGET};
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

fn print_row(loud: bool, always: bool, line: String) {
    if loud || always {
        println!("{line}");
    }
}

fn base(path: &str) -> String {
    Path::new(path).file_name().map_or(String::new(), |f| f.to_string_lossy().into_owned())
}

/// How many cases misbehave. With `loud` off, only the rows that matter come out.
pub fn run(args: &Args, loud: bool) -> usize {
    let tools = Tools::find();
    let default = repo().join("tools").join("luau_cases").to_string_lossy().into_owned();
    let cases = args.opt("--dir").map_or(default, String::from);
    let show = args.opt("--show");
    let binary = args.opt("--bin").map_or(tools.decompiler.clone(), String::from);
    let mut srcs: Vec<String> = std::fs::read_dir(&cases)
        .expect("the cases folder")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "luau"))
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    srcs.sort();
    let tmp = Temp::new("cs");
    let bc = tools.compile(&srcs, &tmp.path("a")).bytecode;
    let mut decs: HashMap<String, String> = HashMap::new();
    let mut crashed: HashMap<String, String> = HashMap::new();
    let compiled: Vec<&String> = srcs.iter().filter(|s| bc.contains_key(*s)).collect();
    let code: Vec<String> = compiled.iter().map(|s| bc[*s].clone()).collect();
    for s in srcs.iter().filter(|s| !bc.contains_key(*s)) {
        crashed.insert(s.clone(), "the case itself does not compile".to_string());
    }
    for (s, text) in compiled.iter().zip(decompile_many(&binary, &code, &tmp.path("d"), 300)) {
        match text {
            Ok(path) => decs.insert((*s).clone(), path),
            Err(why) => crashed.insert((*s).clone(), why),
        };
    }
    let dec_paths: Vec<String> = srcs.iter().filter_map(|s| decs.get(s).cloned()).collect();
    let back = tools.compile(&dec_paths, &tmp.path("b")).bytecode;
    let all: Vec<String> = srcs.iter().chain(&dec_paths).cloned().collect();
    let out = tools.outputs(&all, 60, USUAL_BUDGET);
    let mut totals = [0usize; 3];
    let mut bad_run = 0;
    print_row(loud, false, format!("{:<24} {:<4} {:>5} {:>6} {:>6}", "case", "run", "funcs", "shape", "exact"));
    for s in &srcs {
        let name = base(s);
        let Some(d) = decs.get(s) else {
            print_row(loud, true, format!("{name:<24} crash: {}", crashed[s]));
            bad_run += 1;
            continue;
        };
        let Some(recompiled) = back.get(d) else {
            print_row(loud, true, format!("{name:<24} decompiled text does not compile"));
            bad_run += 1;
            continue;
        };
        let same = out.get(s) == out.get(d) && out.get(s).is_some_and(|x| x.ends_with("=== ok"));
        bad_run += usize::from(!same);
        let (a, b) = (module(&bc[s]), module(recompiled));
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
        print_row(loud, !same, format!("{name:<24} {verdict:<4} {n:>5} {shp:>6} {ex:>6}"));
        if show.is_some_and(|w| name.contains(w)) {
            if !same {
                let shown = |p: &String| out.get(p).map_or("None", String::as_str).to_string();
                println!("--- original output\n{}\n--- decompiled output\n{}", shown(s), shown(d));
            }
            print_rows(&rows, &read_text(d));
        }
    }
    let [n, shp, ex] = totals;
    let line = format!(
        "{:<24} {:<4} {n:>5} {shp:>6} {ex:>6}   shape {}, exact {}, {bad_run} misbehave",
        "total",
        "",
        percent(shp, n),
        percent(ex, n)
    );
    print_row(loud, true, line);
    bad_run
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
