//! Shrinks a program the decompiler gets wrong: removes lines and whole blocks
//! while the program stays valid and the failure stays the same kind. "Wrong"
//! means the decompiled program does not compile or prints something else; with
//! --shape, a function that does not recompile to the original sequence.
//!
//! Works from a private copy of the decompiler, so rebuilding does not disturb
//! a run, and stops after --seconds with the smallest program so far.
//!
//! Starting processes is what takes the time here, not the work they do, so
//! candidates go through in batches, one process per step. After a cut the
//! search carries on from where it was: what failed before it mostly fails
//! still, and a last pass over everything catches what does not.

use std::time::{Duration, Instant};

use crate::luau::{decompile_many, module, par_map, read_text, workers, Args, Temp, Tools};
use crate::shape::shape;

type Lines = Vec<String>;

/// Files per compiler process within a batch.
const SPLIT: usize = 8;
/// Seconds before a batch counts as hung.
const DECOMPILE_LIMIT: u64 = 30;
const RUN_LIMIT: u64 = 10;
/// A cut that breaks a loop's end runs until the VM stops it, and these programs
/// need nothing near the usual budget: this one stops such a cut in a tenth of
/// the time. A program that does need more gets the usual one.
const SMALL_BUDGET: u64 = 2_000_000;

struct Probe<'a> {
    tools: &'a Tools,
    binary: String,
    shape: bool,
    budget: u64,
}

impl Probe<'_> {
    /// What goes wrong with each program, None when nothing does.
    fn kinds(&self, programs: &[Lines]) -> Vec<Option<&'static str>> {
        let dir = Temp::new("rd");
        let sources: Vec<String> = programs
            .iter()
            .enumerate()
            .map(|(i, lines)| {
                let path = dir.path(&format!("{i}.luau"));
                std::fs::write(&path, lines.join("\n") + "\n").expect("a candidate");
                path
            })
            .collect();
        let code = self.tools.compile(&sources, &dir.path("a"), SPLIT).bytecode;
        let compiled: Vec<usize> = (0..programs.len()).filter(|i| code.contains_key(&sources[*i])).collect();
        let files: Vec<String> = compiled.iter().map(|i| code[&sources[*i]].clone()).collect();
        let texts = decompile_many(&self.binary, &files, &dir.path("d"), DECOMPILE_LIMIT);

        let mut kinds = vec![None; programs.len()];
        let mut decompiled: Vec<(usize, String)> = Vec::new();
        for (i, text) in compiled.iter().zip(texts) {
            match text {
                Ok(path) => decompiled.push((*i, path)),
                Err(_) => kinds[*i] = Some("crash"),
            }
        }
        let paths: Vec<String> = decompiled.iter().map(|d| d.1.clone()).collect();
        let back = self.tools.compile(&paths, &dir.path("b"), SPLIT).bytecode;
        let mut good: Vec<(usize, String)> = Vec::with_capacity(decompiled.len());
        for (i, path) in decompiled {
            if back.contains_key(&path) {
                good.push((i, path));
            } else {
                kinds[i] = Some("no-compile");
            }
        }
        if self.shape {
            let verdicts = par_map(&good, |(i, path)| {
                let (a, b) = (module(&code[&sources[*i]]), module(&back[path]));
                let differs = |(k, p): (usize, &crate::bytecode::Proto)| {
                    k >= b.protos.len() || shape(&a, p) != shape(&b, &b.protos[k])
                };
                a.protos.iter().enumerate().any(differs)
            });
            for ((i, _), differs) in good.iter().zip(verdicts) {
                kinds[*i] = differs.then_some("shape");
            }
            return kinds;
        }
        // the time limit is for what the budget misses: it counts calls and
        // loops, and one huge string.rep is neither
        let all: Vec<String> = good.iter().flat_map(|(i, p)| [sources[*i].clone(), p.clone()]).collect();
        let out = self.tools.outputs(&all, RUN_LIMIT, self.budget);
        for (i, path) in &good {
            let (x, y) = (out.get(&sources[*i]), out.get(path));
            // a program that now fails on its own is a different problem: without
            // this the search drifts to undefined names and error message wording
            if x.is_some_and(|x| x.ends_with("=== ok")) && x != y {
                kinds[*i] = Some("differs");
            }
        }
        kinds
    }
}

/// One way to make a program smaller.
enum Cut {
    Remove(usize, usize),
    /// Keep a block's body, drop its header and end.
    Unwrap(usize, usize),
    Line(usize, String),
}

fn apply(lines: &[String], cut: &Cut) -> Lines {
    match cut {
        Cut::Remove(from, to) => lines[..*from].iter().chain(&lines[*to..]).cloned().collect(),
        Cut::Unwrap(at, end) => {
            let ind = indent(&lines[*at]);
            let body = lines[at + 1..end - 1].iter().map(|l| {
                if indent(l) > ind {
                    l.chars().skip(4).collect()
                } else {
                    l.clone()
                }
            });
            lines[..*at].iter().cloned().chain(body).chain(lines[*end..].iter().cloned()).collect()
        }
        Cut::Line(at, new) => {
            let mut out = lines.to_vec();
            out[*at] = new.clone();
            out
        }
    }
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Does the line have this word, as a word?
fn has_word(line: &str, word: &str) -> bool {
    line.match_indices(word).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + word.len()..].chars().next();
        !before.is_some_and(word_char) && !after.is_some_and(word_char)
    })
}

fn starts_word(line: &str, word: &str) -> bool {
    line.strip_prefix(word).is_some_and(|rest| !rest.chars().next().is_some_and(word_char))
}

/// Line after the block opened at i, or i + 1 when it opens none.
fn block_end(lines: &[String], i: usize) -> usize {
    if !["function", "if", "for", "while", "repeat", "do"].iter().any(|w| has_word(&lines[i], w)) {
        return i + 1;
    }
    let ind = indent(&lines[i]);
    for j in i + 1..lines.len() {
        let s = lines[j].trim();
        if !s.is_empty() && indent(&lines[j]) <= ind {
            if starts_word(s, "end") || starts_word(s, "until") {
                return j + 1;
            }
            if !starts_word(s, "else") && !starts_word(s, "elseif") {
                return j;
            }
        }
    }
    i + 1
}

/// (start, end) of every parenthesized group, outermost first.
fn parens(line: &[char]) -> Vec<(usize, usize)> {
    let (mut stack, mut found, mut quote) = (Vec::new(), Vec::new(), None);
    for (i, c) in line.iter().enumerate() {
        match quote {
            Some(q) => {
                if *c == q && line[i - 1] != '\\' {
                    quote = None;
                }
            }
            None if matches!(c, '"' | '\'' | '`') => quote = Some(*c),
            None if *c == '(' => stack.push(i),
            None if *c == ')' => {
                if let Some(s) = stack.pop() {
                    found.push((s, i));
                }
            }
            None => {}
        }
    }
    found.sort_by_key(|(s, e)| std::cmp::Reverse(e - s));
    found
}

fn expression_cuts(lines: &[String]) -> Vec<Cut> {
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let chars: Vec<char> = l.chars().collect();
        if chars.len() < 30 {
            continue;
        }
        let text = |r: &[char]| r.iter().collect::<String>();
        for (s, e) in parens(&chars) {
            let inner = text(&chars[s + 1..e]);
            for what in ["1", "true", "\"a\"", inner.as_str()] {
                // a call keeps its parentheses, its arguments get simpler
                let call = s > 0 && (chars[s - 1].is_alphanumeric() || chars[s - 1] == '_');
                let new = if call {
                    format!("{}{what}{}", text(&chars[..=s]), text(&chars[e..]))
                } else {
                    format!("{}{what}{}", text(&chars[..s]), text(&chars[e + 1..]))
                };
                // strictly shorter: swapping one literal for another would never end
                if new.chars().count() < chars.len() {
                    out.push(Cut::Line(i, new));
                }
            }
        }
    }
    out
}

const STAGES: usize = 5;

/// The cuts of one stage, biggest first. Later stages are worth building only
/// once the earlier ones have nothing left.
fn plan(lines: &[String], stage: usize) -> Vec<Cut> {
    let n = lines.len();
    let ends = || (0..n).map(|i| block_end(lines, i)).collect::<Vec<usize>>();
    match stage {
        0 => {
            let mut out = Vec::new();
            let mut size = n / 2;
            while size >= 2 {
                for i in (0..=n - size).step_by((size / 2).max(1)) {
                    out.push(Cut::Remove(i, i + size));
                }
                size /= 2;
            }
            out
        }
        1 => {
            let ends = ends();
            let mut order: Vec<usize> = (0..n).filter(|i| ends[*i] - *i > 1).collect();
            order.sort_by_key(|i| std::cmp::Reverse(ends[*i] - *i));
            order.into_iter().map(|i| Cut::Remove(i, ends[i])).collect()
        }
        2 => (0..n).map(|i| Cut::Remove(i, i + 1)).collect(),
        3 => {
            let ends = ends();
            (0..n).filter(|i| ends[*i] - *i > 2).map(|i| Cut::Unwrap(i, ends[i])).collect()
        }
        _ => expression_cuts(lines),
    }
}

pub fn main(args: &Args) -> u8 {
    let Some(path) = args.0.first() else {
        eprintln!("harness reduce FILE [--shape] [--out FILE] [--seconds N] [--bin PATH]");
        return 2;
    };
    let tools = Tools::find();
    let tmp = Temp::new("red");
    let binary = tmp.path(&format!("decompiler{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(args.opt("--bin").unwrap_or(&tools.decompiler), &binary).expect("a decompiler to copy");
    let mut probe = Probe { tools: &tools, binary, shape: args.flag("--shape"), budget: SMALL_BUDGET };
    let mut lines: Lines = read_text(path).lines().map(String::from).collect();
    let stop = Instant::now() + Duration::from_secs_f64(args.num("--seconds", 600.0));
    let mut want = probe.kinds(std::slice::from_ref(&lines))[0];
    if want.is_none() {
        probe.budget = crate::luau::USUAL_BUDGET;
        want = probe.kinds(std::slice::from_ref(&lines))[0];
    }
    let Some(want) = want else {
        println!("nothing wrong with this program");
        return 1;
    };
    let width = workers();
    let mut tried = 0;
    let mut cuts = 0;
    let mut changed = true;
    while changed && Instant::now() < stop {
        changed = false;
        for stage in 0..STAGES {
            let mut at = 0;
            let mut steps = plan(&lines, stage);
            while at < steps.len() && Instant::now() < stop {
                let batch = &steps[at..(at + width).min(steps.len())];
                let programs: Vec<Lines> = batch.iter().map(|c| apply(&lines, c)).collect();
                tried += programs.len();
                let got = probe.kinds(&programs);
                match got.iter().position(|g| *g == Some(want)) {
                    Some(hit) => {
                        lines = programs.into_iter().nth(hit).unwrap_or_default();
                        at += hit;
                        cuts += 1;
                        changed = true;
                        steps = plan(&lines, stage);
                    }
                    None => at += batch.len(),
                }
            }
        }
    }
    let text = lines.join("\n") + "\n";
    println!("-- {want}, {} lines, {cuts} cuts of {tried} tried", lines.len());
    print!("{text}");
    if let Some(target) = args.opt("--out") {
        std::fs::write(target, text).expect("the reduced program");
    }
    0
}
