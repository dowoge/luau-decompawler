//! The ground truth: random programs are compiled at -O2 like Roblox does,
//! decompiled, and run side by side with the original. Their output must be
//! identical. Failing cases (source, decompiled text, both outputs) are kept.

use std::collections::HashMap;
use std::path::Path;

use crate::gen;
use crate::luau::{decompile, module, par_map, percent, read_text, Args, Temp, Tools, BATCH};
use crate::shape::{pair, shape};

pub struct Options {
    pub count: u64,
    pub seed: u64,
    pub size: usize,
    pub keep: Option<String>,
    pub binary: String,
}

pub struct Outcome {
    pub clean: bool,
    pub summary: String,
    pub detail: Vec<String>,
}

pub fn main(args: &Args) -> u8 {
    let tools = Tools::find();
    let options = Options {
        count: args.num("--count", 200),
        seed: args.num("--seed", 1),
        size: args.num("--size", 25),
        keep: args.opt("--keep").map(String::from),
        binary: args.opt("--bin").map_or(tools.decompiler.clone(), String::from),
    };
    println!("decompiler: {}", options.binary);
    let out = run(&tools, &options);
    println!("{}", out.summary);
    for line in &out.detail {
        println!("{line}");
    }
    if out.clean {
        0
    } else {
        1
    }
}

fn base(path: &str) -> &str {
    Path::new(path).file_name().and_then(|f| f.to_str()).unwrap_or(path)
}

fn stem(path: &str) -> &str {
    base(path).strip_suffix(".luau").unwrap_or(base(path))
}

fn has_flag(text: &str) -> bool {
    text.match_indices("local jump_").any(|(at, m)| {
        let rest = &text[at + m.len()..];
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        digits > 0 && rest[digits..].starts_with(" = false")
    })
}

pub fn run(tools: &Tools, o: &Options) -> Outcome {
    let tmp = Temp::new("dt");
    let seeds: Vec<u64> = (o.seed..o.seed + o.count).collect();
    let srcs: Vec<String> = par_map(&seeds, |seed| {
        let path = tmp.path(&format!("g{seed}.luau"));
        std::fs::write(&path, gen::program(*seed, o.size)).expect("a generated program");
        path
    });
    let mut detail = Vec::new();
    let bad_src = tools.errors(&srcs);
    if !bad_src.is_empty() {
        detail.push(format!("generator wrote invalid programs: {:?}", &bad_src[..bad_src.len().min(3)]));
    }
    let mapping = tools.emit(&srcs, &tmp.path("bc"), BATCH);
    let compiled: Vec<&String> = srcs.iter().filter(|s| mapping.contains_key(*s)).collect();
    let done = par_map(&compiled, |s| decompile(&o.binary, &mapping[*s]));

    let mut decs: Vec<(String, String)> = Vec::new();
    let mut crashed: Vec<(String, String)> = Vec::new();
    for (s, text) in compiled.iter().zip(done) {
        match text {
            Ok(text) => {
                let d = format!("{}.dec.luau", s.strip_suffix(".luau").unwrap_or(s));
                std::fs::write(&d, text).expect("decompiled text");
                decs.push(((*s).clone(), d));
            }
            Err(why) => crashed.push(((*s).clone(), why)),
        }
    }
    let dec_paths: Vec<String> = decs.iter().map(|d| d.1.clone()).collect();
    let compile_fail: Vec<String> =
        tools.errors(&dec_paths).iter().filter_map(|l| l.split_once(": ").map(|p| p.0.to_string())).collect();

    let all: Vec<String> = srcs.iter().chain(&dec_paths).cloned().collect();
    let out = tools.outputs(&all);
    let (mut same, mut differ, mut skipped) = (0, 0, 0);
    let mut failures: Vec<(String, String, &str)> = Vec::new();
    for (s, d) in &decs {
        if compile_fail.contains(d) {
            failures.push((s.clone(), d.clone(), "decompiled program does not compile"));
            continue;
        }
        let (a, b) = (out.get(s), out.get(d));
        if a.is_some_and(|x| x.ends_with("budget exceeded")) {
            skipped += 1; // the original itself does not finish
            continue;
        }
        if a == b {
            same += 1;
        } else {
            differ += 1;
            failures.push((s.clone(), d.clone(), "output differs"));
        }
    }

    // how many functions recompile to the original instruction sequence
    let dmap = tools.emit(&dec_paths, &tmp.path("dbc"), BATCH);
    let counted = par_map(&decs, |(s, d)| {
        let Some(back) = dmap.get(d) else { return (0, 0) };
        let (a, b) = (module(&mapping[s]), module(back));
        let paired = pair(&a, &b);
        let same = |(i, p): (usize, &crate::bytecode::Proto)| {
            paired[i].is_some_and(|j| shape(&a, p) == shape(&b, &b.protos[j]))
        };
        (a.protos.len(), a.protos.iter().enumerate().filter(|x| same(*x)).count())
    });
    let funcs: usize = counted.iter().map(|c| c.0).sum();
    let ident: usize = counted.iter().map(|c| c.1).sum();

    let summary = format!(
        "{} programs ({skipped} skipped: never finish): {same} behave identically, {differ} differ, \
         {} decompiled programs do not compile, {} decompiler crashes",
        o.count,
        compile_fail.len(),
        crashed.len()
    );
    detail.push(format!("{funcs} functions: {ident} recompile instruction-for-instruction ({})", percent(ident, funcs)));
    let texts: Vec<String> = dec_paths.iter().map(|d| read_text(d)).collect();
    let flags = texts.iter().filter(|t| has_flag(t)).count();
    let blocks = texts.iter().filter(|t| t.contains("local block = 0")).count();
    detail.push(format!("fallbacks: {flags} programs with jump flags, {blocks} with a dispatcher"));
    for (s, e) in crashed.iter().take(5) {
        detail.push(format!("  crash: {} {e}", base(s)));
    }
    if let Some(keep) = o.keep.as_ref().filter(|_| !failures.is_empty() || !crashed.is_empty()) {
        std::fs::create_dir_all(keep).expect("a folder for failures");
        for (s, d, why) in &failures {
            keep_failure(keep, s, Some(d), why, &out);
        }
        for (s, e) in &crashed {
            keep_failure(keep, s, None, &format!("decompiler crash: {e}"), &out);
        }
        detail.push(format!("  failing cases written to {keep}"));
    }
    for (s, _, why) in failures.iter().take(8) {
        detail.push(format!("  {}: {why}", base(s)));
    }
    Outcome { clean: failures.is_empty() && crashed.is_empty(), summary, detail }
}

fn keep_failure(keep: &str, s: &str, d: Option<&String>, why: &str, out: &HashMap<String, String>) {
    let at = Path::new(keep).join(stem(s));
    let at = at.to_string_lossy();
    let _ = std::fs::copy(s, format!("{at}.src.luau"));
    let mut note = format!("{why}\n");
    if let Some(d) = d {
        let _ = std::fs::copy(d, format!("{at}.dec.luau"));
        let shown = |p: &String| out.get(p).map_or("None", String::as_str).to_string();
        note += &format!("--- original output\n{}\n--- decompiled output\n{}\n", shown(&s.to_string()), shown(d));
    }
    let _ = std::fs::write(format!("{at}.why.txt"), note);
}
