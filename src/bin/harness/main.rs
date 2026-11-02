//! harness: tests the decompiler against the real Luau compiler and VM.
//!
//!   harness check [--full]        everything that has to hold before a change goes in
//!   harness cases [--show NAME]   the hand-written programs in tools/luau_cases
//!   harness difftest              random programs must print the same after decompiling
//!   harness reduce FILE           shrinks a program the decompiler gets wrong
//!   harness edits                 which real functions differ, and by what edit
//!   harness undeclared DIR...     generated names used without a declaration
//!   harness same A B              are two output trees identical?
//!   harness gen SEED SIZE         prints one generated program
//!
//! `luau_check` and `luau_run` (see CMakeLists.txt) are the real compiler and
//! VM; they are looked for in build/Release, or in $LUAU_BIN.

#[allow(dead_code)]
#[path = "../../bytecode.rs"]
mod bytecode;

mod cases;
mod check;
mod diff;
mod difftest;
mod edits;
mod gen;
mod luau;
mod reduce;
mod rng;
mod shape;
mod text;

use std::process::ExitCode;

use luau::Args;

const USAGE: &str = "\
harness check [--full]
harness cases [--dir DIR] [--show NAME] [--bin PATH]
harness difftest [--count N] [--seed S] [--size N] [--keep DIR] [--bin PATH]
harness reduce FILE [--shape] [--out FILE] [--seconds N]
harness edits [--places a,b] [--top N] [--show KEY] [--count N]
              [--save NAME] [--since NAME] [--worse KEY]
harness undeclared DIR...
harness same A B
harness gen SEED SIZE
";

fn main() -> ExitCode {
    let mut argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.is_empty() {
        eprint!("{USAGE}");
        return ExitCode::from(2);
    }
    let command = argv.remove(0);
    let args = Args(argv);
    let code = match command.as_str() {
        "check" => check::main(&args),
        "cases" => cases::main(&args),
        "difftest" => difftest::main(&args),
        "reduce" => reduce::main(&args),
        "edits" => edits::main(&args),
        "undeclared" => text::undeclared_main(&args),
        "same" => text::same_main(&args),
        "gen" => gen::main(&args),
        _ => {
            eprint!("{USAGE}");
            2
        }
    };
    ExitCode::from(code)
}
