//! Everything that has to hold before a change goes in: the decompiler builds,
//! the hand-written cases behave, random programs behave. With $LUAU_RUN
//! pointing at a folder of script dumps (<place>/<id>.wire) it also decompiles
//! those into out/ and reports jump flags, dispatchers, undeclared names and how
//! many functions recompile to the original.
//!
//! Failing random programs are kept in failed/.

use crate::cases;
use crate::difftest::{self, Options};
use crate::edits::{compare_place, dump_places};
use crate::luau::{command, percent, repo, text_of, Args, Tools};
use crate::text::undeclared;

const QUICK: [(u64, usize, u64); 1] = [(300, 12, 700000)];
const FULL: [(u64, usize, u64); 19] = [
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
];

fn build() -> bool {
    let out = command("cargo")
        .args(["build", "--release", "--bin", "luau_decompile"])
        .current_dir(repo())
        .output()
        .expect("cargo");
    if !out.status.success() {
        let err = text_of(&out.stderr);
        println!("{}", &err[err.len().saturating_sub(3000)..]);
    }
    out.status.success()
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

pub fn main(args: &Args) -> u8 {
    let tools = Tools::find();
    let mut ok = build();
    if ok {
        ok = cases::run(&Args(Vec::new()), false) == 0;
    }
    let seeds: &[(u64, usize, u64)] = if args.flag("--full") { &FULL } else { &QUICK };
    ok = random_sets(&tools, seeds) && ok;
    if let Ok(dumps) = std::env::var("LUAU_RUN") {
        ok = real_scripts(&tools, &dumps) && ok;
    }
    println!("{}", if ok { "all good" } else { "FAILED" });
    u8::from(!ok)
}
