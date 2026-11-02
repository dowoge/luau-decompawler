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

use crate::bytecode::{is_for, is_forgprep, op, op_name, Insn, Module, Proto, K};
use crate::decomp::{import_expr, kexpr};

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
    /// Caller registers just loaded with a constant, as source text.
    loaded: HashMap<usize, String>,
    upvals: Vec<Option<Up>>,
    /// Helper registers holding an upvalue the caller has not been seen to read.
    unread: HashMap<usize, usize>,
    /// Every (helper register, caller register) a write has paired.
    held: HashSet<(usize, usize)>,
}

impl Walk<'_> {
    fn read(&mut self, p: usize, c: usize, pc: i32) -> Option<()> {
        if let Some(Some(Arg::Const(text))) = self.args.get(p) {
            return (self.loaded.get(&c) == Some(text)).then_some(());
        }
        if let Some(u) = self.unread.remove(&p) {
            self.upvalue(u, Up::Reg(c))?;
            self.map[p] = Some(c);
            return Some(());
        }
        match *self.map.get(p)? {
            Some(x) => (x == c).then_some(()),
            None if p < self.helper.numparams => {
                self.args[p] = Some(match self.loaded.get(&c) {
                    Some(text) => Arg::Const(text.clone()),
                    None => {
                        self.map[p] = Some(c);
                        Arg::Reg(c, pc)
                    }
                });
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
                self.loaded.insert(ca, text);
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
pub fn match_call(m: &Module, caller: &Proto, id: usize, start: usize) -> Option<Inlined> {
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
    };
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
    for (k, p) in want.iter().enumerate() {
        if folded > 0 {
            folded -= 1;
            continue;
        }
        let mut c = code.get(i)?;
        REACHED.with(|r| r.set((k, want.len(), p.op, c.op)));
        landing.insert(p.pc, c.pc);
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
        // constants the caller loads for the helper to read
        while plain(p.op) != plain(c.op) && p.op != op::RETURN && p.op != op::MOVE {
            let Some(text) = walk.load_text(c) else { break };
            walk.loaded.insert(c.a(), text);
            walk.written.push(c.a());
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
                        Some(None) => true,
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
                    let value = (*walk.map.get(p.a())?)?;
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
                    } else {
                        return refuse("returns a local with no store");
                    };
                    if unused || result.is_some_and(|r| r != stored) {
                        return None;
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
        } else if plain(p.op) == plain(c.op) {
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
        let here = *landing.get(&land(&helper.code, *p))?;
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
    if is_for(after.op) || !(first <= last) {
        return None;
    }
    let results = if result.is_some() { results.max(1) } else { 0 };
    Some(Inlined { helper: id, upvals: walk.upvals, args: walk.args, end, last, result, results, written: walk.written })
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

/// Where the code after a helper jumps straight on to, if it does.
fn after_jump(after: &Insn) -> Option<i32> {
    (plain(after.op) == op::JUMP).then(|| after.jump_target()).flatten()
}
