//! Comparing a function with what it recompiles to. Instructions that do not
//! change behaviour are left out: fast-call hints, Roblox-only variants of
//! standard opcodes, NOP and COVERAGE, long forms of jumps and loads. What comes
//! back identical almost certainly means what the original meant.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::bytecode::{op_name, Insn, Module, Proto, K};
use crate::diff::{Matcher, Tag};

fn names() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| (0..=255u8).map(|o| &*Box::leak(op_name(o).into_boxed_str())).collect())
}

fn dropped(name: &str) -> bool {
    matches!(
        name,
        "NOP" | "COVERAGE" | "FASTCALL" | "FASTCALL1" | "FASTCALL2" | "FASTCALL2K" | "FASTCALL3" | "NATIVECALL" | "BREAK"
    )
}

fn same_as(name: &'static str) -> &'static str {
    match name {
        "CALLFB" => "CALL",
        "NAMECALLUDATA" => "NAMECALL",
        "GETUDATAKS" => "GETTABLEKS",
        "SETUDATAKS" => "SETTABLEKS",
        "JUMPX" | "JUMPBACK" => "JUMP",
        "LOADKX" => "LOADK",
        n => n,
    }
}

/// Is this GETIMPORT of the global pcall or xpcall?
fn is_pcall(m: &Module, p: &Proto, ins: &Insn) -> bool {
    if ins.aux >> 30 != 1 {
        return false;
    }
    match p.k.get(((ins.aux >> 20) & 1023) as usize) {
        Some(K::Str(Some(s))) => matches!(m.strings[*s].as_slice(), b"pcall" | b"xpcall"),
        _ => false,
    }
}

/// The opcode sequence. Roblox's compiler treats pcall as a fast builtin and
/// loads it after its arguments; the Luau we recompile with loads it before.
/// No source can choose between those, so both compare with the load at the call.
pub fn shape(m: &Module, p: &Proto) -> Vec<&'static str> {
    let table = names();
    let mut out = Vec::with_capacity(p.code.len());
    let mut waiting: HashSet<usize> = HashSet::new();
    for ins in &p.code {
        let name = table[ins.op as usize];
        if name == "GETIMPORT" && is_pcall(m, p, ins) {
            waiting.insert(ins.a());
            continue;
        }
        if matches!(name, "CALL" | "CALLFB") && waiting.remove(&ins.a()) {
            out.push("GETIMPORT");
        }
        if name != "FASTPCALL" && !dropped(name) {
            out.push(same_as(name));
        }
    }
    out.extend(waiting.iter().map(|_| "GETIMPORT"));
    out
}

/// Also the target register, so also whether the variables came back as written.
pub fn exact(p: &Proto) -> Vec<String> {
    let table = names();
    p.code
        .iter()
        .filter(|i| !dropped(table[i.op as usize]))
        .map(|i| format!("{}:{}", same_as(table[i.op as usize]), i.a()))
        .collect()
}

/// Every instruction word the same (registers, operands, jump offsets and aux
/// too) and the constants equal by value in order, a closure constant meaning
/// its paired function: the function came back as it was compiled.
pub fn strict(am: &Module, a: &Proto, bm: &Module, b: &Proto, paired: &[Option<usize>]) -> bool {
    let words = |p: &Proto| p.code.iter().map(|i| (i.w, i.aux)).collect::<Vec<_>>();
    if a.code.len() != b.code.len() || words(a) != words(b) || a.k.len() != b.k.len() {
        return false;
    }
    let text = |m: &Module, s: &Option<usize>| s.map(|s| m.strings[s].clone());
    a.k.iter().zip(&b.k).all(|(x, y)| match (x, y) {
        (K::Nil, K::Nil) | (K::Class, K::Class) => true,
        (K::Bool(x), K::Bool(y)) => x == y,
        (K::Number(x), K::Number(y)) => x.to_bits() == y.to_bits(),
        (K::Integer(x), K::Integer(y)) => x == y,
        (K::Vector(x), K::Vector(y)) => x.iter().zip(y).all(|(p, q)| p.to_bits() == q.to_bits()),
        (K::Str(x), K::Str(y)) => text(am, x) == text(bm, y),
        // constant indices, the same constants: the same names
        (K::Import(x), K::Import(y)) => x == y,
        (K::TableK(x), K::TableK(y)) => x == y,
        (K::Closure(x), K::Closure(y)) => paired.get(*x).copied().flatten() == Some(*y),
        _ => false,
    })
}

/// For each function of a, its counterpart in b. They are numbered in compile
/// order, so one gone missing would misalign all that follow: pair the same
/// function by name and arity, then what is left by identical code, then by
/// similarity.
pub fn pair(am: &Module, bm: &Module) -> Vec<Option<usize>> {
    let (a, b) = (&am.protos, &bm.protos);
    let mut paired: Vec<Option<usize>> = vec![None; a.len()];
    let mut used = vec![false; b.len()];
    let sig = |p: &Proto| (p.name.clone(), p.numparams);
    let sig_a: Vec<_> = a.iter().map(sig).collect();
    let sig_b: Vec<_> = b.iter().map(sig).collect();
    for (tag, i1, i2, j1, _) in Matcher::new(&sig_a, &sig_b).opcodes() {
        if tag == Tag::Equal {
            for k in 0..i2 - i1 {
                paired[i1 + k] = Some(j1 + k);
                used[j1 + k] = true;
            }
        }
    }
    let shapes_a: Vec<Vec<&'static str>> = a.iter().map(|p| shape(am, p)).collect();
    let shapes_b: Vec<Vec<&'static str>> = b.iter().map(|p| shape(bm, p)).collect();

    let mut by_sig: HashMap<&(Option<String>, usize), Vec<usize>> = HashMap::new();
    for j in (0..b.len()).filter(|j| !used[*j]) {
        by_sig.entry(&sig_b[j]).or_default().push(j);
    }
    for i in 0..a.len() {
        if paired[i].is_none() {
            if let Some(list) = by_sig.get_mut(&sig_a[i]).filter(|l| !l.is_empty()) {
                let j = list.remove(0);
                paired[i] = Some(j);
                used[j] = true;
            }
        }
    }

    let mut by_code: HashMap<&[&'static str], Vec<usize>> = HashMap::new();
    for j in (0..b.len()).filter(|j| !used[*j]) {
        by_code.entry(&shapes_b[j]).or_default().push(j);
    }
    for i in 0..a.len() {
        if paired[i].is_none() {
            if let Some(list) = by_code.get_mut(shapes_a[i].as_slice()).filter(|l| !l.is_empty()) {
                let j = list.remove(0);
                paired[i] = Some(j);
                used[j] = true;
            }
        }
    }

    // what is left: the most alike of the same arity
    let mut free: Vec<usize> = (0..b.len()).filter(|j| !used[*j]).collect();
    for i in 0..a.len() {
        if paired[i].is_some() {
            continue;
        }
        let sa = &shapes_a[i];
        let (mut best, mut score) = (None, 0.5);
        for &j in &free {
            let sb = &shapes_b[j];
            let size = (sa.len() + 1) as f64 / (sb.len() + 1) as f64;
            if b[j].numparams != a[i].numparams || !(0.5..=2.0).contains(&size) {
                continue;
            }
            let m = Matcher::new(sa, sb);
            if m.real_quick_ratio() > score && m.quick_ratio() > score {
                let r = m.ratio();
                if r > score {
                    (best, score) = (Some(j), r);
                }
            }
        }
        if let Some(j) = best {
            paired[i] = Some(j);
            free.retain(|x| *x != j);
        }
    }
    paired
}
