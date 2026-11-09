//! harness assist [--seed S] [--rounds N]: `--assist` with the harness for its
//! model. The names it proposes are partly fine and partly wrong on purpose (a
//! global the script reads, another variable's name, a field's, a keyword, a
//! line that declares nothing, two variables given one name), and the real
//! decompiler applies and checks them over the semantic suites' programs.
//! Whatever it keeps must leave each program compiling to the same bytecode
//! and printing the same as before: a rename that changed either would be one
//! the check let through.
//!
//! The harness plays the model when $LUAU_FAKE_MODEL is set (its seed): it
//! reads a request on standard input and answers it.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::luau::{command, decompile_many, module, read_text, repo, run_limited, text_of, Args, Item, Temp, Tools};
use crate::rng::Rng;

pub fn main(args: &Args) -> u8 {
    let tools = Tools::find();
    u8::from(!run(&tools, args.num("--seed", 1u64), args.num("--rounds", 2usize)))
}

/// The suites' programs, decompiled, renamed by the fake model through
/// `--assist-text`, compiled and run before and after.
pub fn run(tools: &Tools, seed: u64, rounds: usize) -> bool {
    let mut dirs = vec![repo().join("tools").join("semantic")];
    if let Ok(more) = std::env::var("LUAU_SUITES") {
        dirs.extend(more.split(';').filter(|d| !d.is_empty()).map(PathBuf::from));
    }
    let mut programs: Vec<Item> = Vec::new();
    for dir in &dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        let mut files: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "luau")).collect();
        files.sort();
        for f in files {
            let name = format!("{}_{}", programs.len(), f.file_stem().unwrap_or_default().to_string_lossy());
            programs.push(Item::new(name, read_text(&f.to_string_lossy())));
        }
    }
    let compiled = tools.compile(&programs);
    let mut texts: Vec<(String, String)> = Vec::new();
    let bytecode: Vec<Vec<u8>> = compiled.iter().filter_map(|c| c.as_ref().ok().cloned()).collect();
    let names: Vec<&String> = programs.iter().zip(&compiled).filter(|(_, c)| c.is_ok()).map(|(p, _)| &p.name).collect();
    for (name, text) in names.into_iter().zip(decompile_many(&tools.decompiler, &bytecode, 120)) {
        if let Ok(t) = text {
            texts.push((name.clone(), t));
        }
    }
    let input = Temp::new("assist_in");
    for (name, text) in &texts {
        let _ = std::fs::write(input.path(&format!("{name}.lua")), text);
    }
    let me = std::env::current_exe().expect("the harness's own path");
    let mut good = true;
    let (mut kept_all, mut proposed_all) = (0usize, 0usize);
    let (mut edits_kept_all, mut edits_all) = (0usize, 0usize);
    let (mut notes_kept_all, mut notes_all) = (0usize, 0usize);
    for round in 0..rounds {
        let output = Temp::new("assist_out");
        let mut cmd = command(&tools.decompiler);
        cmd.arg("--assist-text").arg(&input.0).arg("--out").arg(&output.0);
        cmd.arg("--assist-claude").arg(&me).arg("--luau-check").arg(&tools.check);
        cmd.arg("--assist-batch").arg("4").arg("--assist-jobs").arg("4").arg("--assist-comments").arg("--assist-edits");
        cmd.env("LUAU_FAKE_MODEL", (seed + round as u64).to_string());
        let ran = run_limited(&mut cmd, None, 600);
        let said = text_of(&ran.stderr);
        let summary = said.lines().find(|l| l.contains(" requests (") && l.contains(" renames kept")).unwrap_or("").to_string();
        let (kept, proposed) = counts(&summary, " renames kept");
        let (edits_kept, edits) = counts(&summary, " edits kept");
        let (notes_kept, notes) = counts(&summary, " comments kept");
        notes_kept_all += notes_kept;
        notes_all += notes;
        kept_all += kept;
        proposed_all += proposed;
        edits_kept_all += edits_kept;
        edits_all += edits;
        // every file, before and after: the same program (line numbers aside:
        // an edit may move lines), the same output (line numbers in error
        // messages aside)
        let before: Vec<Item> = texts.iter().map(|(n, t)| Item::new(n.clone(), t.as_str())).collect();
        let after: Vec<Item> =
            texts.iter().map(|(n, t)| Item::new(n.clone(), read_or(&output.0.join(format!("{n}.lua")), t))).collect();
        let shape = |c: &Result<Vec<u8>, String>| c.as_ref().ok().map(|b| crate::bytecode::skeleton(&module(b)));
        let (b1, b2) = (tools.compile(&before), tools.compile(&after));
        let mut changed_code = Vec::new();
        for (i, (x, y)) in b1.iter().zip(&b2).enumerate() {
            if x.is_ok() && shape(x) != shape(y) {
                changed_code.push(texts[i].0.clone());
            }
        }
        let (o1, o2) = (tools.outputs(&before, 10, 1 << 26), tools.outputs(&after, 10, 1 << 26));
        let mut changed_output = Vec::new();
        for (n, _) in &texts {
            if o1.get(n).map(|o| no_lines(o)) != o2.get(n).map(|o| no_lines(o)) {
                changed_output.push(n.clone());
            }
        }
        let renamed = texts.iter().filter(|(n, t)| read_or(&output.0.join(format!("{n}.lua")), t) != *t).count();
        println!(
            "round {}: {kept} of {proposed} renames, {edits_kept} of {edits} edits and {notes_kept} of {notes} comments kept, {renamed} of {} programs changed; {} another program, {} printing otherwise",
            round + 1,
            texts.len(),
            changed_code.len(),
            changed_output.len()
        );
        for n in changed_code.iter().chain(&changed_output).take(5) {
            println!("  {n}");
        }
        if summary.is_empty() {
            println!("  the decompiler said: {}", said.trim().lines().last().unwrap_or("nothing"));
        }
        good &= changed_code.is_empty() && changed_output.is_empty() && !summary.is_empty();
    }
    // a check that kept nothing, or refused nothing, tested nothing
    let kinds = [("renames", kept_all, proposed_all), ("edits", edits_kept_all, edits_all), ("comments", notes_kept_all, notes_all)];
    for (what, kept, all) in kinds {
        if kept == 0 || kept == all {
            println!("assist: {kept} of {all} {what} kept: the fake model should give some of each kind");
            good = false;
        }
    }
    println!("assist {}", if good { "clean" } else { "FAILED" });
    good
}

fn read_or(path: &Path, fallback: &str) -> String {
    std::fs::read(path).map(|b| text_of(&b)).unwrap_or_else(|_| fallback.to_string())
}

/// Output with the line numbers of error messages (`name:12:`) left out.
fn no_lines(output: &str) -> String {
    let b = output.as_bytes();
    let mut out = String::with_capacity(output.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b':' {
            let digits = b[i + 1..].iter().take_while(|c| c.is_ascii_digit()).count();
            if digits > 0 && b.get(i + 1 + digits) == Some(&b':') {
                out.push_str(":#");
                i += 1 + digits;
                continue;
            }
        }
        let ch = output[i..].chars().next().unwrap_or('?');
        out.push(ch);
        i += ch.len_utf8().max(1);
    }
    out
}

/// (kept, proposed) from `..., K of P <what>, ...`.
fn counts(summary: &str, what: &str) -> (usize, usize) {
    let Some(at) = summary.find(what) else { return (0, 0) };
    let head = &summary[..at];
    let w: Vec<&str> = head.split_whitespace().collect();
    let n = w.len();
    if n >= 3 && w[n - 2] == "of" {
        (w[n - 3].parse().unwrap_or(0), w[n - 1].parse().unwrap_or(0))
    } else {
        (0, 0)
    }
}

// ---- the fake model -----------------------------------------------------------

/// Answers a request on standard input with renames, some fine and some wrong.
pub fn fake_model(seed: u64) -> u8 {
    let mut request = String::new();
    if std::io::stdin().read_to_string(&mut request).is_err() {
        return 1;
    }
    // the same request gets the same answer, whichever order requests go in
    let hash = request.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
    let mut rng = Rng::new(seed ^ hash);
    let mut answer = String::new();
    for (k, file) in files(&request).iter().enumerate() {
        let decls = declarations(file);
        let words = identifiers(file);
        let fields = fields(file);
        let mut given: Vec<String> = Vec::new();
        for (line, old) in &decls {
            if rng.below(10) >= 7 {
                continue;
            }
            let other = |rng: &mut Rng| decls[rng.below(decls.len())].1.clone();
            let (line, new) = match rng.below(20) {
                0..=8 => (*line, format!("{old}{}", ["Value", "Item", "Count", "Part", "Name"][rng.below(5)])),
                9 | 10 => (*line, other(&mut rng)),
                11 | 12 => (*line, words[rng.below(words.len())].clone()),
                13 => (*line, ["print", "game", "math", "table", "string", "script", "workspace", "error"][rng.below(8)].to_string()),
                14 if !fields.is_empty() => (*line, fields[rng.below(fields.len())].clone()),
                15 => (*line, ["end", "local", "self", "nil", "function"][rng.below(5)].to_string()),
                16 => (line + 1, format!("{old}Moved")),
                17 if !given.is_empty() => (*line, given[rng.below(given.len())].clone()),
                _ => (*line, format!("{old}Renamed")),
            };
            given.push(new.clone());
            answer.push_str(&format!("{} {line} {old} {new}\n", k + 1));
        }
        answer.push_str("not a rename\n");
        // edits: some that keep the program (a note, a blank line), some that
        // do not (a line gone, doubled or swapped, a number or a test changed,
        // code added), some that cannot be made (past the end, overlapping)
        let n = file.len();
        for _ in 0..rng.below(4) + usize::from(n > 0) {
            let a = rng.below(n) + 1;
            let line = &file[a - 1];
            let pad: String = line.chars().take_while(|c| *c == ' ').collect();
            let (first, last, body): (usize, usize, Vec<String>) = match rng.below(12) {
                0 | 1 => (a, a, vec![format!("{pad}-- a note"), line.clone()]),
                2 | 3 => (a, a, vec![String::new(), line.clone()]),
                4 => (a, a, Vec::new()),
                5 => (a, a, vec![line.clone(), line.clone()]),
                6 if a < n => (a, a + 1, vec![file[a].clone(), line.clone()]),
                7 => (a, a, vec![bump_number(line)]),
                8 => (a, a, vec![line.replace(" == ", " ~= ").replace(" < ", " <= ")]),
                9 => (a, a, vec![line.clone(), format!("{pad}print(\"injected\")")]),
                10 => (n + 1, n + 2, vec!["-- past the end".to_string()]),
                _ => (a.saturating_sub(1).max(1), a, vec![format!("{pad}-- spanning"), line.clone()]),
            };
            answer.push_str(&format!("EDIT {} {first} {last}\n", k + 1));
            for b in body {
                answer.push_str(&b);
                answer.push('\n');
            }
            answer.push_str("END\n");
        }
        // comments: above any line at all, past the end, or text a comment
        // could be read otherwise by (`[[`)
        for _ in 0..rng.below(3) {
            let a = rng.below(n + 2) + 1;
            let text = ["Does the thing.", "[[ not a long comment", "ends --]]", "x = 1"][rng.below(4)];
            answer.push_str(&format!("NOTE {} {a} {text}\n", k + 1));
        }
    }
    print!("{answer}");
    0
}

/// The line with its first number one more, if it has one.
fn bump_number(line: &str) -> String {
    let b = line.as_bytes();
    let start = (0..b.len()).find(|&i| b[i].is_ascii_digit() && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_')));
    let Some(s) = start else { return format!("{line} -- no number") };
    let e = (s..b.len()).find(|&i| !b[i].is_ascii_digit()).unwrap_or(b.len());
    let n: u64 = line[s..e].parse().unwrap_or(0);
    format!("{}{}{}", &line[..s], n + 1, &line[e..])
}

/// The numbered lines of each file in a request.
fn files(request: &str) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    for l in request.lines() {
        if l.starts_with("=== FILE ") {
            out.push(Vec::new());
        } else if let (Some(file), Some((_, text))) = (out.last_mut(), l.split_once("| ")) {
            file.push(text.to_string());
        } else if let (Some(file), true) = (out.last_mut(), l.ends_with('|')) {
            file.push(String::new());
        }
    }
    out
}

/// (line from 1, name) of what looks declared: locals, parameters, loop
/// variables. Rough on purpose: what is not a declaration is refused.
fn declarations(file: &[String]) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (i, l) in file.iter().enumerate() {
        let s = l.trim_start();
        let mut names: Vec<&str> = Vec::new();
        if let Some(r) = s.strip_prefix("local ") {
            if !r.starts_with("function ") {
                names.extend(r.split(" = ").next().unwrap_or("").split(", "));
            }
        }
        if let Some(r) = s.strip_prefix("for ") {
            names.extend(r.split([' ', '=']).next().unwrap_or("").split(','));
            if let Some((vars, _)) = r.split_once(" in ") {
                names.extend(vars.split(", "));
            }
        }
        if let Some(open) = l.rfind("function") {
            if let Some(params) = l[open..].split_once('(').and_then(|(_, p)| p.strip_suffix(')')) {
                names.extend(params.split(", ").map(|p| p.split(':').next().unwrap_or("")));
            }
        }
        let mut seen = HashSet::new();
        for n in names {
            let n = n.trim();
            if !n.is_empty() && n != "..." && n.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') && seen.insert(n) {
                out.push((i + 1, n.to_string()));
            }
        }
    }
    out
}

/// Every identifier in the file, fields and all.
fn identifiers(file: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for l in file {
        for w in l.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
            if !w.is_empty() && !w.as_bytes()[0].is_ascii_digit() && seen.insert(w.to_string()) {
                out.push(w.to_string());
            }
        }
    }
    if out.is_empty() {
        out.push("x".to_string());
    }
    out
}

/// The names after `.` or `:` in the file.
fn fields(file: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for l in file {
        let b = l.as_bytes();
        for (i, c) in b.iter().enumerate() {
            if (*c == b'.' || *c == b':') && i + 1 < b.len() && (b[i + 1].is_ascii_alphabetic() || b[i + 1] == b'_') {
                let e = b[i + 1..].iter().position(|c| !(c.is_ascii_alphanumeric() || *c == b'_')).map_or(b.len(), |p| i + 1 + p);
                let w = &l[i + 1..e];
                if seen.insert(w.to_string()) {
                    out.push(w.to_string());
                }
            }
        }
    }
    out
}
