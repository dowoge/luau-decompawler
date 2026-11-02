//! The ground truth: random programs are compiled at -O2 like Roblox does,
//! decompiled, and run side by side with the original. Their output must be
//! identical. Failing cases (source, decompiled text, both outputs) are kept.

use std::collections::HashMap;
use std::path::Path;

use crate::gen;
use crate::luau::{decompile_many, module, par_map, percent, Args, Item, Tools, USUAL_BUDGET};
use crate::shape::{pair, shape};

const DECOMPILE_LIMIT: u64 = 300;
const RUN_LIMIT: u64 = 60;

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

fn has_flag(text: &str) -> bool {
    text.match_indices("local jump_").any(|(at, m)| {
        let rest = &text[at + m.len()..];
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        digits > 0 && rest[digits..].starts_with(" = false")
    })
}

pub fn run(tools: &Tools, o: &Options) -> Outcome {
    let seeds: Vec<u64> = (o.seed..o.seed + o.count).collect();
    let srcs: Vec<Item> = par_map(&seeds, |seed| Item::new(format!("g{seed}.luau"), gen::program(*seed, o.size)));
    let mut detail = Vec::new();
    let code = tools.compile(&srcs);
    let invalid: Vec<(&str, &String)> =
        srcs.iter().zip(&code).filter_map(|(s, c)| Some((s.name.as_str(), c.as_ref().err()?))).take(3).collect();
    if !invalid.is_empty() {
        detail.push(format!("generator wrote invalid programs: {invalid:?}"));
    }
    let compiled: Vec<usize> = (0..srcs.len()).filter(|i| code[*i].is_ok()).collect();
    let bytecode: Vec<Vec<u8>> = compiled.iter().map(|i| code[*i].clone().unwrap_or_default()).collect();
    let done = decompile_many(&o.binary, &bytecode, DECOMPILE_LIMIT);

    // (source, its decompiled program)
    let mut decs: Vec<(usize, Item)> = Vec::new();
    let mut crashed: Vec<(usize, String)> = Vec::new();
    for (i, text) in compiled.iter().zip(done) {
        match text {
            Ok(text) => decs.push((*i, Item::new(format!("g{}.dec.luau", seeds[*i]), text))),
            Err(why) => crashed.push((*i, why)),
        }
    }
    let dec_items: Vec<Item> = decs.iter().map(|d| d.1.clone()).collect();
    let back = tools.compile(&dec_items);
    let compile_fail = back.iter().filter(|b| b.is_err()).count();

    let all: Vec<Item> = srcs.iter().chain(&dec_items).cloned().collect();
    let out = tools.outputs(&all, RUN_LIMIT, USUAL_BUDGET);
    let (mut same, mut differ, mut skipped) = (0, 0, 0);
    // (source, decompiled, why)
    let mut failures: Vec<(usize, usize, &str)> = Vec::new();
    for (k, (i, d)) in decs.iter().enumerate() {
        if back[k].is_err() {
            failures.push((*i, k, "decompiled program does not compile"));
            continue;
        }
        let (a, b) = (out.get(&srcs[*i].name), out.get(&d.name));
        if a.is_some_and(|x| x.ends_with("budget exceeded")) {
            skipped += 1; // the original itself does not finish
            continue;
        }
        if a == b {
            same += 1;
        } else {
            differ += 1;
            failures.push((*i, k, "output differs"));
        }
    }

    // how many functions recompile to the original instruction sequence
    let pairs: Vec<(usize, usize)> = decs.iter().enumerate().map(|(k, d)| (d.0, k)).collect();
    let counted = par_map(&pairs, |(i, k)| {
        let (Ok(orig), Ok(back)) = (&code[*i], &back[*k]) else { return (0, 0) };
        let (a, b) = (module(orig), module(back));
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
         {compile_fail} decompiled programs do not compile, {} decompiler crashes",
        o.count,
        crashed.len()
    );
    detail.push(format!("{funcs} functions: {ident} recompile instruction-for-instruction ({})", percent(ident, funcs)));
    let texts: Vec<String> = dec_items.iter().map(|d| String::from_utf8_lossy(&d.data).into_owned()).collect();
    let flags = texts.iter().filter(|t| has_flag(t)).count();
    let blocks = texts.iter().filter(|t| t.contains("local block = 0")).count();
    detail.push(format!("fallbacks: {flags} programs with jump flags, {blocks} with a dispatcher"));
    for (i, e) in crashed.iter().take(5) {
        detail.push(format!("  crash: {} {e}", srcs[*i].name));
    }
    if let Some(keep) = o.keep.as_ref().filter(|_| !failures.is_empty() || !crashed.is_empty()) {
        std::fs::create_dir_all(keep).expect("a folder for failures");
        for (i, k, why) in &failures {
            keep_failure(keep, &srcs[*i], Some(&dec_items[*k]), why, &out);
        }
        for (i, e) in &crashed {
            keep_failure(keep, &srcs[*i], None, &format!("decompiler crash: {e}"), &out);
        }
        detail.push(format!("  failing cases written to {keep}"));
    }
    for (i, _, why) in failures.iter().take(8) {
        detail.push(format!("  {}: {why}", srcs[*i].name));
    }
    Outcome { clean: failures.is_empty() && crashed.is_empty(), summary, detail }
}

fn keep_failure(keep: &str, src: &Item, dec: Option<&Item>, why: &str, out: &HashMap<String, String>) {
    let stem = src.name.strip_suffix(".luau").unwrap_or(&src.name);
    let at = Path::new(keep).join(stem);
    let at = at.to_string_lossy();
    let _ = std::fs::write(format!("{at}.src.luau"), &src.data);
    let mut note = format!("{why}\n");
    if let Some(dec) = dec {
        let _ = std::fs::write(format!("{at}.dec.luau"), &dec.data);
        let shown = |name: &String| out.get(name).map_or("None", String::as_str).to_string();
        note += &format!("--- original output\n{}\n--- decompiled output\n{}\n", shown(&src.name), shown(&dec.name));
    }
    let _ = std::fs::write(format!("{at}.why.txt"), note);
}
