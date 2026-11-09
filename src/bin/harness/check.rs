//! Everything that has to hold before a change goes in: the decompiler builds,
//! the hand-written cases behave, random programs behave. With $LUAU_RUN
//! pointing at a folder of script dumps (<place>/<id>.wire) it also decompiles
//! those into out/ and reports jump flags, dispatchers, undeclared names and how
//! many functions recompile to the original. tools/semantic and the folders in
//! $LUAU_SUITES must behave too, but for what their known.txt lists: with
//! --full, compiled and run at -O0, -O1 and -O2.
//!
//! Failing random programs are kept in failed/.

use std::path::Path;

use crate::cases::{self, Print, Suite};
use crate::difftest::{self, Options};
use crate::edits::{compare_place, dump_places};
use crate::luau::{command, percent, repo, text_of, Args, Tools};
use crate::text::undeclared;

const QUICK: [(u64, usize, u64); 1] = [(300, 12, 700000)];
const FULL: [(u64, usize, u64); 40] = [
    (300, 3, 5000),
    (300, 18, 20000),
    (300, 12, 700000),
    (200, 25, 1),
    (300, 15, 900000),
    (300, 8, 1300000),
    (300, 10, 2000000),
    (300, 20, 3000000),
    (300, 14, 4000000),
    (300, 16, 5000000),
    (300, 22, 6000000),
    (300, 30, 7000000),
    (300, 6, 8000000),
    (300, 11, 9000000),
    (300, 13, 10000000),
    (300, 24, 11000000),
    (300, 9, 12000000),
    (300, 17, 13000000),
    (300, 7, 14000000),
    // programs that each found a bug (count 1: that seed alone), and big ones:
    // large functions reach what small ones never do (the local limit, one
    // variable per register, tails written twice)
    (1, 25, 9279),
    (1, 25, 70208),
    (1, 25, 70654),
    (1, 25, 32271),
    (1, 25, 33554),
    (1, 25, 34394),
    (1, 25, 35640),
    (1, 80, 82039),
    (1, 120, 90008),
    (1, 120, 90149),
    (1, 120, 90404),
    (1, 120, 90426),
    (1, 120, 90462),
    (1, 120, 90045),
    (1, 120, 120085),
    (1, 100, 100098),
    (1, 160, 160034),
    (1, 200, 200055),
    (1, 160, 55530),
    (1, 160, 55650),
    (60, 160, 55501),
];

/// The decompiler builds, and its own unit tests pass.
fn build() -> bool {
    for args in [&["build", "--release", "--bin", "luau_decompile"][..], &["test", "--release", "--bin", "luau_decompile"]] {
        let out = command("cargo").args(args).current_dir(repo()).output().expect("cargo");
        if !out.status.success() {
            let said = format!("{}{}", text_of(&out.stdout), text_of(&out.stderr));
            println!("cargo {}:\n{}", args[0], &said[said.len().saturating_sub(3000)..]);
            return false;
        }
    }
    true
}

fn random_sets(tools: &Tools, seeds: &[(u64, usize, u64)]) -> bool {
    let mut bad = 0;
    for (count, size, seed) in seeds {
        let options = Options {
            count: *count,
            seed: *seed,
            size: *size,
            keep: Some(repo().join("failed").to_string_lossy().into_owned()),
            binary: tools.decompiler.clone(),
        };
        let out = difftest::run(tools, &options);
        if !out.clean {
            bad += 1;
            println!("seed {seed}: {}", out.summary);
            for line in out.detail.iter().filter(|l| l.starts_with("  ")) {
                println!("{line}");
            }
        }
    }
    println!("random sets clean: {} of {}", seeds.len() - bad, seeds.len());
    bad == 0
}

fn real_scripts(tools: &Tools, dumps: &str) -> bool {
    let root = std::path::Path::new(dumps);
    let output = repo().join("out");
    let places = dump_places(root);
    let mut good = true;
    for place in &places {
        let (wire, lua) = (root.join(place), output.join(place));
        let run = command(&tools.decompiler).arg("--dir").arg(&wire).arg("--out").arg(&lua).output().expect("the decompiler");
        let said = text_of(&run.stdout) + &text_of(&run.stderr);
        let line = said.trim().lines().last().unwrap_or("").to_string();
        println!("{line}");
        if !run.status.success() {
            println!("  the decompiler stopped on {}: {}", place, run.status);
            good = false;
        }
        let found = compare_place(tools, root, &output, place);
        let same = found.functions - found.rows.len();
        println!("  {} functions, {same} identical ({})", found.functions, percent(same, found.functions));
        good &= line.contains(", 0 failed") && line.contains(" 0 dispatchers");
    }
    let dirs: Vec<String> = places.iter().map(|p| output.join(p).to_string_lossy().into_owned()).collect();
    let found = undeclared(&dirs);
    println!("{} undeclared generated names in {} files", found.names, found.files);
    for line in &found.examples {
        println!("{line}");
    }
    good && found.names == 0
}

/// A folder of programs that print what they do, each about one way a
/// decompiler can change behaviour: tools/semantic, and any in $LUAU_SUITES
/// (separated by `;`). Those known not to behave yet are listed in the folder's
/// known.txt, with why; any other failing, or a listed one now behaving, fails
/// the check, so the list stays true. With `full`, the programs go through at
/// -O1 and -O0 as well: compiled, decompiled and run at each level, since a
/// table's layout and so what `pairs` visits first differ between levels.
fn suite(tools: &Tools, dir: &Path, full: bool) -> bool {
    let has_programs = std::fs::read_dir(dir)
        .map(|d| d.flatten().any(|e| e.path().extension().is_some_and(|x| x == "luau")))
        .unwrap_or(false);
    if !has_programs {
        return true;
    }
    let levels: &[u8] = if full { &[2, 1, 0] } else { &[2] };
    let mut ok = true;
    for &level in levels {
        ok = suite_at(tools, dir, level) && ok;
    }
    ok
}

fn suite_at(tools: &Tools, dir: &Path, level: u8) -> bool {
    let suite = Suite {
        dir: dir.to_string_lossy().into_owned(),
        binary: tools.decompiler.clone(),
        show: None,
        print: Print::Nothing,
        errors_are_output: true,
        level,
    };
    let found = cases::run_suite(&suite);
    let listed = std::fs::read_to_string(dir.join("known.txt")).unwrap_or_default();
    let known: Vec<&str> = listed
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    let bad: Vec<&(String, &str)> = found.misbehaving.iter().filter(|m| !known.contains(&m.0.as_str())).collect();
    let fixed: Vec<&&str> = known.iter().filter(|k| !found.misbehaving.iter().any(|m| m.0 == **k)).collect();
    let behave = found.total - found.misbehaving.len();
    let at = if level == 2 { String::new() } else { format!(" at -O{level}") };
    println!("{}{at}: {behave} of {} behave, {} known not to yet", dir.display(), found.total, known.len());
    for (name, why) in &bad {
        println!("  {name}: {why}");
    }
    for name in &fixed {
        println!("  {name} behaves now: take it off its known.txt");
    }
    bad.is_empty() && fixed.is_empty()
}

fn suites(tools: &Tools, full: bool) -> bool {
    let mut dirs = vec![repo().join("tools").join("semantic")];
    if let Ok(more) = std::env::var("LUAU_SUITES") {
        dirs.extend(more.split(';').filter(|d| !d.is_empty()).map(std::path::PathBuf::from));
    }
    let mut ok = true;
    for dir in &dirs {
        ok = suite(tools, dir, full) && ok;
    }
    ok
}

pub fn main(args: &Args) -> u8 {
    let tools = Tools::find();
    let mut ok = build();
    if ok {
        ok = cases::run(&Args(Vec::new()), false) == 0;
        ok = suites(&tools, args.flag("--full")) && ok;
        ok = crate::golden::run(&tools, false) && ok;
    }
    let seeds: &[(u64, usize, u64)] = if args.flag("--full") { &FULL } else { &QUICK };
    ok = random_sets(&tools, seeds) && ok;
    if args.flag("--full") {
        // damaged bytecode is refused or decompiled, never a crash or a hang
        ok = crate::damage::run(&tools, &tools.decompiler, 60, 1, 25, 10) && ok;
        // bytecode no compiler lays out this way but the VM still runs: these
        // forms (no line info, flipped tests, jumps through jumps) reorder
        // nothing, so each must come out behaving the same; edge-value number
        // constants (NaN, infinities, -0) must round-trip; none may crash or
        // hang (block reorderings, which may be refused, are not checked here)
        let craft = crate::craft::Options {
            count: 120,
            seed: 1,
            size: 25,
            per: 1,
            kinds: vec![
                crate::craft::Kind::Stripped,
                crate::craft::Kind::Inverted,
                crate::craft::Kind::Bounced,
                crate::craft::Kind::Numbers,
            ],
            keep: None,
            write: None,
            binary: tools.decompiler.clone(),
        };
        ok = crate::craft::run(&tools, &craft) && ok;
        // --assist: renames a model might give, right and wrong, may change
        // no program's bytecode or output
        ok = crate::assist::run(&tools, 1, 2) && ok;
    }
    if let Ok(dumps) = std::env::var("LUAU_RUN") {
        ok = real_scripts(&tools, &dumps) && ok;
    }
    println!("{}", if ok { "all good" } else { "FAILED" });
    u8::from(!ok)
}
