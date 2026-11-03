//! Damaged bytecode: random programs are compiled, then each is copied with
//! a few bytes flipped, cut short, or with bytes put in (seeded). Whatever
//! comes in, the decompiler answers: a text, or why it refuses. It must
//! never crash or hang.

use crate::gen;
use crate::luau::{decompile_many, par_map, Args, Item, Tools};
use crate::rng::Rng;

const LIMIT: u64 = 60;

pub fn main(args: &Args) -> u8 {
    let tools = Tools::find();
    let binary = args.opt("--bin").map_or(tools.decompiler.clone(), String::from);
    let clean = run(&tools, &binary, args.num("--count", 100), args.num("--seed", 1), args.num("--size", 25), args.num("--per", 10));
    u8::from(!clean)
}

/// Prints what came of it; true when nothing crashed or hung.
pub fn run(tools: &Tools, binary: &str, count: u64, seed: u64, size: usize, per: usize) -> bool {
    let seeds: Vec<u64> = (seed..seed + count).collect();
    let srcs: Vec<Item> = par_map(&seeds, |s| Item::new(format!("g{s}.luau"), gen::program(*s, size)));
    let code: Vec<Vec<u8>> = tools.compile(&srcs).into_iter().flatten().collect();
    let mut rng = Rng::new(seed ^ 0x5eed_da4a_6e00);
    let mut damaged: Vec<Vec<u8>> = Vec::with_capacity(code.len() * per);
    for bc in &code {
        for _ in 0..per {
            damaged.push(damage(bc, &mut rng));
        }
    }
    let done = decompile_many(binary, &damaged, LIMIT);
    let (mut text, mut refused, mut crashed, mut hung) = (0, 0, Vec::new(), Vec::new());
    for (i, r) in done.iter().enumerate() {
        match r {
            Ok(_) => text += 1,
            Err(why) if why == "timed out" => hung.push(i),
            // a crash leaves only the status: a refusal says why and exits 1
            Err(why) if why.starts_with("exit status ") && !why.starts_with("exit status 1:") => crashed.push((i, why.clone())),
            Err(_) => refused += 1,
        }
    }
    println!(
        "{} damaged copies of {} programs: {text} decompiled, {refused} refused, {} crashed, {} hung",
        damaged.len(),
        code.len(),
        crashed.len(),
        hung.len()
    );
    for (i, why) in crashed.iter().take(10) {
        println!("  crashed on copy {i} of program {}: {why}", seeds[*i / per]);
    }
    for i in hung.iter().take(10) {
        println!("  hung on copy {i} of program {}", seeds[*i / per]);
    }
    crashed.is_empty() && hung.is_empty()
}

/// A copy with some bytes flipped (most often), cut short, or with bytes put in.
fn damage(bc: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut b = bc.to_vec();
    match rng.below(5) {
        0 => b.truncate(1 + rng.below(b.len().max(2) - 1)),
        1 => {
            let at = rng.below(b.len().max(1));
            let n = 1 + rng.below(8);
            let extra: Vec<u8> = (0..n).map(|_| rng.below(256) as u8).collect();
            b.splice(at..at, extra);
        }
        _ => {
            for _ in 0..1 + rng.below(4) {
                let at = rng.below(b.len().max(1));
                if let Some(x) = b.get_mut(at) {
                    *x = rng.below(256) as u8;
                }
            }
        }
    }
    b
}
