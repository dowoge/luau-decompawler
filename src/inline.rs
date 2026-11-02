//! Luau inlines small local functions at -O2, and keeps their definitions.
//! Source lines show where a helper's code sits inside its caller; this walks
//! the helper's instructions against the caller's to be sure, and to learn
//! which registers held its arguments. Any difference means no match: the code
//! then stays as it is, so a wrong call cannot come out of here.
//!
//! The compiler's work shows in three places. Arguments that are locals are
//! used where they are, so a parameter maps to any register. `return` becomes
//! a store to the call's result and a jump to the end, or a copy of the return
//! that follows when the jump folds. Upvalues of the helper may be plain
//! locals of the caller. A constant argument is loaded where it is read, or
//! becomes the K form of the instruction; when it folds a branch away the
//! code no longer matches and is left alone.

use std::cell::Cell;
use crate::hash::{HashMap, HashSet};

use crate::bytecode::{is_forgprep, op, op_name, Insn, Module, Proto, K};
use crate::decomp::{import_expr, kexpr};
use crate::analysis::full_effects;

thread_local! {
    /// How far the last walk got: (instructions matched, of, helper's op, caller's op).
    static REACHED: Cell<(usize, usize, u8, u8)> = const { Cell::new((0, 0, 0, 0)) };
    /// The final check that refused, if the walk itself got through.
    static CHECK: Cell<&'static str> = const { Cell::new("") };
}

fn refuse<T>(why: &'static str) -> Option<T> {
    CHECK.with(|c| c.set(why));
    None
}

/// How far the last match_call got, for LUAU_WHY.
pub fn last_refusal() -> (usize, String) {
    let (k, n, p, c) = REACHED.with(Cell::get);
    let check = CHECK.with(Cell::get);
    (k, format!("{k}/{n} {} / {} {check}", op_name(p), op_name(c)))
}

#[derive(Clone)]
pub enum Arg {
    /// The register it was read from and the pc of that read.
    Reg(usize, i32),
    /// A constant, as source text.
    Const(String),
}

/// What one of the helper's upvalues is, in the caller.
#[derive(Clone, Copy, PartialEq)]
pub enum Up {
    Reg(usize),
    Up(usize),
}

pub struct Inlined {
    /// The helper's function.
    pub helper: usize,
    /// Its upvalues, as far as its code showed them.
    pub upvals: Vec<Option<Up>>,
    /// What each parameter was given.
    pub args: Vec<Option<Arg>>,
    /// The first pc after the helper's code, and the last pc in it.
    pub end: i32,
    pub last: i32,
    /// Where the result went, if the call has one, and how many in a row.
    pub result: Option<usize>,
    pub results: usize,
    /// Registers the code writes.
    pub written: Vec<usize>,
    /// Caller locals the copy wrote for an upvalue of the helper's: the call
    /// writes them through the closure. Only a closure made where its
    /// captures show can be trusted to.
    pub upval_writes: Vec<usize>,
    /// Parameters the call left out: nil, and not written when last.
    pub omitted: Vec<bool>,
}

fn ignored(o: u8) -> bool {
    matches!(
        o,
        op::NOP | op::BREAK | op::COVERAGE | op::NATIVECALL | op::FASTCALL | op::FASTCALL1 | op::FASTCALL2
            | op::FASTCALL2K | op::FASTCALL3 | op::FASTPCALL | op::PREPVARARGS
    )
}

/// Variants that mean the same thing.
fn plain(o: u8) -> u8 {
    match o {
        op::CALLFB => op::CALL,
        op::NAMECALLUDATA => op::NAMECALL,
        op::GETUDATAKS => op::GETTABLEKS,
        op::SETUDATAKS => op::SETTABLEKS,
        op::JUMPBACK | op::JUMPX => op::JUMP,
        op::LOADKX => op::LOADK,
        other => other,
    }
}

/// The first instruction at or after index i that means something.
fn skip(code: &[Insn], mut i: usize) -> usize {
    while code.get(i).is_some_and(|c| ignored(c.op)) {
        i += 1;
    }
    i
}

/// Where a jump to pc really lands.
fn land(code: &[Insn], pc: i32) -> i32 {
    let i = skip(code, code.partition_point(|c| c.pc < pc));
    code.get(i).map_or(pc, |c| c.pc)
}

struct Walk<'a> {
    m: &'a Module,
    helper: &'a Proto,
    caller: &'a Proto,
    map: Vec<Option<usize>>,
    args: Vec<Option<Arg>>,
    written: Vec<usize>,
    jumps: Vec<(i32, i32)>,
    /// Caller registers just loaded with a constant, as source text, and the
    /// pc of the load.
    loaded: HashMap<usize, (String, i32)>,
    /// Parameters the helper assigns: Luau loads a constant given for one
    /// into its register before the copy, where it does not for the others.
    assigned: Vec<bool>,
    /// Parameters the copy shows were left out (nil, not written).
    omitted: Vec<bool>,
    upvals: Vec<Option<Up>>,
    /// Helper registers holding an upvalue the caller has not been seen to read.
    unread: HashMap<usize, usize>,
    /// Every (helper register, caller register) a write has paired.
    held: HashSet<(usize, usize)>,
    /// Parameters taken to be a false or a true constant: tested before
    /// the copy showed what they were.
    assumed: Vec<Option<bool>>,
    /// A parameter a test needed to know, unknown yet.
    need: Option<usize>,
    /// Caller locals written in place of the helper's upvalue stores.
    upval_writes: Vec<usize>,
}

fn falsy(text: &str) -> bool {
    text == "nil" || text == "false"
}

impl Walk<'_> {
    /// The constant in caller register c, if it was loaded where a call
    /// given it for parameter p would load it: before the read at pc with no
    /// statement between, or anywhere for a parameter the helper assigns.
    /// `a, b = x, k` loads k before the store of a, which a call cannot say.
    fn given(&self, p: usize, c: usize, pc: i32) -> Option<String> {
        let (text, at) = self.loaded.get(&c)?;
        if self.assigned.get(p).copied().unwrap_or(false) {
            return Some(text.clone());
        }
        let from = self.caller.code.partition_point(|i| i.pc <= *at);
        let mut between = self.caller.code[from..].iter().take_while(|i| i.pc < pc);
        let statement = |o: u8| {
            matches!(plain(o), op::SETTABLE | op::SETTABLEKS | op::SETTABLEN | op::SETUPVAL | op::SETGLOBAL | op::CALL | op::RETURN)
        };
        (!between.any(|i| statement(i.op))).then(|| text.clone())
    }

    fn read(&mut self, p: usize, c: usize, pc: i32) -> Option<()> {
        if let Some(Some(Arg::Const(text))) = self.args.get(p) {
            return (self.loaded.get(&c).map(|l| &l.0) == Some(text)).then_some(());
        }
        if let Some(Some(truthy)) = self.assumed.get(p).copied() {
            // what it was taken for: a constant of that truth
            let text = self.given(p, c, pc)?;
            (falsy(&text) != truthy).then_some(())?;
            self.args[p] = Some(Arg::Const(text));
            return Some(());
        }
        if let Some(u) = self.unread.remove(&p) {
            self.upvalue(u, Up::Reg(c))?;
            self.map[p] = Some(c);
            return Some(());
        }
        match *self.map.get(p)? {
            Some(x) => (x == c).then_some(()),
            None if p < self.helper.numparams => {
                if self.loaded.contains_key(&c) {
                    let text = self.given(p, c, pc)?;
                    self.args[p] = Some(Arg::Const(text));
                    return Some(());
                }
                self.map[p] = Some(c);
                self.args[p] = Some(Arg::Reg(c, pc));
                Some(())
            }
            None => None,
        }
    }

    /// Parameter p is this constant, wherever it is read.
    fn constant(&mut self, p: usize, text: String) -> Option<()> {
        if self.map.get(p)?.is_some() || p >= self.helper.numparams {
            return None;
        }
        if self.assumed[p].is_some_and(|truthy| falsy(&text) == truthy) {
            return None;
        }
        match &self.args[p] {
            None => {
                self.args[p] = Some(Arg::Const(text));
                Some(())
            }
            Some(Arg::Const(t)) => (*t == text).then_some(()),
            Some(_) => None,
        }
    }

    fn write(&mut self, p: usize, c: usize) -> Option<()> {
        self.unread.remove(&p);
        *self.map.get_mut(p)? = Some(c);
        self.loaded.remove(&c);
        self.written.push(c);
        self.held.insert((p, c));
        Some(())
    }

    fn upvalue(&mut self, i: usize, up: Up) -> Option<()> {
        match self.upvals.get_mut(i)? {
            Some(known) => (*known == up).then_some(()),
            slot => {
                *slot = Some(up);
                Some(())
            }
        }
    }

    /// Does this test of the helper's go one way for the constants it was
    /// given? Some(true) if it jumps. A parameter tested before anything
    /// showed it is noted as needed, for a try with each truth.
    fn folded_test(&mut self, p: &Insn) -> Option<bool> {
        let r = p.a();
        if r >= self.helper.numparams || self.map[r].is_some() || self.unread.contains_key(&r) {
            return None;
        }
        let text = match &self.args[r] {
            Some(Arg::Const(t)) => Some(t.clone()),
            Some(Arg::Reg(..)) => return None,
            None => None,
        };
        match plain(p.op) {
            op::JUMPIF | op::JUMPIFNOT => {
                let truthy = match (&text, self.assumed[r]) {
                    (Some(t), _) => !falsy(t),
                    (None, Some(b)) => b,
                    (None, None) => {
                        self.need = self.need.or(Some(r));
                        return None;
                    }
                };
                Some(truthy == (p.op == op::JUMPIF))
            }
            op::JUMPXEQKNIL | op::JUMPXEQKB | op::JUMPXEQKN | op::JUMPXEQKS => {
                let text = text?;
                let k = match p.op {
                    op::JUMPXEQKNIL => "nil".to_string(),
                    op::JUMPXEQKB => if p.aux & 1 != 0 { "true" } else { "false" }.to_string(),
                    _ => kexpr(self.m, self.helper, (p.aux & 0xFFFFFF) as i64),
                };
                Some((text == k) != (p.aux >> 31 != 0))
            }
            _ => None,
        }
    }

    /// The constant c loads, as source text.
    fn load_text(&self, c: &Insn) -> Option<String> {
        Some(match c.op {
            op::LOADN => c.d().to_string(),
            op::LOADK => kexpr(self.m, self.caller, c.d() as i64),
            op::LOADKX => kexpr(self.m, self.caller, c.aux as i64),
            op::LOADNIL => "nil".to_string(),
            op::LOADB if c.c() == 0 => if c.b() != 0 { "true" } else { "false" }.to_string(),
            _ => return None,
        })
    }

    /// The helper does this with a parameter in a register; the constant the
    /// caller passed made it a K instruction.
    fn k_form(&mut self, p: &Insn, c: &Insn) -> Option<()> {
        let (pa, pb, pcc) = (p.a(), p.b(), p.c());
        let (ca, cb, cc) = (c.a(), c.b(), c.c());
        let pc = c.pc;
        let k = |w: &Self, i: i64| kexpr(w.m, w.caller, i);
        match (p.op, c.op) {
            (op::ADD, op::ADDK) | (op::SUB, op::SUBK) | (op::MUL, op::MULK) | (op::DIV, op::DIVK)
            | (op::MOD, op::MODK) | (op::POW, op::POWK) | (op::IDIV, op::IDIVK) | (op::AND, op::ANDK)
            | (op::OR, op::ORK) => {
                self.read(pb, cb, pc)?;
                let text = k(self, cc as i64);
                self.constant(pcc, text)?;
                self.write(pa, ca)
            }
            (op::SUB, op::SUBRK) | (op::DIV, op::DIVRK) => {
                self.read(pcc, cc, pc)?;
                let text = k(self, cb as i64);
                self.constant(pb, text)?;
                self.write(pa, ca)
            }
            (op::GETTABLE, op::GETTABLEKS | op::GETUDATAKS) => {
                self.read(pb, cb, pc)?;
                let text = k(self, c.aux as i64);
                self.constant(pcc, text)?;
                self.write(pa, ca)
            }
            (op::GETTABLE, op::GETTABLEN) => {
                self.read(pb, cb, pc)?;
                self.constant(pcc, (cc + 1).to_string())?;
                self.write(pa, ca)
            }
            (op::SETTABLE, op::SETTABLEKS | op::SETUDATAKS) => {
                self.read(pa, ca, pc)?;
                self.read(pb, cb, pc)?;
                let text = k(self, c.aux as i64);
                self.constant(pcc, text)
            }
            (op::SETTABLE, op::SETTABLEN) => {
                self.read(pa, ca, pc)?;
                self.read(pb, cb, pc)?;
                self.constant(pcc, (cc + 1).to_string())
            }
            (op::JUMPIFEQ | op::JUMPIFNOTEQ, op::JUMPXEQKNIL | op::JUMPXEQKB | op::JUMPXEQKN | op::JUMPXEQKS) => {
                // JUMPXEQK* has the sense of `jump if equal`, with a flag for not
                let not = c.aux >> 31 != 0;
                if not != (p.op == op::JUMPIFNOTEQ) {
                    return None;
                }
                let text = match c.op {
                    op::JUMPXEQKNIL => "nil".to_string(),
                    op::JUMPXEQKB => if c.aux & 1 != 0 { "true" } else { "false" }.to_string(),
                    _ => k(self, (c.aux & 0xFFFFFF) as i64),
                };
                let other = (p.aux & 0xFF) as usize;
                let param = |w: &Self, r: usize| r < w.helper.numparams && w.map[r].is_none();
                if param(self, other) || !param(self, pa) {
                    self.read(pa, ca, pc)?;
                    self.constant(other, text)?;
                } else {
                    self.read(other, ca, pc)?;
                    self.constant(pa, text)?;
                }
                self.jump(p, c)
            }
            (op::SETUPVAL, op::MOVE) => {
                self.upvalue(pb, Up::Reg(ca))?;
                self.read(pa, cb, pc)
            }
            (op::MOVE, _) => {
                let text = self.load_text(c)?;
                self.constant(pb, text.clone())?;
                self.write(pa, ca)?;
                self.loaded.insert(ca, (text, c.pc));
                Some(())
            }
            _ => None,
        }
    }

    /// `"Missing " .. name` with name constant: the compiler folded the string,
    /// so the helper's load, move and concat are one load in the caller.
    /// Returns how many of the helper's instructions that covers.
    fn folded_concat(&mut self, want: &[&Insn], k: usize, c: &Insn) -> Option<usize> {
        let mut text = Vec::new();
        for (n, i) in want[k..].iter().enumerate().take(8) {
            match i.op {
                op::LOADK => text.push(kexpr(self.m, self.helper, i.d() as i64)),
                op::MOVE => match self.args.get(i.b()) {
                    Some(Some(Arg::Const(t))) => text.push(t.clone()),
                    _ => return None,
                },
                op::CONCAT => {
                    if n != i.c() - i.b() + 1 {
                        return None;
                    }
                    let joined: Option<String> =
                        text.iter().map(|t| t.strip_prefix('"')?.strip_suffix('"').map(String::from)).collect();
                    let whole = format!("\"{}\"", joined?);
                    (self.load_text(c)? == whole).then_some(())?;
                    self.write(i.a(), c.a())?;
                    return Some(n + 1);
                }
                _ => return None,
            }
        }
        None
    }

    /// `{ name = name }` with name given a constant: the compiler put that
    /// constant into the copy's template and stores nothing. The keys are the
    /// same, in order; each key only the copy's template gives a value is
    /// stored by the helper, from a parameter, before anything but stores
    /// into the table is done with it: that parameter is the constant. The
    /// helper's stores the copy has none of, as indices into want.
    fn template_args(&mut self, want: &[&Insn], k: usize, p: &Insn, c: &Insn) -> Option<Vec<usize>> {
        let (K::TableK(hk), K::TableK(ck)) = (self.helper.k.get(p.d() as usize)?, self.caller.k.get(c.d() as usize)?) else {
            return None;
        };
        if hk.len() != ck.len() {
            return None;
        }
        let table = p.a();
        let mut skip = Vec::new();
        for (&(hkey, hval), &(ckey, cval)) in hk.iter().zip(ck) {
            let key = kexpr(self.m, self.helper, hkey as i64);
            if key != kexpr(self.m, self.caller, ckey as i64) {
                return None;
            }
            match (hval >= 0, cval >= 0) {
                (true, true) => self.same_k(hval as i64, cval as i64)?,
                (false, false) => {}
                (true, false) => return None,
                (false, true) => {
                    let at = k + 1 + want[k + 1..].iter().position(|w| {
                        plain(w.op) == op::SETTABLEKS && w.b() == table && kexpr(self.m, self.helper, w.aux as i64) == key
                    })?;
                    // the table is still being made up to there
                    let only_stores = want[k + 1..at].iter().all(|w| {
                        let store = matches!(plain(w.op), op::SETTABLEKS | op::SETTABLE | op::SETTABLEN) && w.b() == table;
                        let (reads, writes) = full_effects(w, self.helper.maxstack);
                        store || (!reads.has(table) && !writes.has(table))
                    });
                    if !only_stores || skip.contains(&at) {
                        return None;
                    }
                    let text = kexpr(self.m, self.caller, cval as i64);
                    let src = want[at].a();
                    if src < self.helper.numparams {
                        self.constant(src, text)?;
                    } else {
                        // `face or size` of two parameters, folded with the
                        // arguments: size's value means face was left out
                        let or = want[at - 1];
                        let (a, b) = (or.b(), or.c());
                        let params = a < self.helper.numparams && b < self.helper.numparams;
                        if plain(or.op) != op::OR || or.a() != src || at - 1 <= k || !params {
                            return None;
                        }
                        let Some(Some(Arg::Const(other))) = self.args.get(b).cloned() else { return None };
                        if text == other {
                            self.constant(a, "nil".to_string())?;
                            self.omitted[a] = true;
                        } else if falsy(&text) {
                            return None;
                        } else {
                            self.constant(a, text)?;
                        }
                        skip.push(at - 1);
                    }
                    skip.push(at);
                }
            }
        }
        (!skip.is_empty()).then_some(skip)
    }

    /// The same function, in both protos?
    fn same_closure(&self, p: &Insn, c: &Insn) -> Option<()> {
        let child = |x: &Proto, i: &Insn| -> Option<usize> {
            if i.op == op::NEWCLOSURE {
                return x.children.get(i.d() as usize).copied();
            }
            match x.k.get(i.d() as usize)? {
                K::Closure(c) => Some(*c),
                _ => None,
            }
        };
        (child(self.helper, p)? == child(self.caller, c)?).then_some(())
    }

    fn same_k(&self, p: i64, c: i64) -> Option<()> {
        (kexpr(self.m, self.helper, p) == kexpr(self.m, self.caller, c)).then_some(())
    }

    fn jump(&mut self, p: &Insn, c: &Insn) -> Option<()> {
        self.jumps.push((p.jump_target()?, c.jump_target()?));
        Some(())
    }

    /// Reads of `count` registers from p and c.
    fn range(&mut self, p: usize, c: usize, count: usize, pc: i32) -> Option<()> {
        for i in 0..count {
            self.read(p + i, c + i, pc)?;
        }
        Some(())
    }

    fn step(&mut self, p: &Insn, c: &Insn) -> Option<()> {
        let (pa, pb, pcc) = (p.a(), p.b(), p.c());
        let (ca, cb, cc) = (c.a(), c.b(), c.c());
        let pc = c.pc;
        match plain(p.op) {
            op::LOADNIL => self.write(pa, ca),
            op::LOADB => {
                if (pb, pcc) != (cb, cc) {
                    return None;
                }
                if pcc != 0 {
                    self.jumps.push((p.pc + 1 + pcc as i32, c.pc + 1 + cc as i32));
                }
                self.write(pa, ca)
            }
            op::LOADN => (p.d() == c.d()).then_some(())?.then_write(self, pa, ca),
            op::LOADK => {
                let (x, y) = (
                    if p.op == op::LOADKX { p.aux as i64 } else { p.d() as i64 },
                    if c.op == op::LOADKX { c.aux as i64 } else { c.d() as i64 },
                );
                self.same_k(x, y)?;
                self.write(pa, ca)
            }
            op::MOVE | op::NOT | op::MINUS | op::LENGTH => {
                self.read(pb, cb, pc)?;
                self.write(pa, ca)
            }
            op::GETGLOBAL => {
                self.same_k(p.aux as i64, c.aux as i64)?;
                self.write(pa, ca)
            }
            op::SETGLOBAL => {
                self.same_k(p.aux as i64, c.aux as i64)?;
                self.read(pa, ca, pc)
            }
            op::SETUPVAL => {
                self.upvalue(pb, Up::Up(cb))?;
                self.read(pa, ca, pc)
            }
            op::GETIMPORT => {
                (import_expr(self.m, self.helper, p.aux) == import_expr(self.m, self.caller, c.aux)).then_some(())?;
                self.write(pa, ca)
            }
            op::GETTABLE | op::ADD | op::SUB | op::MUL | op::DIV | op::MOD | op::POW | op::IDIV | op::AND
            | op::OR => {
                self.read(pb, cb, pc)?;
                self.read(pcc, cc, pc)?;
                self.write(pa, ca)
            }
            op::SETTABLE => {
                self.read(pa, ca, pc)?;
                self.read(pb, cb, pc)?;
                self.read(pcc, cc, pc)
            }
            op::GETTABLEKS => {
                self.read(pb, cb, pc)?;
                self.same_k(p.aux as i64, c.aux as i64)?;
                self.write(pa, ca)
            }
            op::SETTABLEKS => {
                self.read(pa, ca, pc)?;
                self.read(pb, cb, pc)?;
                self.same_k(p.aux as i64, c.aux as i64)
            }
            op::GETTABLEN => {
                (pcc == cc).then_some(())?;
                self.read(pb, cb, pc)?;
                self.write(pa, ca)
            }
            op::SETTABLEN => {
                (pcc == cc).then_some(())?;
                self.read(pa, ca, pc)?;
                self.read(pb, cb, pc)
            }
            op::NAMECALL => {
                self.read(pb, cb, pc)?;
                self.same_k(p.aux as i64, c.aux as i64)?;
                self.write(pa, ca)?;
                self.write(pa + 1, ca + 1)
            }
            op::NEWCLOSURE | op::DUPCLOSURE => {
                self.same_closure(p, c)?;
                self.write(pa, ca)
            }
            op::CAPTURE => {
                if pa != 2 && ca != 2 {
                    self.read(pb, cb, pc)?;
                }
                Some(())
            }
            op::CALL => {
                (pb == cb).then_some(())?;
                if pb != 0 {
                    self.range(pa, ca, pb, pc)?;
                } else {
                    // up to the last open list, which sits in a register above
                    self.read(pa, ca, pc)?;
                    let mut i = 1;
                    while self.map.get(pa + i).is_some_and(|m| m.is_some()) {
                        self.read(pa + i, ca + i, pc)?;
                        i += 1;
                    }
                }
                if pcc == 0 {
                    // `return f(x)` keeps all results; inlined, the caller says how many
                    (cc == 0 || cc == 2).then_some(())?;
                    return self.write(pa, ca);
                }
                (pcc == cc).then_some(())?;
                for i in 0..pcc - 1 {
                    self.write(pa + i, ca + i)?;
                }
                Some(())
            }
            op::ADDK | op::SUBK | op::MULK | op::DIVK | op::MODK | op::POWK | op::IDIVK | op::ANDK | op::ORK => {
                self.read(pb, cb, pc)?;
                self.same_k(pcc as i64, cc as i64)?;
                self.write(pa, ca)
            }
            op::SUBRK | op::DIVRK => {
                self.read(pcc, cc, pc)?;
                self.same_k(pb as i64, cb as i64)?;
                self.write(pa, ca)
            }
            op::CONCAT => {
                (pcc - pb == cc - cb).then_some(())?;
                self.range(pb, cb, pcc - pb + 1, pc)?;
                self.write(pa, ca)
            }
            op::NEWTABLE => {
                ((pb, p.aux) == (cb, c.aux)).then_some(())?;
                self.write(pa, ca)
            }
            op::DUPTABLE => {
                self.same_k(p.d() as i64, c.d() as i64)?;
                self.write(pa, ca)
            }
            op::SETLIST => {
                ((pcc, p.aux) == (cc, c.aux)).then_some(())?;
                self.read(pa, ca, pc)?;
                if pcc == 0 {
                    // up to the last open list, which sits in a register above
                    self.read(pb, cb, pc)?;
                    let mut i = 1;
                    while self.map.get(pb + i).is_some_and(|m| m.is_some()) {
                        self.read(pb + i, cb + i, pc)?;
                        i += 1;
                    }
                    return Some(());
                }
                self.range(pb, cb, pcc - 1, pc)
            }
            op::JUMP => self.jump(p, c),
            op::JUMPIF | op::JUMPIFNOT => {
                self.read(pa, ca, pc)?;
                self.jump(p, c)
            }
            op::JUMPIFEQ | op::JUMPIFLE | op::JUMPIFLT | op::JUMPIFNOTEQ | op::JUMPIFNOTLE | op::JUMPIFNOTLT => {
                self.read(pa, ca, pc)?;
                self.read((p.aux & 0xFF) as usize, (c.aux & 0xFF) as usize, pc)?;
                self.jump(p, c)
            }
            op::JUMPXEQKNIL | op::JUMPXEQKB => {
                (p.aux == c.aux).then_some(())?;
                self.read(pa, ca, pc)?;
                self.jump(p, c)
            }
            op::JUMPXEQKN | op::JUMPXEQKS => {
                (p.aux >> 31 == c.aux >> 31).then_some(())?;
                self.same_k((p.aux & 0xFFFFFF) as i64, (c.aux & 0xFFFFFF) as i64)?;
                self.read(pa, ca, pc)?;
                self.jump(p, c)
            }
            op::FORNPREP => {
                self.range(pa, ca, 3, pc)?;
                self.jump(p, c)
            }
            op::FORNLOOP => {
                self.range(pa, ca, 3, pc)?;
                self.write(pa + 2, ca + 2)?;
                self.jump(p, c)
            }
            op::FORGLOOP => {
                (p.aux == c.aux).then_some(())?;
                self.range(pa, ca, 3, pc)?;
                for i in 0..((p.aux & 0xFF) as usize).max(1) {
                    self.write(pa + 3 + i, ca + 3 + i)?;
                }
                self.jump(p, c)
            }
            o if is_forgprep(o) => {
                self.range(pa, ca, 3, pc)?;
                // the body reads what FORGLOOP, further down, writes
                for i in 3..3 + 8 {
                    *self.map.get_mut(pa + i)? = Some(ca + i);
                }
                self.jump(p, c)
            }
            // the helper's locals a closure captured end with it
            op::CLOSEUPVALS => (self.map.get(pa).copied().flatten() == Some(ca)).then_some(()),
            _ => None, // closures, varargs and the like: leave it
        }
    }
}

trait ThenWrite {
    fn then_write(self, w: &mut Walk<'_>, p: usize, c: usize) -> Option<()>;
}

impl ThenWrite for () {
    fn then_write(self, w: &mut Walk<'_>, p: usize, c: usize) -> Option<()> {
        w.write(p, c)
    }
}

impl Inlined {
    pub fn holds_result(&self, r: usize) -> bool {
        self.result.is_some_and(|t| t <= r && r < t + self.results)
    }
}

/// Is the code of `helper` inlined at index `start` of the caller's code?
/// A copy given constants has the tests on them folded: a parameter tested
/// before the copy shows what it is is tried as false, then as true.
pub fn match_call(m: &Module, caller: &Proto, id: usize, start: usize) -> Option<Inlined> {
    let mut assumed = vec![None; m.protos[id].numparams];
    // the parameters assumed, in the order tests needed them
    let mut taken: Vec<usize> = Vec::new();
    for _ in 0..16 {
        let (found, need) = match_assuming(m, caller, id, start, &assumed);
        if found.is_some() {
            return found;
        }
        if let Some(r) = need.filter(|r| assumed[*r].is_none() && taken.len() < 3) {
            assumed[r] = Some(false);
            taken.push(r);
            continue;
        }
        // that assumption failed: the last one false becomes true, those
        // true already are dropped
        loop {
            let r = *taken.last()?;
            if assumed[r] == Some(false) {
                assumed[r] = Some(true);
                break;
            }
            assumed[r] = None;
            taken.pop();
        }
    }
    None
}

fn match_assuming(m: &Module, caller: &Proto, id: usize, start: usize, assumed: &[Option<bool>]) -> (Option<Inlined>, Option<usize>) {
    let mut need = None;
    let found = match_once(m, caller, id, start, assumed, &mut need);
    (found, need)
}

fn match_once(m: &Module, caller: &Proto, id: usize, start: usize, assumed: &[Option<bool>], need: &mut Option<usize>) -> Option<Inlined> {
    let helper = &m.protos[id];
    CHECK.with(|c| c.set(""));
    let want: Vec<&Insn> = helper.code.iter().filter(|i| !ignored(i.op)).collect();
    if want.len() < 2 {
        return None; // an empty function leaves nothing to find
    }
    let code = &caller.code;
    let mut walk = Walk {
        m,
        helper,
        caller,
        map: vec![None; 512],
        args: vec![None; helper.numparams],
        written: Vec::new(),
        jumps: Vec::new(),
        loaded: HashMap::default(),
        upvals: vec![None; helper.nups],
        unread: HashMap::default(),
        held: HashSet::default(),
        assumed: assumed.to_vec(),
        need: None,
        upval_writes: Vec::new(),
        assigned: (0..helper.numparams)
            .map(|r| helper.code.iter().any(|i| !ignored(i.op) && full_effects(i, helper.maxstack).1.has(r)))
            .collect(),
        omitted: vec![false; helper.numparams],
    };
    // helper code a folded test left out of the copy: up to this pc
    let mut dead_until: Option<i32> = None;
    // the caller register a CLOSEUPVALS put after the result's moves closes
    let mut late_close: Option<usize> = None;
    // the jump over an else a folded test left out, and where it goes
    let mut dead_jump: Option<(i32, i32)> = None;
    let mut landing: HashMap<i32, i32> = HashMap::default();
    let mut to_end: Vec<i32> = Vec::new();
    let mut copied_return: Option<(usize, usize)> = None;
    // a return's store into the result: its pc, the pc after, helper register
    let mut stores: HashMap<i32, (i32, usize)> = HashMap::default();
    // the caller's way out of each return in the middle of the helper
    let mut exits: HashSet<i32> = HashSet::default();
    let mut result: Option<usize> = None;
    let mut results = 0;
    let mut unused = false;
    let mut i = skip(code, start);
    let first = code.get(i)?.pc;
    let mut last = first;
    let mut folded = 0;
    // the helper's stores into a constructor that the copy's template made
    let mut skipped: HashSet<usize> = HashSet::default();
    for (k, p) in want.iter().enumerate() {
        if folded > 0 {
            folded -= 1;
            continue;
        }
        if skipped.contains(&k) {
            continue;
        }
        if let Some(t) = dead_until {
            if p.pc < t {
                continue;
            }
            dead_until = None;
        }
        if dead_jump.is_some_and(|(at, _)| at == p.pc) {
            dead_until = dead_jump.take().map(|(_, to)| to);
            // a jump to it goes on where the copy does
            if let Some(c) = code.get(i) {
                landing.entry(p.pc).or_insert(c.pc);
            }
            continue;
        }
        // a test of a constant argument: no code in the copy, one way taken.
        // A parameter not known yet is a constant only if the copy does not
        // test something there itself.
        let tests = matches!(plain(p.op), op::JUMPIF | op::JUMPIFNOT | op::JUMPXEQKNIL | op::JUMPXEQKB | op::JUMPXEQKN | op::JUMPXEQKS);
        let constant = tests
            && p.a() < helper.numparams
            && (matches!(walk.args[p.a()], Some(Arg::Const(_))) || walk.assumed[p.a()].is_some());
        let tested_here = code.get(i).is_some_and(|c| plain(c.op) == plain(p.op));
        if tests && (constant || !tested_here) {
            let jumps = walk.folded_test(p);
            if walk.need.is_some() {
                *need = walk.need;
                return refuse("a test of a parameter not known yet");
            }
            if let Some(jumps) = jumps {
                // a jump to the test lands where the copy goes on
                if let Some(c) = code.get(i) {
                    landing.entry(p.pc).or_insert(c.pc);
                }
                let target = p.jump_target()?;
                if jumps {
                    dead_until = Some(target);
                } else {
                    // `if c then A else B end`: A ends with a jump over B
                    let before = want.iter().take_while(|w| w.pc < target).last();
                    if let Some(j) = before.filter(|j| plain(j.op) == op::JUMP && j.pc > p.pc) {
                        let to = j.jump_target()?;
                        if to > target {
                            dead_jump = Some((j.pc, to));
                        }
                    }
                }
                continue;
            }
        }
        let mut c = code.get(i)?;
        REACHED.with(|r| r.set((k, want.len(), p.op, c.op)));
        landing.insert(p.pc, c.pc);
        // `return a, b` of locals a closure captured: the copy moves the
        // results out first and closes the locals after
        if p.op == op::CLOSEUPVALS && c.op != op::CLOSEUPVALS && want.get(k + 1).is_some_and(|w| w.op == op::RETURN) {
            late_close = Some(walk.map.get(p.a()).copied().flatten()?);
            continue;
        }
        if p.op == op::LOADK {
            if let Some(n) = walk.folded_concat(&want, k, c) {
                folded = n - 1;
                last = c.pc;
                i = skip(code, i + 1);
                continue;
            }
        }
        // a value nobody asked for: the compiler left its load out
        let simple = matches!(plain(p.op), op::MOVE | op::LOADN | op::LOADB | op::LOADK | op::LOADNIL);
        let fits = plain(p.op) == plain(c.op) && (p.op != op::MOVE || walk.map[p.b()] == Some(c.b()));
        if simple && !fits && feeds_return(&want, k) {
            walk.map[p.a()] = if p.op == op::MOVE { walk.map[p.b()] } else { None };
            continue;
        }
        // a store into an upvalue that is a local where the copy is: the copy
        // made the value in that local, and has no store of its own
        if p.op == op::SETUPVAL {
            let src = walk.map[p.a()];
            let stored = c.op == op::SETUPVAL || (c.op == op::MOVE && Some(c.b()) == src);
            if let Some(r) = src.filter(|r| !stored && walk.written.contains(r)) {
                walk.upvalue(p.b(), Up::Reg(r))?;
                walk.upval_writes.push(r);
                continue;
            }
        }
        // constants the caller loads for the helper to read (an upvalue read
        // may have no instruction in the copy, and reads none of them)
        while plain(p.op) != plain(c.op)
            && p.op != op::RETURN
            && p.op != op::MOVE
            && p.op != op::GETUPVAL
            && p.op != op::SETUPVAL
        {
            if let Some(text) = walk.load_text(c) {
                walk.loaded.insert(c.a(), (text, c.pc));
                walk.written.push(c.a());
            } else if c.op == op::MOVE && walk.unread.len() == 1 {
                // an upvalue the copy reads from a local: moved into its
                // argument's place only just before the call (the call's
                // range read then shows it was that)
                let (&hr, &u) = walk.unread.iter().next()?;
                walk.upvalue(u, Up::Reg(c.b()))?;
                walk.unread.remove(&hr);
                walk.map[hr] = Some(c.a());
                walk.written.push(c.a());
            } else {
                break;
            }
            last = c.pc;
            i = skip(code, i + 1);
            c = code.get(i)?;
        }
        if p.op == op::RETURN {
            let finished = k + 1 == want.len();
            // B=0 returns an open list: `return f(x)`, one value once inlined
            match p.b() {
                1 => {
                    if c.op == op::LOADNIL && result.is_none_or(|r| r == c.a()) && (result.is_some() || !finished) {
                        result = Some(c.a());
                        last = c.pc;
                        i = skip(code, i + 1);
                    }
                }
                // `return value` of a parameter given a constant (or first read
                // here): the copy loads that constant where the result goes
                2 if p.a() < walk.helper.numparams
                    && walk.map[p.a()].is_none()
                    && result.is_none_or(|r| r == c.a())
                    && walk.load_text(c).is_some_and(|t| match walk.args.get(p.a()) {
                        Some(Some(Arg::Const(k))) => *k == t,
                        // taken for a constant of one truth, by a test before
                        Some(None) => walk.assumed[p.a()].is_none_or(|truthy| falsy(&t) != truthy),
                        _ => false,
                    }) =>
                {
                    let text = walk.load_text(c)?;
                    walk.args[p.a()] = Some(Arg::Const(text));
                    result = Some(c.a());
                    last = c.pc;
                    i = skip(code, i + 1);
                }
                _ if p.b() != 1 && walk.map[p.a()].is_none() && !walk.unread.contains_key(&p.a()) => {
                    if result.is_some() {
                        return refuse("a value is used and not used");
                    }
                    unused = true;
                }
                0 | 2 => {
                    if let Some(u) = walk.unread.remove(&p.a()) {
                        // `return upvalue`: the store reads the local
                        if c.op != op::MOVE {
                            return None;
                        }
                        walk.upvalue(u, Up::Reg(c.b()))?;
                        walk.map[p.a()] = Some(c.b());
                    }
                    let Some(value) = walk.map.get(p.a()).copied().flatten() else {
                        return refuse("returns a value the copy never made");
                    };
                    let inside = walk.written.contains(&value);
                    let leaves = code.get(skip(code, i + 1)).is_some_and(|e| matches!(plain(e.op), op::JUMP | op::RETURN));
                    let moved = c.op == op::MOVE && c.b() == value;
                    let stored = if moved && (!inside || result == Some(c.a()) || (!finished && leaves)) {
                        last = c.pc;
                        i = skip(code, i + 1);
                        stores.insert(c.pc, (code.get(i)?.pc, p.a()));
                        c.a()
                    } else if inside {
                        value
                    } else if matches!(walk.args.get(p.a()), Some(Some(Arg::Reg(r, _))) if *r == value) {
                        // `return v` of a parameter: the copy left its argument's
                        // register alone, and the caller reads the result there
                        value
                    } else {
                        return refuse("returns a local with no store");
                    };
                    if unused {
                        return refuse("a value is used and not used");
                    }
                    if result.is_some_and(|r| r != stored) {
                        return refuse("returns into two places");
                    }
                    result = Some(stored);
                }
                many => {
                    // `return a, b`: consecutive registers, as many as the
                    // caller asked for; the others were never computed
                    let mut stored = None;
                    let mut count = 0;
                    for n in 0..many - 1 {
                        let Some(value) = walk.map.get(p.a() + n).copied().flatten() else { break };
                        let e = code.get(i)?;
                        let next = stored.map(|s: usize| s + n);
                        if e.op == op::MOVE && e.b() == value && next.is_none_or(|s| s == e.a()) {
                            stored = stored.or(Some(e.a()));
                            last = e.pc;
                            i = skip(code, i + 1);
                        } else if walk.written.contains(&value) && next.is_none_or(|s| s == value) {
                            stored = stored.or(Some(value));
                        } else {
                            break;
                        }
                        count += 1;
                    }
                    let Some(stored) = stored else { return refuse("a first value not stored") };
                    if unused || result.is_some_and(|r| r != stored) || (results != 0 && results != count) {
                        return refuse("results in two places");
                    }
                    result = Some(stored);
                    results = count;
                }
            }
            if !finished {
                let e = code.get(i)?;
                match plain(e.op) {
                    op::JUMP => to_end.push(e.jump_target()?),
                    op::RETURN => copied_return = Some((e.a(), e.b())),
                    // the constants given folded every test on the way: this
                    // return is certain, and the compiler copies nothing of
                    // the helper after it, so the caller's own code follows
                    _ if walk.jumps.is_empty() && outside(helper, caller, e.pc) => break,
                    _ => return refuse("no way out after a return"),
                }
                exits.insert(e.pc);
                last = e.pc;
                i = skip(code, i + 1);
            }
            continue;
        }
        if p.op == op::GETUPVAL {
            match c.op {
                op::GETUPVAL => walk.upvalue(p.b(), Up::Up(c.b()))?,
                op::MOVE => walk.upvalue(p.b(), Up::Reg(c.b()))?,
                _ => {
                    // no instruction of its own: read where it is used
                    walk.map[p.a()] = None;
                    walk.unread.insert(p.a(), p.b());
                    continue;
                }
            }
            walk.write(p.a(), c.a())?;
        } else if p.op == op::DUPTABLE && c.op == op::DUPTABLE && walk.same_k(p.d() as i64, c.d() as i64).is_none() {
            skipped.extend(walk.template_args(&want, k, p, c)?);
            walk.write(p.a(), c.a())?;
        } else if plain(p.op) == plain(c.op) || (p.op == op::NEWCLOSURE && c.op == op::DUPCLOSURE) {
            // a literal that captures a parameter is new on every call; in a
            // copy given a function literal it is shared, and loaded as one
            walk.step(p, c)?;
        } else {
            walk.k_form(p, c)?;
        }
        last = c.pc;
        i = skip(code, i + 1);
        // the helper's local that it returns lives in the caller's result, which
        // is not the top of the stack: a call reaches it through a temporary
        if plain(p.op) == op::CALL && p.c() == 2 {
            let uses_it = want.get(k + 1).is_some_and(|w| matches!(w.op, op::MOVE | op::RETURN) && (w.a() == p.a() || w.b() == p.a()));
            if let Some(e) = code.get(i).filter(|e| e.op == op::MOVE && e.b() == c.a() && e.a() < c.a()) {
                if !uses_it {
                    walk.write(p.a(), e.a())?;
                    last = e.pc;
                    i = skip(code, i + 1);
                }
            }
        }
    }
    if let Some(r) = late_close {
        let close = code.get(i)?;
        if close.op != op::CLOSEUPVALS || close.a() != r {
            return refuse("captured locals not closed after the results");
        }
        last = close.pc;
        i = skip(code, i + 1);
    }
    let after = code.get(i)?;
    let end = after.pc;
    if end == first {
        return refuse("nothing of it is left to find");
    }
    // a value joined from two branches just before: the code from the join on
    // is the helper, and what came before is its argument
    let inside = |pc: i32| first <= pc && pc < end;
    if code.iter().any(|c| !inside(c.pc) && c.jump_target().is_some_and(|t| first < t && t < end)) {
        return refuse("a jump from outside lands inside");
    }
    // a branch's return may have gone through the store of the last one
    let last_return = landing.get(&want[want.len() - 1].pc).copied().unwrap_or(end);
    // two copies of the caller's last `return` are one way out
    let same_return = |t: i32| {
        let at = code.partition_point(|c| c.pc < t);
        after.op == op::RETURN && code.get(at).is_some_and(|c| c.op == op::RETURN && (c.a(), c.b()) == (after.a(), after.b()))
    };
    // the jumps went where the helper's did
    for (p, c) in &walk.jumps {
        let Some(&here) = landing.get(&land(&helper.code, *p)) else {
            return refuse("a jump lands where the copy has nothing");
        };
        let there = land(code, *c);
        // `local r = false; if x then r = f() end; return r`: where the jump
        // starts, r is already in the result
        let held = stores.get(&here).is_some_and(|(next, reg)| {
            land(code, *next) == there && result.is_some_and(|r| walk.held.contains(&(*reg, r)))
        });
        // the compiler sends a jump to a `return` straight to where that leads
        let left = exits.contains(&here) && (there == end || after_jump(after) == Some(there) || same_return(there));
        if here != there && !held && !left && !(here == end && after_jump(after) == Some(there)) {
            return refuse("a jump lands elsewhere");
        }
    }
    let reaches_end = |t: i32| t == end || t == last_return || after_jump(after) == Some(t) || same_return(t);
    if to_end.iter().any(|t| !reaches_end(land(code, *t))) {
        return refuse("a return goes elsewhere");
    }
    if let Some(r) = copied_return {
        if after.op != op::RETURN || (after.a(), after.b()) != r {
            return refuse("a copied return differs");
        }
    }
    if !(first <= last) {
        return None;
    }
    let results = if result.is_some() { results.max(1) } else { 0 };
    // what a parameter was taken for, when nothing of the copy read it
    let mut args = walk.args;
    for (r, a) in args.iter_mut().enumerate() {
        if a.is_none() {
            if let Some(truthy) = walk.assumed[r] {
                *a = Some(Arg::Const(if truthy { "true" } else { "false" }.to_string()));
            }
        }
    }
    Some(Inlined {
        helper: id,
        upvals: walk.upvals,
        args,
        end,
        last,
        result,
        results,
        written: walk.written,
        upval_writes: walk.upval_writes,
        omitted: walk.omitted,
    })
}

/// Does this load only set up a value of the `return` that follows it?
fn feeds_return(want: &[&Insn], k: usize) -> bool {
    let a = want[k].a();
    for w in &want[k..] {
        match plain(w.op) {
            op::MOVE | op::LOADN | op::LOADB | op::LOADK | op::LOADNIL => {}
            op::RETURN => return w.b() >= 2 && (w.a()..w.a() + w.b() - 1).contains(&a),
            _ => return false,
        }
    }
    false
}

/// Is the caller's instruction at pc on a line outside the helper's? (False
/// when there are no lines to tell by.)
fn outside(helper: &Proto, caller: &Proto, pc: i32) -> bool {
    let Some(&line) = caller.lines.get(pc as usize) else { return false };
    let (lo, hi) = helper
        .code
        .iter()
        .filter_map(|c| helper.lines.get(c.pc as usize).copied())
        .fold((u32::MAX, 0), |(lo, hi), l| (lo.min(l), hi.max(l)));
    lo <= hi && !(lo..=hi).contains(&line)
}

/// Where the code after a helper jumps straight on to, if it does.
fn after_jump(after: &Insn) -> Option<i32> {
    (plain(after.op) == op::JUMP).then(|| after.jump_target()).flatten()
}
