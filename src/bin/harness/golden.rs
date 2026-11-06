//! Bytecode no source here compiles to, kept as it came: tools/bytecode/NAME.luaubc
//! (a Roblox client's form) or NAME.bc (stock). Its decompiled text must stay
//! NAME.luau, and where NAME.out holds what the original printed (its output
//! and the `=== ok` or `=== error: ...` line), the decompiled program must print
//! that. `--bless` writes the decompiled text over each NAME.luau after a change
//! meant to alter it; NAME.out is never written, it comes from the original.
//! `--bin` tries another decompiler binary.

use std::path::{Path, PathBuf};

use crate::luau::{command, read_text, repo, run_limited, text_of, Args, Item, Tools, USUAL_BUDGET};

pub fn main(args: &Args) -> u8 {
    let mut tools = Tools::find();
    if let Some(binary) = args.opt("--bin") {
        tools.decompiler = binary.to_string();
    }
    u8::from(!run(&tools, args.flag("--bless")))
}

fn cases(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "luaubc" || x == "bc"))
        .collect();
    found.sort();
    found
}

pub fn run(tools: &Tools, bless: bool) -> bool {
    let dir = repo().join("tools").join("bytecode");
    let files = cases(&dir);
    if files.is_empty() {
        return true;
    }
    let mut bad: Vec<String> = Vec::new();
    let mut programs: Vec<Item> = Vec::new();
    let mut wanted: Vec<(String, String)> = Vec::new();
    for file in &files {
        let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let mut cmd = command(&tools.decompiler);
        if file.extension().is_some_and(|x| x == "bc") {
            cmd.arg("--upstream");
        }
        cmd.arg(file);
        let ran = run_limited(&mut cmd, None, 120);
        if !ran.status.is_some_and(|s| s.success()) {
            let err = text_of(&ran.stderr);
            bad.push(format!("{stem}: not decompiled: {}", err.trim().lines().last().unwrap_or("timed out")));
            continue;
        }
        let text = text_of(&ran.stdout);
        let kept = dir.join(format!("{stem}.luau"));
        if bless {
            std::fs::write(&kept, &text).expect("the kept text written");
        } else {
            let want = read_text(&kept.to_string_lossy());
            if want != text {
                let (n, (a, b)) = want
                    .lines()
                    .chain(std::iter::repeat(""))
                    .zip(text.lines().chain(std::iter::repeat("")))
                    .enumerate()
                    .find(|(_, (a, b))| a != b)
                    .unwrap_or((0, ("", "")));
                bad.push(format!("{stem}: the text differs from {stem}.luau at line {}:\n    kept: {a}\n    now:  {b}", n + 1));
            }
        }
        let out = dir.join(format!("{stem}.out"));
        if out.exists() {
            programs.push(Item::new(stem.clone(), text));
            wanted.push((stem, read_text(&out.to_string_lossy())));
        }
    }
    let found = tools.outputs(&programs, 60, USUAL_BUDGET);
    for (stem, want) in &wanted {
        match found.get(stem) {
            Some(got) if got.trim_end() == want.trim_end() => {}
            Some(got) => bad.push(format!("{stem}: prints otherwise than the original:\n    original: {}\n    now:      {}", want.trim_end().replace('\n', " | "), got.trim_end().replace('\n', " | "))),
            None => bad.push(format!("{stem}: never finished")),
        }
    }
    let verb = if bless { "kept anew" } else { "as kept" };
    println!(
        "{}: {} of {} bytecode files {verb}, {} of {} printing as the original",
        dir.display(),
        files.len() - bad.iter().filter(|b| !b.contains("prints") && !b.contains("never")).count(),
        files.len(),
        wanted.len() - bad.iter().filter(|b| b.contains("prints") || b.contains("never")).count(),
        wanted.len()
    );
    for b in &bad {
        println!("  {b}");
    }
    bad.is_empty()
}
