//! Shrinks a program the decompiler gets wrong: removes lines and whole blocks
//! while the program stays valid and the failure stays the same kind. "Wrong"
//! means the decompiled program does not compile or prints something else; with
//! --shape, a function that does not recompile to the original sequence.
//!
//! Works from a private copy of the decompiler, so rebuilding does not disturb
//! a run, and stops after --seconds with the smallest program so far.

use std::time::{Duration, Instant};

use crate::luau::{command, decompile, module, par_map, read_text, run_limited, text_of, workers, Args, Temp, Tools};
use crate::shape::shape;

type Lines = Vec<String>;

struct Probe<'a> {
    tools: &'a Tools,
    binary: String,
    tmp: &'a Temp,
    shape: bool,
}

impl Probe<'_> {
    fn compiled(&self, path: &str, out: &str) -> Option<String> {
        self.tools.emit(&[path.to_string()], out, 1).get(path).cloned()
    }

    /// What goes wrong with this program, None when nothing does.
    fn kind_of(&self, lines: &[String], name: &str) -> Option<&'static str> {
        let dir = self.tmp.path(name);
        std::fs::create_dir_all(&dir).ok()?;
        let at = |f: &str| format!("{dir}/{f}");
        let src = at("s.luau");
        std::fs::write(&src, lines.join("\n") + "\n").ok()?;
        let bc = self.compiled(&src, &at("a"))?;
        let Ok(text) = decompile(&self.binary, &bc) else { return Some("crash") };
        let dec = at("d.luau");
        std::fs::write(&dec, text).ok()?;
        let Some(bd) = self.compiled(&dec, &at("b")) else { return Some("no-compile") };
        if self.shape {
            let (a, b) = (module(&bc), module(&bd));
            let differs = |(i, p): (usize, &crate::bytecode::Proto)| {
                i >= b.protos.len() || shape(&a, p) != shape(&b, &b.protos[i])
            };
            return a.protos.iter().enumerate().any(differs).then_some("shape");
        }
        // the VM budget counts calls and loops: one huge string.rep is neither
        let run = run_limited(command(&self.tools.run).arg(&src).arg(&dec), 10)?;
        let out = crate::luau::parse_outputs(&text_of(&run.stdout));
        let (x, y) = (out.get(&src), out.get(&dec));
        // a program that now fails on its own is a different problem: without
        // this the search drifts to undefined names and error message wording
        if !x?.ends_with("=== ok") {
            return None;
        }
        (x != y).then_some("differs")
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

fn replaced(lines: &[String], i: usize, new: String) -> Lines {
    let mut out = lines.to_vec();
    out[i] = new;
    out
}

fn expression_cuts(lines: &[String], out: &mut Vec<Lines>) {
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
                    out.push(replaced(lines, i, new));
                }
            }
        }
    }
}

fn without(lines: &[String], from: usize, to: usize) -> Lines {
    lines[..from].iter().chain(&lines[to..]).cloned().collect()
}

/// Smaller programs, biggest cuts first.
fn candidates(lines: &[String]) -> Vec<Lines> {
    let n = lines.len();
    let mut out = Vec::new();
    let mut size = n / 2;
    while size >= 2 {
        for i in (0..=n - size).step_by((size / 2).max(1)) {
            out.push(without(lines, i, i + size));
        }
        size /= 2;
    }
    let ends: Vec<usize> = (0..n).map(|i| block_end(lines, i)).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|i| std::cmp::Reverse(ends[*i] - *i));
    for i in order {
        if ends[i] - i > 1 {
            out.push(without(lines, i, ends[i]));
        }
    }
    for i in 0..n {
        out.push(without(lines, i, i + 1));
    }
    // keep a block's body, drop its header and end
    for i in 0..n {
        let e = ends[i];
        if e - i > 2 {
            let ind = indent(&lines[i]);
            let body = lines[i + 1..e - 1].iter().map(|l| {
                if indent(l) > ind {
                    l.chars().skip(4).collect()
                } else {
                    l.clone()
                }
            });
            out.push(lines[..i].iter().cloned().chain(body).chain(lines[e..].iter().cloned()).collect());
        }
    }
    expression_cuts(lines, &mut out);
    out
}

pub fn main(args: &Args) -> u8 {
    let Some(path) = args.0.first() else {
        eprintln!("harness reduce FILE [--shape] [--out FILE] [--seconds N]");
        return 2;
    };
    let tools = Tools::find();
    let tmp = Temp::new("red");
    let binary = tmp.path(&format!("decompiler{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(args.opt("--bin").unwrap_or(&tools.decompiler), &binary).expect("a decompiler to copy");
    let probe = Probe { tools: &tools, binary, tmp: &tmp, shape: args.flag("--shape") };
    let mut lines: Lines = read_text(path).lines().map(String::from).collect();
    let stop = Instant::now() + Duration::from_secs_f64(args.num("--seconds", 600.0));
    let Some(want) = probe.kind_of(&lines, "start") else {
        println!("nothing wrong with this program");
        return 1;
    };
    let width = workers();
    let mut serial = 0;
    let mut progress = true;
    while progress && Instant::now() < stop {
        progress = false;
        let cands = candidates(&lines);
        for batch in cands.chunks(width) {
            if Instant::now() >= stop {
                break;
            }
            let named: Vec<(&Lines, String)> =
                batch.iter().enumerate().map(|(i, c)| (c, format!("c{}", serial + i))).collect();
            serial += batch.len();
            let got = par_map(&named, |(c, name)| probe.kind_of(c, name));
            for (_, name) in &named {
                let _ = std::fs::remove_dir_all(tmp.path(name));
            }
            if let Some(hit) = named.iter().zip(&got).find(|(_, g)| **g == Some(want)) {
                lines = hit.0 .0.clone();
                progress = true;
                break;
            }
        }
    }
    let text = lines.join("\n") + "\n";
    println!("-- {want}, {} lines", lines.len());
    print!("{text}");
    if let Some(target) = args.opt("--out") {
        std::fs::write(target, text).expect("the reduced program");
    }
    0
}
