//! harness validate DIR...: what the decompiler wrote, checked the way a user
//! would take it. Every .lua and .luau file under the folders compiles with
//! the real compiler (-O2, or `--level N`), and uses no generated name it
//! does not declare.

use std::path::Path;

use crate::luau::{clip, walk, Args, Item, Tools};
use crate::text::undeclared;

/// One call to the compiler takes this many files: a time limit applies to
/// each call.
const BATCH: usize = 200;

pub fn main(args: &Args) -> u8 {
    let mut dirs: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.0.len() {
        if args.0[i] == "--level" {
            i += 2;
            continue;
        }
        dirs.push(args.0[i].clone());
        i += 1;
    }
    if dirs.is_empty() {
        eprintln!("harness validate DIR... [--level N]");
        return 2;
    }
    let mut tools = Tools::find();
    tools.level = args.num("--level", 2u8);
    let mut files = Vec::new();
    for d in &dirs {
        walk(Path::new(d), &mut files);
    }
    files.retain(|p| p.extension().is_some_and(|x| x == "lua" || x == "luau"));
    files.sort();
    let items: Vec<Item> = files
        .iter()
        .map(|p| Item::new(p.to_string_lossy().into_owned(), std::fs::read(p).unwrap_or_default()))
        .collect();
    let mut failed: Vec<(String, String)> = Vec::new();
    for batch in items.chunks(BATCH) {
        for (item, got) in batch.iter().zip(tools.compile(batch)) {
            if let Err(why) = got {
                failed.push((item.name.clone(), why));
            }
        }
    }
    println!("{} files: {} compile, {} do not", items.len(), items.len() - failed.len(), failed.len());
    for (name, why) in failed.iter().take(10) {
        println!("  {name}: {}", clip(why.trim(), 160));
    }
    let found = undeclared(&dirs);
    println!("{} undeclared generated names in {} files", found.names, found.files);
    for line in found.examples.iter().take(10) {
        println!("{line}");
    }
    u8::from(!failed.is_empty() || found.names > 0)
}
