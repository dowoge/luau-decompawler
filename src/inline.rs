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
use crate::expr::{fmt_number, lua_string};

thread_local! {
    /// How far the last walk got: (instructions matched, of, helper's op, caller's op).
    static REACHED: Cell<(usize, usize, u8, u8)> = const { Cell::new((0, 0, 0, 0)) };
    /// The final check that refused, if the walk itself got through.
    static CHECK: Cell<&'static str> = const { Cell::new("") };
    /// Parameters the last walk had seen no value of, where it stopped (one
    /// bit each).
    static UNSEEN: Cell<u64> = const { Cell::new(0) };
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
    /// Helper registers holding a value Luau folded: worked out from
    /// constants the call gave (or the helper's own). The copy has no code for
    /// them; what reads one reads that value, loaded.
    known: HashMap<usize, Val>,
}

/// A number written as source text (a constant, or a LOADN's operand).
fn number(text: &str) -> Option<f64> {
    let t = text.trim();
    // a fraction the printer writes (`7 / 60`)
    if let Some((p, q)) = t.split_once(" / ") {
        return Some(number(p)? / number(q)?);
    }
    let digit = t.trim_start_matches('-').starts_with(|c: char| c.is_ascii_digit() || c == '.');
    if digit {
        t.parse::<f64>().ok()
    } else {
        None
    }
}

fn falsy(text: &str) -> bool {
    text == "nil" || text == "false"
}

/// A value Luau's constant folding knows, as the walk follows it.
#[derive(Clone, Debug)]
enum Val {
    Nil,
    Bool(bool),
    Num(f64),
    Str(Vec<u8>),
}

impl Val {
    fn truthy(&self) -> bool {
        !matches!(self, Val::Nil | Val::Bool(false))
    }

    fn num(&self) -> Option<f64> {
        match self {
            Val::Num(v) => Some(*v),
            _ => None,
        }
    }

    /// Luau's `==` on two constants (what a test folds to: -0 == 0).
    fn same(&self, o: &Val) -> bool {
        match (self, o) {
            (Val::Nil, Val::Nil) => true,
            (Val::Bool(a), Val::Bool(b)) => a == b,
            (Val::Num(a), Val::Num(b)) => a == b,
            (Val::Str(a), Val::Str(b)) => a == b,
            _ => false,
        }
    }

    /// The very same constant: what a load must give to be this value
    /// (-0 is not 0, which prints and divides otherwise).
    fn identical(&self, o: &Val) -> bool {
        match (self, o) {
            (Val::Num(a), Val::Num(b)) => a.to_bits() == b.to_bits(),
            _ => self.same(o),
        }
    }

    /// As the decompiler writes it.
    fn text(&self) -> String {
        match self {
            Val::Nil => "nil".to_string(),
            Val::Bool(b) => b.to_string(),
            Val::Num(v) => fmt_number(*v),
            Val::Str(s) => lua_string(s),
        }
    }

    /// The constant a text of the decompiler's stands for.
    fn parse(t: &str) -> Option<Val> {
        let pi = std::f64::consts::PI;
        Some(match t {
            "nil" => Val::Nil,
            "true" => Val::Bool(true),
            "false" => Val::Bool(false),
            "math.pi" => Val::Num(pi),
            "-math.pi" => Val::Num(-pi),
            "math.huge" | "1e999" => Val::Num(f64::INFINITY),
            "-math.huge" | "-1e999" => Val::Num(f64::NEG_INFINITY),
            "0/0" | "-(0/0)" => Val::Num(f64::NAN),
            _ if t.starts_with('"') => Val::Str(unquote(t)?),
            _ => Val::Num(number(t)?),
        })
    }

    fn of_k(p: &Proto, m: &Module, i: usize) -> Option<Val> {
        Some(match p.k.get(i)? {
            K::Nil => Val::Nil,
            K::Bool(b) => Val::Bool(*b),
            K::Number(v) => Val::Num(*v),
            K::Str(Some(s)) => Val::Str(m.strings.get(*s)?.to_vec()),
            K::Str(None) => Val::Str(Vec::new()),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_read_back_as_written() {
        for b in [&b"plain"[..], &[7, b'1', 0, 255, b'"', b'\\', b'\n', 200, b'9'], &[]] {
            assert_eq!(unquote(&lua_string(b)).as_deref(), Some(b));
        }
    }

    #[test]
    fn builtins_fold_as_luau_does() {
        let n = |v: f64| Val::Num(v);
        let num = |v: Option<Val>| v.and_then(|v| v.num());
        assert_eq!(num(builtin("math.abs", &[n(-3.0)])), Some(3.0));
        assert_eq!(num(builtin("math.max", &[n(1.0), n(5.0), n(2.0)])), Some(5.0));
        assert_eq!(num(builtin("math.clamp", &[n(9.0), n(0.0), n(4.0)])), Some(4.0));
        assert!(builtin("math.clamp", &[n(1.0), n(4.0), n(0.0)]).is_none());
        assert_eq!(num(builtin("string.len", &[Val::Str(b"abc".to_vec())])), Some(3.0));
        assert_eq!(num(builtin("bit32.arshift", &[n(-8.0), n(1.0)])), Some(f64::from(0xFFFF_FFFCu32)));
        assert!(matches!(builtin("type", &[Val::Nil]), Some(Val::Str(s)) if s == b"nil"));
        assert!(matches!(builtin("string.sub", &[Val::Str(b"hello".to_vec()), n(2.0), n(-2.0)]), Some(Val::Str(s)) if s == b"ell"));
        // not exact in every C library: left to the compiler
        assert!(builtin("math.sin", &[n(1.0)]).is_none());
    }

    #[test]
    fn negative_zero_is_its_own_constant() {
        assert!(Val::Num(0.0).same(&Val::Num(-0.0)));
        assert!(!Val::Num(0.0).identical(&Val::Num(-0.0)));
    }
}

/// The bytes of a string as lua_string writes it.
fn unquote(t: &str) -> Option<Vec<u8>> {
    let b = t.strip_prefix('"')?.strip_suffix('"')?.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'\\' {
            out.push(b[i]);
            i += 1;
            continue;
        }
        i += 1;
        match *b.get(i)? {
            b'\\' => out.push(b'\\'),
            b'"' => out.push(b'"'),
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            d if d.is_ascii_digit() => {
                // up to three digits, as Lua reads them
                let mut v: u32 = 0;
                let mut n = 0;
                while n < 3 && b.get(i).is_some_and(u8::is_ascii_digit) {
                    v = v * 10 + u32::from(b[i] - b'0');
                    i += 1;
                    n += 1;
                }
                out.push(u8::try_from(v).ok()?);
                continue;
            }
            _ => return None,
        }
        i += 1;
    }
    Some(out)
}

/// What Luau's builtin folding makes of a call on constants (BuiltinFolding):
/// only the functions it folds whose result is exact, so that this is the
/// very number the compiler computed. None where it would not fold.
fn builtin(name: &str, args: &[Val]) -> Option<Val> {
    let n = |i: usize| args.get(i).and_then(Val::num);
    let nums = || args.iter().map(Val::num).collect::<Option<Vec<f64>>>();
    let bits = |v: f64| (v as i64) as u32;
    let rad_deg = std::f64::consts::PI / 180.0;
    let str_arg = |i: usize| match args.get(i) {
        Some(Val::Str(s)) => Some(s),
        _ => None,
    };
    let num = |v: f64| Some(Val::Num(v));
    let count = args.len();
    match name {
        "math.abs" if count == 1 => num(n(0)?.abs()),
        "math.ceil" if count == 1 => num(n(0)?.ceil()),
        "math.floor" if count == 1 => num(n(0)?.floor()),
        "math.sqrt" if count == 1 => num(n(0)?.sqrt()),
        "math.fmod" if count == 2 => num(n(0)? % n(1)?),
        "math.deg" if count == 1 => num(n(0)? / rad_deg),
        "math.rad" if count == 1 => num(n(0)? * rad_deg),
        "math.round" if count == 1 => num(n(0)?.round()),
        "math.sign" if count == 1 => {
            let v = n(0)?;
            num(if v > 0.0 { 1.0 } else if v < 0.0 { -1.0 } else { 0.0 })
        }
        "math.max" | "math.min" if count >= 1 => {
            let v = nums()?;
            let max = name == "math.max";
            num(v[1..].iter().fold(v[0], |r, &a| if (max && a > r) || (!max && a < r) { a } else { r }))
        }
        "math.clamp" if count == 3 => {
            let (v, lo, hi) = (n(0)?, n(1)?, n(2)?);
            if !(lo <= hi) {
                return None;
            }
            let v = if v < lo { lo } else { v };
            num(if v > hi { hi } else { v })
        }
        "bit32.band" | "bit32.bor" | "bit32.bxor" | "bit32.btest" if count >= 1 => {
            let v = nums()?;
            let r = v[1..].iter().fold(bits(v[0]), |r, &a| match name {
                "bit32.bor" => r | bits(a),
                "bit32.bxor" => r ^ bits(a),
                _ => r & bits(a),
            });
            if name == "bit32.btest" {
                Some(Val::Bool(r != 0))
            } else {
                num(f64::from(r))
            }
        }
        "bit32.bnot" if count == 1 => num(f64::from(!bits(n(0)?))),
        "bit32.lshift" | "bit32.rshift" | "bit32.arshift" if count == 2 => {
            let (u, s) = (bits(n(0)?), n(1)? as i32);
            if !(0..32).contains(&s) {
                return None;
            }
            num(f64::from(match name {
                "bit32.lshift" => u << s,
                "bit32.rshift" => u >> s,
                _ => ((u as i32) >> s) as u32,
            }))
        }
        "bit32.lrotate" | "bit32.rrotate" if count == 2 => {
            let (u, s) = (bits(n(0)?), n(1)? as i32);
            let s = (s & 31) as u32;
            num(f64::from(if name == "bit32.lrotate" { u.rotate_left(s) } else { u.rotate_right(s) }))
        }
        "type" | "typeof" if count == 1 => Some(Val::Str(
            match &args[0] {
                Val::Nil => "nil",
                Val::Bool(_) => "boolean",
                Val::Num(_) => "number",
                Val::Str(_) => "string",
            }
            .as_bytes()
            .to_vec(),
        )),
        "string.len" if count == 1 => num(str_arg(0)?.len() as f64),
        "string.byte" if count == 1 => num(f64::from(*str_arg(0)?.first()?)),
        "string.byte" if count == 2 => {
            let (s, i) = (str_arg(0)?, n(1)? as i32);
            if i < 1 || i as usize > s.len() {
                return None;
            }
            num(f64::from(s[i as usize - 1]))
        }
        "string.sub" if count == 2 || count == 3 => {
            let s = str_arg(0)?;
            let len = s.len() as i32;
            let mut start = n(1)? as i32;
            let mut end = if count == 3 { n(2)? as i32 } else { len };
            if start < 0 {
                start += len + 1;
            }
            if end < 0 {
                end += len + 1;
            }
            if end < 1 {
                return Some(Val::Str(Vec::new()));
            }
            let start = start.max(1);
            let end = end.min(len);
            Some(Val::Str(if start <= end { s[start as usize - 1..end as usize].to_vec() } else { Vec::new() }))
        }
        "string.char" if count < 128 => {
            let v = nums()?;
            let bytes: Option<Vec<u8>> = v.iter().map(|&c| u8::try_from(c as i32).ok().filter(|b| f64::from(*b) == c.trunc())).collect();
            Some(Val::Str(bytes?))
        }
        _ => None,
    }
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
        if let Some(v) = self.known.get(&p) {
            // a folded value: the caller loaded it where it is read
            return self.loaded.get(&c).and_then(|l| Val::parse(&l.0)).is_some_and(|x| x.identical(v)).then_some(());
        }
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
        if let Some(v) = self.known.get(&p) {
            return Val::parse(&text).is_some_and(|x| x.identical(v)).then_some(()); // the K form of a folded value
        }
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

    /// What was in caller register c is gone: no helper register is there.
    fn clobber(&mut self, c: usize) {
        for m in self.map.iter_mut().filter(|m| **m == Some(c)) {
            *m = None;
        }
    }

    fn write(&mut self, p: usize, c: usize) -> Option<()> {
        self.unread.remove(&p);
        self.known.remove(&p);
        self.clobber(c);
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

    /// The value helper register r holds as Luau saw it at compile time: a
    /// folded one, or a parameter given a constant and never assigned.
    fn val(&self, r: usize) -> Option<Val> {
        if let Some(v) = self.known.get(&r) {
            return Some(v.clone());
        }
        if r < self.helper.numparams && !self.assigned[r] && self.map[r].is_none() {
            if let Some(Some(Arg::Const(t))) = self.args.get(r) {
                return Val::parse(t);
            }
        }
        None
    }

    fn value(&self, r: usize) -> Option<f64> {
        self.val(r)?.num()
    }

    fn helper_k(&self, i: i64) -> Option<Val> {
        Val::of_k(self.helper, self.m, usize::try_from(i).ok()?)
    }

    /// p's result when all it reads are known: what Luau's constant folding
    /// makes of it. A load is known too: the copy has none of it when what
    /// reads it was folded.
    fn fold(&self, p: &Insn) -> Option<Val> {
        let k = |i: i64| self.helper_k(i)?.num();
        let o = plain(p.op);
        match o {
            op::MINUS => return Some(Val::Num(-self.value(p.b())?)),
            op::NOT => return Some(Val::Bool(!self.val(p.b())?.truthy())),
            op::LENGTH => {
                return match self.val(p.b())? {
                    Val::Str(s) => Some(Val::Num(s.len() as f64)),
                    _ => None,
                };
            }
            op::MOVE => return self.val(p.b()),
            op::LOADN => return Some(Val::Num(f64::from(p.d()))),
            op::LOADK => return self.helper_k(if p.op == op::LOADKX { i64::from(p.aux) } else { i64::from(p.d()) }),
            op::LOADNIL => return Some(Val::Nil),
            op::LOADB if p.c() == 0 => return Some(Val::Bool(p.b() != 0)),
            op::AND | op::OR => {
                let a = self.val(p.b())?;
                return if a.truthy() == (o == op::AND) { self.val(p.c()) } else { Some(a) };
            }
            op::ANDK | op::ORK => {
                let a = self.val(p.b())?;
                return if a.truthy() == (o == op::ANDK) { self.helper_k(p.c() as i64) } else { Some(a) };
            }
            op::CONCAT => {
                let mut s = Vec::new();
                for r in p.b()..=p.c() {
                    match self.val(r)? {
                        Val::Str(x) => s.extend_from_slice(&x),
                        _ => return None,
                    }
                }
                return (s.len() <= 4096).then_some(Val::Str(s));
            }
            _ => {}
        }
        let (a, b) = match o {
            op::ADD | op::SUB | op::MUL | op::DIV | op::MOD | op::POW | op::IDIV => (self.value(p.b())?, self.value(p.c())?),
            op::ADDK | op::SUBK | op::MULK | op::DIVK | op::MODK | op::POWK | op::IDIVK => (self.value(p.b())?, k(p.c() as i64)?),
            op::SUBRK | op::DIVRK => (k(p.b() as i64)?, self.value(p.c())?),
            _ => return None,
        };
        Some(Val::Num(match o {
            op::ADD | op::ADDK => a + b,
            op::SUB | op::SUBK | op::SUBRK => a - b,
            op::MUL | op::MULK => a * b,
            op::DIV | op::DIVK | op::DIVRK => a / b,
            op::MOD | op::MODK => a - (a / b).floor() * b,
            op::POW | op::POWK => a.powf(b),
            _ => (a / b).floor(), // IDIV, IDIVK
        }))
    }

    /// A test of known values, folded: Some(true) if it jumps.
    fn known_test(&self, p: &Insn) -> Option<bool> {
        let o = plain(p.op);
        let not = p.aux >> 31 != 0;
        match o {
            op::JUMPIF | op::JUMPIFNOT => Some(self.known.get(&p.a())?.truthy() == (o == op::JUMPIF)),
            op::JUMPIFEQ | op::JUMPIFNOTEQ | op::JUMPIFLT | op::JUMPIFNOTLT | op::JUMPIFLE | op::JUMPIFNOTLE => {
                // (parameters given constants, never assigned, are constants
                // to the compiler too: `p > p` of `f(1)` is folded)
                let (x, y) = (self.val(p.a())?, self.val((p.aux & 0xFF) as usize)?);
                let holds = match o {
                    op::JUMPIFEQ | op::JUMPIFNOTEQ => x.same(&y),
                    // only numbers are ordered at compile time
                    op::JUMPIFLT | op::JUMPIFNOTLT => x.num()? < y.num()?,
                    _ => x.num()? <= y.num()?,
                };
                Some(holds != matches!(o, op::JUMPIFNOTEQ | op::JUMPIFNOTLT | op::JUMPIFNOTLE))
            }
            op::JUMPXEQKNIL => Some(matches!(self.known.get(&p.a())?, Val::Nil) != not),
            op::JUMPXEQKB => Some(matches!(self.known.get(&p.a())?, Val::Bool(b) if *b == (p.aux & 1 != 0)) != not),
            op::JUMPXEQKN | op::JUMPXEQKS => {
                let x = self.known.get(&p.a())?;
                let k = self.helper_k(i64::from(p.aux & 0xFFFFFF))?;
                Some(x.same(&k) != not)
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
/// before the copy shows what it is is tried as false, then as true. One
/// whose every use was folded left no trace: see `solve`.
pub fn match_call(m: &Module, caller: &Proto, id: usize, start: usize) -> Option<Inlined> {
    let none = vec![None; m.protos[id].numparams];
    if let Some(found) = match_given(m, caller, id, start, &none) {
        return Some(found);
    }
    let saved = (REACHED.with(Cell::get), CHECK.with(Cell::get));
    let unseen = UNSEEN.with(Cell::get);
    let found = solve(m, caller, id, start, saved.0 .0, unseen);
    if found.is_none() {
        // what LUAU_WHY reports is the plain walk's refusal
        REACHED.with(|r| r.set(saved.0));
        CHECK.with(|c| c.set(saved.1));
    }
    found
}

/// Arguments the copy left no trace of, every use of them folded
/// (`frames(13)` is the constant 13 / 60; in `f(1, 1)` a test on both is
/// gone). Each parameter the helper does not assign is tried with the
/// constants the copy holds, a few plain ones, and, where the helper's result
/// is its arithmetic on one parameter and the copy one constant, the number
/// that comes to it. The walk checks the whole copy against what the helper
/// makes of those: any set that gets through is a call compiling to this
/// very code, provided it is all the copy: given made-up constants nearly
/// all of a helper folds, and a few instructions would match, so what
/// follows has to be off the helper's lines. Tried only where the plain walk
/// got into the copy with a parameter it saw no value of, or that number
/// exists; at most 200 walks.
fn solve(m: &Module, caller: &Proto, id: usize, start: usize, reached: usize, unseen: u64) -> Option<Inlined> {
    let helper = &m.protos[id];
    let n = helper.numparams;
    let want = helper.code.iter().filter(|i| !ignored(i.op)).count();
    if n == 0 || n > 3 || want > 64 {
        return None;
    }
    if !fresh(m, caller, id, start, 4) {
        return None;
    }
    // all of the copy: its returns jump to where it ends, which says so (code
    // after it may keep its line: `(f(x) or 0)`); a copy without them ends
    // where the helper's lines stop
    let whole = |found: Inlined| -> Option<Inlined> {
        let code = &caller.code;
        let from = code.partition_point(|c| c.pc < code.get(start).map_or(0, |s| s.pc));
        let jumped = code[from..].iter().take_while(|c| c.pc < found.end).any(|c| c.jump_target() == Some(found.end));
        let next = code.get(code.partition_point(|c| c.pc < found.end));
        (jumped || next.is_none_or(|c| outside(helper, caller, c.pc))).then_some(found)
    };
    let code = &caller.code;
    let mut cands: Vec<Val> = Vec::new();
    let add = |v: Val, cands: &mut Vec<Val>| {
        if !matches!(v, Val::Num(x) if x.is_nan()) && !cands.iter().any(|c| c.identical(&v)) {
            cands.push(v);
        }
    };
    if let Some(found) = folded_whole(m, caller, id, start) {
        return Some(found);
    }
    if reached == 0 || unseen == 0 {
        return None;
    }
    let none = vec![None; n];
    let mut found: Vec<Val> = Vec::new();
    for c in code[start..].iter().filter(|c| !ignored(c.op)).take(want + 8) {
        for v in consts_of(m, caller, c) {
            add(v, &mut found);
        }
    }
    // the helper's arithmetic on a parameter and a constant, undone from the
    // copy's numbers (`p // 3` folded to 2 gives 6): first, being likeliest
    for ins in helper.code.iter().filter(|i| !ignored(i.op)) {
        let o = plain(ins.op);
        let (param, k) = match o {
            op::ADDK | op::SUBK | op::MULK | op::DIVK | op::IDIVK | op::MODK => (ins.b(), ins.c()),
            op::SUBRK | op::DIVRK => (ins.c(), ins.b()),
            _ => continue,
        };
        let Some(k) = (param < n).then(|| Val::of_k(helper, m, k).and_then(|v| v.num())).flatten() else { continue };
        for y in found.iter().filter_map(Val::num) {
            let x = match o {
                op::ADDK => y - k,
                op::SUBK => y + k,
                op::MULK if k != 0.0 => y / k,
                op::DIVK | op::IDIVK => y * k,
                op::MODK => y,
                op::SUBRK => k - y,
                op::DIVRK if y != 0.0 => k / y,
                _ => continue,
            };
            if x.is_finite() {
                add(Val::Num(x), &mut cands);
            }
        }
    }
    for v in found {
        add(v, &mut cands);
    }
    for v in [Val::Num(0.0), Val::Num(1.0), Val::Num(-1.0), Val::Num(2.0), Val::Bool(true), Val::Bool(false), Val::Nil] {
        add(v, &mut cands);
    }
    cands.truncate(16);
    let assigned = |r: usize| helper.code.iter().any(|i| !ignored(i.op) && full_effects(i, helper.maxstack).1.has(r));
    // only those the plain walk saw no value of: the others it paired
    let free: Vec<usize> = (0..n).filter(|r| !assigned(*r) && (*r >= 64 || unseen & (1 << r) != 0)).collect();
    if free.is_empty() {
        return None;
    }
    let mut tries = 0;
    // one parameter given, then two, then three; each a choice of candidates
    for size in 1..=free.len() {
        let mut pick: Vec<usize> = (0..size).collect();
        loop {
            let mut which = vec![0usize; size];
            loop {
                tries += 1;
                if tries > 200 {
                    return None;
                }
                let mut preset = vec![None; n];
                for (j, &p) in pick.iter().enumerate() {
                    preset[free[p]] = Some(cands[which[j]].clone());
                }
                let mut need = None;
                if let Some(found) = match_once(m, caller, id, start, &none, &preset, &mut need).and_then(whole) {
                    return Some(found);
                }
                // the next choice of candidates
                let mut j = size;
                while j > 0 && which[j - 1] + 1 == cands.len() {
                    which[j - 1] = 0;
                    j -= 1;
                }
                if j == 0 {
                    break;
                }
                which[j - 1] += 1;
            }
            // the next choice of parameters
            let mut j = size;
            while j > 0 && pick[j - 1] == free.len() - size + j - 1 {
                j -= 1;
            }
            if j == 0 {
                break;
            }
            pick[j - 1] += 1;
            for q in j..size {
                pick[q] = pick[q - 1] + 1;
            }
        }
    }
    None
}

/// The copy folded whole: one constant, the helper's arithmetic on one
/// parameter, and the number that comes to it (rounder ones first). Its
/// copy given a register starts with arithmetic, never a load, so a match of
/// just that load is all of a copy.
fn folded_whole(m: &Module, caller: &Proto, id: usize, start: usize) -> Option<Inlined> {
    let helper = &m.protos[id];
    let code = &caller.code;
    let at = skip(code, start);
    let (r, a, b) = linear_return(m, helper)?;
    let Val::Num(k) = load_val(m, caller, code.get(at)?)? else { return None };
    let x = (k - b) / a;
    if !x.is_finite() {
        return None;
    }
    let after = code.get(at + 1).map_or(i32::MAX, |c| c.pc);
    let none = vec![None; helper.numparams];
    for v in [x.round(), (x * 10.0).round() / 10.0, (x * 100.0).round() / 100.0, (x * 1000.0).round() / 1000.0, x] {
        let mut preset = vec![None; helper.numparams];
        preset[r] = Some(Val::Num(v));
        let mut need = None;
        if let Some(found) = match_once(m, caller, id, start, &none, &preset, &mut need).filter(|f| f.end == after) {
            return Some(found);
        }
    }
    None
}

/// Do the helper's lines begin at instruction i? Luau gives what it compiles
/// the line it last set: a constant just after a copy has the copy's line,
/// and looks like a copy folded to it. The code of a copy starts a line of
/// the helper's, so the instruction before is on other lines, or is itself
/// a whole folded copy of the helper (`f(1), f(2)`).
fn fresh(m: &Module, caller: &Proto, id: usize, i: usize, depth: usize) -> bool {
    let helper = &m.protos[id];
    let code = &caller.code;
    let Some(prev) = (0..i.min(code.len())).rev().find(|&j| !ignored(code[j].op)) else { return true };
    if outside(helper, caller, code[prev].pc) {
        return true;
    }
    depth > 0
        && load_val(m, caller, &code[prev]).is_some()
        && fresh(m, caller, id, prev, depth - 1)
        && folded_whole(m, caller, id, prev).is_some()
}

/// The value a caller's load of a constant gives.
fn load_val(m: &Module, caller: &Proto, c: &Insn) -> Option<Val> {
    match c.op {
        op::LOADN => Some(Val::Num(f64::from(c.d()))),
        op::LOADK => Val::of_k(caller, m, usize::try_from(c.d()).ok()?),
        op::LOADKX => Val::of_k(caller, m, c.aux as usize),
        op::LOADNIL => Some(Val::Nil),
        op::LOADB if c.c() == 0 => Some(Val::Bool(c.b() != 0)),
        _ => None,
    }
}

/// The constants a caller's instruction holds: what it loads, or its K.
fn consts_of(m: &Module, caller: &Proto, c: &Insn) -> Vec<Val> {
    let k = |i: usize| Val::of_k(caller, m, i);
    let one = match plain(c.op) {
        op::LOADN | op::LOADK | op::LOADNIL | op::LOADB => load_val(m, caller, c),
        op::ADDK | op::SUBK | op::MULK | op::DIVK | op::MODK | op::POWK | op::IDIVK | op::ANDK | op::ORK => k(c.c()),
        op::SUBRK | op::DIVRK => k(c.b()),
        op::JUMPXEQKN | op::JUMPXEQKS => k((c.aux & 0xFFFFFF) as usize),
        op::JUMPXEQKB => Some(Val::Bool(c.aux & 1 != 0)),
        op::JUMPXEQKNIL => Some(Val::Nil),
        op::GETTABLEN | op::SETTABLEN => Some(Val::Num((c.c() + 1) as f64)),
        _ => None,
    };
    one.into_iter().collect()
}

/// The helper's result when it is straight arithmetic on one parameter and
/// constants: that parameter, with a and b of `a * p + b`.
fn linear_return(m: &Module, helper: &Proto) -> Option<(usize, f64, f64)> {
    // a constant, or a * parameter + b
    #[derive(Clone, Copy)]
    enum L {
        K(f64),
        X(usize, f64, f64),
    }
    let k = |i: usize| Val::of_k(helper, m, i)?.num();
    let bin = |o: u8, x: L, y: L| -> Option<L> {
        use L::{K as C, X};
        Some(match (o, x, y) {
            (_, C(a), C(b)) => C(match o {
                op::ADD => a + b,
                op::SUB => a - b,
                op::MUL => a * b,
                _ => a / b,
            }),
            (op::ADD, X(r, a, b), C(c)) | (op::ADD, C(c), X(r, a, b)) => X(r, a, b + c),
            (op::SUB, X(r, a, b), C(c)) => X(r, a, b - c),
            (op::SUB, C(c), X(r, a, b)) => X(r, -a, c - b),
            (op::MUL, X(r, a, b), C(c)) | (op::MUL, C(c), X(r, a, b)) => X(r, a * c, b * c),
            (op::DIV, X(r, a, b), C(c)) if c != 0.0 => X(r, a / c, b / c),
            _ => return None,
        })
    };
    let mut regs: HashMap<usize, L> = HashMap::default();
    for r in 0..helper.numparams {
        regs.insert(r, L::X(r, 1.0, 0.0));
    }
    for ins in helper.code.iter().filter(|i| !ignored(i.op)) {
        let get = |r: usize| regs.get(&r).copied();
        let arith = |o: u8| match o {
            op::ADD | op::ADDK => Some(op::ADD),
            op::SUB | op::SUBK | op::SUBRK => Some(op::SUB),
            op::MUL | op::MULK => Some(op::MUL),
            op::DIV | op::DIVK | op::DIVRK => Some(op::DIV),
            _ => None,
        };
        let o = plain(ins.op);
        let v = match o {
            op::LOADN => L::K(f64::from(ins.d())),
            op::LOADK => L::K(k(usize::try_from(ins.d()).ok()?)?),
            op::MOVE => get(ins.b())?,
            op::MINUS => bin(op::MUL, get(ins.b())?, L::K(-1.0))?,
            op::ADD | op::SUB | op::MUL | op::DIV => bin(arith(o)?, get(ins.b())?, get(ins.c())?)?,
            op::ADDK | op::SUBK | op::MULK | op::DIVK => bin(arith(o)?, get(ins.b())?, L::K(k(ins.c())?))?,
            op::SUBRK | op::DIVRK => bin(arith(o)?, L::K(k(ins.b())?), get(ins.c())?)?,
            op::RETURN if ins.b() == 2 => {
                return match get(ins.a())? {
                    L::X(r, a, b) if a != 0.0 => Some((r, a, b)),
                    _ => None,
                };
            }
            _ => return None,
        };
        regs.insert(ins.a(), v);
    }
    None
}

/// match_call's walks, the truth of untold parameters assumed in turn.
fn match_given(m: &Module, caller: &Proto, id: usize, start: usize, preset: &[Option<Val>]) -> Option<Inlined> {
    let mut assumed = vec![None; m.protos[id].numparams];
    // the parameters assumed, in the order tests needed them
    let mut taken: Vec<usize> = Vec::new();
    for _ in 0..16 {
        let (found, need) = match_assuming(m, caller, id, start, &assumed, preset);
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

fn match_assuming(
    m: &Module,
    caller: &Proto,
    id: usize,
    start: usize,
    assumed: &[Option<bool>],
    preset: &[Option<Val>],
) -> (Option<Inlined>, Option<usize>) {
    let mut need = None;
    let found = match_once(m, caller, id, start, assumed, preset, &mut need);
    (found, need)
}

fn match_once(
    m: &Module,
    caller: &Proto,
    id: usize,
    start: usize,
    assumed: &[Option<bool>],
    preset: &[Option<Val>],
    need: &mut Option<usize>,
) -> Option<Inlined> {
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
        known: HashMap::default(),
        assigned: (0..helper.numparams)
            .map(|r| helper.code.iter().any(|i| !ignored(i.op) && full_effects(i, helper.maxstack).1.has(r)))
            .collect(),
        omitted: vec![false; helper.numparams],
    };
    // constants the call is taken to have given (see solve)
    for (r, v) in preset.iter().enumerate() {
        if let Some(v) = v {
            walk.args[r] = Some(Arg::Const(v.text()));
        }
    }
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
    // a return the compiler made of a jump to another: where the copy jumps
    // instead, the helper register returned, and the copy's register for it
    let mut twin_returns: Vec<(i32, usize, usize)> = Vec::new();
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
        // the other way round: the helper's own code has a copy of a function
        // literal the copy calls for real (past the compiler's inline depth)
        if let Some(back) = real_call(&walk, &want, k, caller, i) {
            for &(param, r, at) in &back.binds {
                match walk.args[param] {
                    None => {
                        walk.args[param] = Some(Arg::Reg(r, at));
                        walk.map[param] = Some(r);
                    }
                    Some(Arg::Reg(x, _)) if x == r => {}
                    _ => return refuse("a parameter given two values"),
                }
            }
            if let Some(hr) = back.result {
                walk.map[hr] = Some(back.target);
            }
            walk.written.extend(back.target..back.target + back.width);
            let at = code.get(i)?.pc;
            for w in &want[k..k + back.skip] {
                landing.insert(w.pc, at);
            }
            last = code[back.call_at].pc;
            i = skip(code, back.call_at + 1);
            folded = back.skip - 1;
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
        // a test of numbers folded from the arguments, the same way
        let known = if tested_here { None } else { walk.known_test(p) };
        if known.is_some() || tests && (constant || !tested_here) {
            let jumps = match known {
                Some(j) => Some(j),
                None => walk.folded_test(p),
            };
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
        // a builtin called on constants (`math.abs(p)` of `f(-1)`): folded,
        // neither the import nor the call is in the copy
        if p.op == op::GETIMPORT {
            let call = want.get(k + 1).copied().filter(|n| plain(n.op) == op::CALL && n.a() == p.a() && n.b() >= 1 && n.c() == 2);
            let c = code.get(i)?;
            let name = import_expr(m, helper, p.aux);
            let in_copy = c.op == op::GETIMPORT && import_expr(m, caller, c.aux) == name;
            if let Some(n) = call.filter(|_| !in_copy) {
                let given: Option<Vec<Val>> = (1..n.b()).map(|j| walk.val(p.a() + j)).collect();
                if let Some(v) = given.and_then(|a| builtin(&name, &a)) {
                    walk.known.insert(p.a(), v);
                    walk.map[p.a()] = None;
                    landing.entry(p.pc).or_insert(c.pc);
                    landing.entry(n.pc).or_insert(c.pc);
                    skipped.insert(k + 1);
                    continue;
                }
            }
        }
        // `a < b` as a value, the test folded: `LOADB r 0 +1; LOADB r 1` is
        // one load in the copy, or none when what reads it was folded too
        if plain(p.op) == op::LOADB && p.c() != 0 {
            let c = code.get(i)?;
            if c.op != op::LOADB || c.c() == 0 {
                if c.op == op::LOADB && c.b() == p.b() {
                    walk.write(p.a(), c.a())?;
                    landing.insert(p.pc, c.pc);
                    last = c.pc;
                    i = skip(code, i + 1);
                } else {
                    walk.known.insert(p.a(), Val::Bool(p.b() != 0));
                    walk.map[p.a()] = None;
                    landing.entry(p.pc).or_insert(c.pc);
                }
                dead_until = Some(p.pc + 1 + p.c() as i32);
                continue;
            }
        }
        // a value worked out from constants: folded, nothing of it in the
        // copy, and what reads the result reads that value. (A move the copy
        // has as a load of that very value, both going on alike, is the K form
        // of the move; going on otherwise, the load is of something folded
        // after it: `math.abs(p)` of `f(3)`, loaded for a test.)
        if let Some(v) = walk.fold(p) {
            let c = code.get(i)?;
            let alike = want.get(k + 1).zip(code.get(skip(code, i + 1))).is_none_or(|(w, d)| plain(w.op) == plain(d.op));
            let loads_it = p.op == op::MOVE && alike && walk.load_text(c).and_then(|t| Val::parse(&t)).is_some_and(|x| x.identical(&v));
            if plain(c.op) != plain(p.op) && !loads_it {
                walk.known.insert(p.a(), v);
                walk.map[p.a()] = None;
                landing.entry(p.pc).or_insert(c.pc);
                continue;
            }
        }
        let mut c = code.get(i)?;
        REACHED.with(|r| r.set((k, want.len(), p.op, c.op)));
        UNSEEN.with(|u| u.set(walk.args.iter().enumerate().filter(|(r, a)| a.is_none() && *r < 64).fold(0, |m, (r, _)| m | 1 << r)));
        landing.insert(p.pc, c.pc);
        // `f()` of a parameter given a local function: the compiler copied
        // that function's code in here too, so the call is a copy of it
        if let Some(nested) = nested_call(&walk, &want, k, caller, start, i) {
            for &(param, r, at) in &nested.binds {
                match walk.args[param] {
                    None => {
                        walk.args[param] = Some(Arg::Reg(r, at));
                        walk.map[param] = Some(r);
                    }
                    Some(Arg::Reg(x, _)) if x == r => {}
                    _ => return refuse("a parameter given two values"),
                }
            }
            for &(u, up) in &nested.ups {
                walk.upvalue(u, up)?;
            }
            let call = want[k + nested.skip];
            if call.c() != 1 {
                walk.map[call.a()] = nested.call.result;
            }
            // what it wrote through its own upvalues is the caller's locals,
            // written by the closure it was given: not the copy's scratch
            walk.written.extend(nested.call.written.iter().copied().filter(|r| !nested.call.upval_writes.contains(r)));
            walk.upval_writes.extend(nested.call.upval_writes.iter().copied());
            // its result was made in the copy too
            walk.written.extend(nested.call.result);
            for w in &want[k..=k + nested.skip] {
                landing.insert(w.pc, c.pc);
            }
            last = nested.call.last;
            i = skip(code, code.partition_point(|x| x.pc < nested.call.end));
            folded = nested.skip;
            continue;
        }
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
        // (a load of another constant is no load of this one: `return 5, 6`
        // for one value leaves the 6 out, and what follows may load a 5)
        let same_load = match (walk.fold(p), walk.load_text(c).and_then(|t| Val::parse(&t))) {
            // (any NaN loads as `0/0`, whatever its bits)
            (Some(Val::Num(a)), Some(Val::Num(b))) if a.is_nan() && b.is_nan() => true,
            (Some(a), Some(b)) => a.identical(&b),
            _ => true,
        };
        let fits = plain(p.op) == plain(c.op) && (p.op != op::MOVE || walk.map[p.b()] == Some(c.b())) && same_load;
        if simple && !fits && feeds_return(&want, k) {
            walk.map[p.a()] = if p.op == op::MOVE { walk.map[p.b()] } else { None };
            continue;
        }
        // a store into an upvalue that is a local where the copy is: the copy
        // made the value in that local, and has no store of its own
        if p.op == op::SETUPVAL {
            let src = walk.map[p.a()];
            // a move of the value into another register is no store into the
            // local (`n += 1 return n`: the move is the return's)
            let into_local = walk.upvals.get(p.b()).and_then(|u| u.as_ref()).is_none_or(|u| *u == Up::Reg(c.a()));
            let stored = c.op == op::SETUPVAL || (c.op == op::MOVE && Some(c.b()) == src && into_local);
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
                // a load over a helper value is not one for the helper to read
                walk.clobber(c.a());
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
                // `return x * 2` folded to a value: the copy loads it where
                // the result goes
                2 if walk.known.get(&p.a()).is_some_and(|v| walk.load_text(c).and_then(|t| Val::parse(&t)).is_some_and(|x| x.identical(v)))
                    && result.is_none_or(|r| r == c.a()) =>
                {
                    result = Some(c.a());
                    last = c.pc;
                    i = skip(code, i + 1);
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
                // `return p` of a parameter nothing read before: the copy moves
                // its argument (a local's register, or its own) into the result
                2 if p.a() < walk.helper.numparams
                    && walk.map[p.a()].is_none()
                    && walk.args[p.a()].is_none()
                    && walk.assumed[p.a()].is_none()
                    && !walk.assigned[p.a()]
                    && !walk.unread.contains_key(&p.a())
                    && c.op == op::MOVE
                    && !walk.written.contains(&c.b())
                    && result.is_none_or(|r| r == c.a()) =>
                {
                    walk.map[p.a()] = Some(c.b());
                    walk.args[p.a()] = Some(Arg::Reg(c.b(), c.pc));
                    result = Some(c.a());
                    last = c.pc;
                    i = skip(code, i + 1);
                    stores.insert(c.pc, (code.get(i)?.pc, p.a()));
                }
                // `return v` of a parameter nothing read: its argument was made
                // where the result goes (the compiler puts a returned local in
                // the caller's result register), by the caller's code just
                // before the copy; nothing of the copy touches that register
                2 if p.a() < walk.helper.numparams
                    && walk.map[p.a()].is_none()
                    && walk.args[p.a()].is_none()
                    && walk.assumed[p.a()].is_none()
                    && !walk.assigned[p.a()]
                    && !walk.unread.contains_key(&p.a())
                    && finished
                    && result.is_none()
                    && argument_before(walk.helper, caller, start).is_some_and(|(r, _)| {
                        !walk.written.contains(&r) && !walk.map.iter().flatten().any(|m| *m == r)
                    }) =>
                {
                    let (r, at) = argument_before(walk.helper, caller, start)?;
                    walk.map[p.a()] = Some(r);
                    walk.args[p.a()] = Some(Arg::Reg(r, at));
                    result = Some(r);
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
                    // `if a or true then return v end`: the helper's jump to
                    // `return v` became a second `return v`, the copy's is a
                    // jump to its store of the first
                    let twin = want.iter().enumerate().any(|(j, w)| j != k && w.op == op::RETURN && (w.a(), w.b()) == (p.a(), p.b()));
                    let to_store = c.jump_target().filter(|t| {
                        let s = code.get(code.partition_point(|e| e.pc < *t));
                        s.is_some_and(|s| s.pc == *t && s.op == op::MOVE && s.b() == value)
                    });
                    if let Some(t) = to_store.filter(|_| twin && !inside && !finished && c.op == op::JUMP) {
                        twin_returns.push((t, p.a(), value));
                        last = c.pc;
                        i = skip(code, i + 1);
                        continue;
                    }
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
                // `local l = f(); for _, v in l do`: the read of the local that
                // follows reads it in its new place, a move of its own
                let read_there = code.get(skip(code, i + 1)).is_some_and(|n| n.op == op::MOVE && n.b() == e.a());
                if !uses_it || read_there {
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
    // each jump for a twin return lands on the copy's store of that return
    for (t, reg, value) in &twin_returns {
        let store = code.get(code.partition_point(|e| e.pc < *t));
        let into_result = store.is_some_and(|s| s.op == op::MOVE && s.b() == *value && Some(s.a()) == result);
        if stores.get(t).is_none_or(|(_, r)| r != reg) || !into_result {
            return refuse("a twin return lands on no store of it");
        }
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

thread_local! {
    /// How deep nested_call and real_call have gone: copies within copies (the
    /// compiler copies in up to LuauCompileInlineDepth, 5, deep, and the
    /// functions given as arguments with them).
    static NESTED: Cell<u32> = const { Cell::new(0) };
}

/// A call of the helper's found in the copy as a copy of the function called:
/// the copy, how many of the helper's instructions after this one it covers
/// (the call last), and the parameters it shows the values of.
struct Nested {
    call: Inlined,
    skip: usize,
    binds: Vec<(usize, usize, i32)>,
    /// upvalues of the helper's that a capture shows: (helper upvalue, what)
    ups: Vec<(usize, Up)>,
}

/// A call of the helper's where the copy has the called function's code: the
/// compiler copied that in at the call site but not in the helper's own code.
///   `MOVE t, param; CALL t`: a parameter given a local function. Each
///   closure the caller made before the copy and left in its register is
///   tried there; exactly one may match.
///   `DUPCLOSURE t, G; MOVE t+1, x ...; CALL t`: a function literal called
///   with values (in the helper, with a parameter it could not fold).
/// The arguments must be the walk's for the helper's call (an unread
/// parameter of the helper takes the copy's), the result where it wants one.
fn nested_call(walk: &Walk<'_>, want: &[&Insn], k: usize, caller: &Proto, start: usize, i: usize) -> Option<Nested> {
    let code = &caller.code;
    if NESTED.with(Cell::get) >= 8 || code.get(i).is_none() {
        return None;
    }
    let p = want[k];
    if matches!(p.op, op::DUPCLOSURE | op::NEWCLOSURE) {
        // (a copy may start with a move: `return f()` of an upvalue f)
        return literal_call(walk, want, k, caller, i);
    }
    if code.get(i).is_none_or(|c| c.op == op::MOVE || plain(c.op) == op::CALL) {
        return None; // a plain call: the function moved into place
    }
    if p.op != op::MOVE || p.b() >= walk.helper.numparams || walk.assigned[p.b()] {
        return None;
    }
    let call = want.get(k + 1)?;
    if plain(call.op) != op::CALL || call.a() != p.a() || call.b() == 0 {
        return None;
    }
    let mut cands: Vec<(usize, i32, usize)> = Vec::new();
    for j in 0..start.min(code.len()) {
        let ins = &code[j];
        let f = match ins.op {
            op::NEWCLOSURE => caller.children.get(ins.d().max(0) as usize).copied(),
            op::DUPCLOSURE => match caller.k.get(ins.d().max(0) as usize) {
                Some(K::Closure(c)) => Some(*c),
                _ => None,
            },
            _ => None,
        };
        let Some(f) = f else { continue };
        let r = ins.a();
        let kept = !code[j + 1..start].iter().any(|x| full_effects(x, caller.maxstack).1.has(r));
        if kept && std::ptr::from_ref(&walk.m.protos[f]) != std::ptr::from_ref(walk.helper) {
            cands.push((r, ins.pc, f));
        }
    }
    if let Some(m) = walk.map[p.b()] {
        cands.retain(|x| x.0 == m);
    }
    let nargs = call.b() - 1;
    let mut found: Option<Nested> = None;
    for (r, at, f) in cands {
        NESTED.with(|n| n.set(n.get() + 1));
    let saved = (REACHED.with(Cell::get), CHECK.with(Cell::get));
        let got = match_call(walk.m, caller, f, i);
        NESTED.with(|n| n.set(n.get() - 1));
    REACHED.with(|r| r.set(saved.0));
    CHECK.with(|c| c.set(saved.1));
        let here = code[i].pc;
        let Some(n) = got.filter(|n| on_lines_of(walk.m, caller, here, n.end, f)) else { continue };
        // the helper's arguments, as the walk has them, are the copy's
        let args_fit = (0..nargs.max(n.args.len())).all(|a| {
            let mine = if a < nargs { walk.map.get(call.a() + 1 + a).copied().flatten() } else { None };
            match (n.args.get(a).cloned().flatten(), mine) {
                (Some(Arg::Reg(x, _)), Some(y)) => x == y,
                (None, None) => true,
                (None, Some(_)) => n.omitted.get(a).copied().unwrap_or(false),
                _ => false,
            }
        });
        let result_fits = match call.c() {
            1 => true,
            2 | 0 => n.result.is_some() && n.results == 1,
            _ => false,
        };
        if !args_fit || !result_fits {
            continue;
        }
        if found.is_some() {
            return None; // two functions would do: neither is known to be it
        }
        found = Some(Nested { call: n, skip: 1, binds: vec![(p.b(), r, at)], ups: Vec::new() });
    }
    found
}

/// A real call in the copy of a function the helper's own code has a copy
/// of: the call's register and its width, where the call is (an index), how
/// many of the helper's instructions the helper's copy covers, the helper
/// register its result is in, and the parameters it shows the values of.
struct RealCall {
    target: usize,
    width: usize,
    call_at: usize,
    skip: usize,
    result: Option<usize>,
    binds: Vec<(usize, usize, i32)>,
}

/// `DUPCLOSURE t, G; MOVE t+1, x ... (or constants); CALL t` in the copy,
/// where the helper's code at this point is a copy of G: the compiler stops
/// copying in at a depth (LuauCompileInlineDepth), so the deeper the copy,
/// the more it keeps as calls that the helper, copied from less deep, has
/// copied in. G's copy is found in the helper's code as any copy is; its
/// arguments must be the call's, as the walk has them.
fn real_call(walk: &Walk<'_>, want: &[&Insn], k: usize, caller: &Proto, i: usize) -> Option<RealCall> {
    let code = &caller.code;
    let c = code.get(i)?;
    let g = match c.op {
        op::NEWCLOSURE => caller.children.get(c.d().max(0) as usize).copied()?,
        op::DUPCLOSURE => match caller.k.get(c.d().max(0) as usize)? {
            K::Closure(x) => *x,
            _ => return None,
        },
        _ => return None,
    };
    let p = want[k];
    let gp = &walk.m.protos[g];
    if matches!(p.op, op::NEWCLOSURE | op::DUPCLOSURE) || std::ptr::from_ref(gp) == std::ptr::from_ref(walk.helper) {
        return None;
    }
    if NESTED.with(Cell::get) >= 8 {
        return None;
    }
    let t = c.a();
    // what the closure captures: caller registers, by value or reference
    let caps: Vec<&Insn> = code[i + 1..].iter().take_while(|x| x.op == op::CAPTURE).collect();
    if caps.len() != gp.nups || caps.iter().any(|x| x.a() == 2) {
        return None;
    }
    // the arguments, into the registers after the function
    let mut j = i + 1 + caps.len();
    let mut given: Vec<&Insn> = Vec::new();
    while let Some(x) = code.get(j) {
        if x.a() == t + 1 + given.len() && (x.op == op::MOVE || walk.load_text(x).is_some()) && x.jump_target().is_none() {
            given.push(x);
            j += 1;
        } else {
            break;
        }
    }
    let call = code.get(j)?;
    if plain(call.op) != op::CALL || call.a() != t || call.b() == 0 || call.b() - 1 != given.len() || given.len() > gp.numparams {
        return None;
    }
    let helper = walk.helper;
    let hidx = helper.code.partition_point(|x| x.pc < p.pc);
    NESTED.with(|n| n.set(n.get() + 1));
    let saved = (REACHED.with(Cell::get), CHECK.with(Cell::get));
    let got = match_call(walk.m, helper, g, hidx);
    NESTED.with(|n| n.set(n.get() - 1));
    REACHED.with(|r| r.set(saved.0));
    CHECK.with(|c| c.set(saved.1));
    let hc = got.filter(|h| on_lines_of(walk.m, helper, p.pc, h.end, g))?;
    let mut binds = Vec::new();
    for a in 0..gp.numparams {
        let mine = hc.args.get(a).cloned().flatten();
        match (mine, given.get(a)) {
            (Some(Arg::Reg(hr, _)), Some(x)) if x.op == op::MOVE => match walk.map.get(hr).copied().flatten() {
                Some(cr) if cr == x.b() => {}
                None if hr < helper.numparams && walk.args[hr].is_none() && !walk.assigned[hr] => binds.push((hr, x.b(), x.pc)),
                _ => return None,
            },
            (Some(Arg::Const(text)), Some(x)) if walk.load_text(x).as_deref() == Some(text.as_str()) => {}
            (None, None) => {}
            _ => return None,
        }
    }
    // each captured register is what the helper's copy reads for that upvalue
    for (u, cap) in caps.iter().enumerate() {
        match hc.upvals.get(u).copied().flatten() {
            Some(Up::Reg(hr)) => match walk.map.get(hr).copied().flatten() {
                Some(cr) if cr == cap.b() => {}
                None if hr < helper.numparams && walk.args[hr].is_none() && !walk.assigned[hr] => {
                    if binds.iter().any(|b: &(usize, usize, i32)| b.0 == hr && b.1 != cap.b()) {
                        return None;
                    }
                    binds.push((hr, cap.b(), cap.pc));
                }
                _ => return None,
            },
            _ => return None,
        }
    }
    let result = match call.c() {
        1 => None,
        2 | 0 if hc.results == 1 => Some(hc.result?),
        _ => return None,
    };
    let skip = want[k..].iter().take_while(|w| w.pc < hc.end).count();
    if skip == 0 {
        return None;
    }
    Some(RealCall { target: t, width: (call.c() as usize).saturating_sub(1).max(1), call_at: j, skip, result, binds })
}

/// The `DUPCLOSURE t, G; MOVE t+1, x ...; CALL t` form of nested_call.
fn literal_call(walk: &Walk<'_>, want: &[&Insn], k: usize, caller: &Proto, i: usize) -> Option<Nested> {
    let helper = walk.helper;
    let p = want[k];
    let g = match p.op {
        op::NEWCLOSURE => helper.children.get(p.d().max(0) as usize).copied()?,
        _ => match helper.k.get(p.d().max(0) as usize)? {
            K::Closure(c) => *c,
            _ => return None,
        },
    };
    let t = p.a();
    let mut j = k + 1;
    // what the closure captures (as the helper's code says it)
    let mut caps: Vec<&Insn> = Vec::new();
    while let Some(w) = want.get(j).filter(|w| w.op == op::CAPTURE) {
        caps.push(w);
        j += 1;
    }
    let mut srcs: Vec<usize> = Vec::new();
    while let Some(w) = want.get(j) {
        if w.op == op::MOVE && w.a() == t + 1 + srcs.len() {
            srcs.push(w.b());
            j += 1;
        } else {
            break;
        }
    }
    let call = want.get(j)?;
    if plain(call.op) != op::CALL || call.a() != t || call.b() == 0 || call.b() - 1 != srcs.len() {
        return None;
    }
    NESTED.with(|n| n.set(n.get() + 1));
    let saved = (REACHED.with(Cell::get), CHECK.with(Cell::get));
    let got = match_call(walk.m, caller, g, i);
    NESTED.with(|n| n.set(n.get() - 1));
    REACHED.with(|r| r.set(saved.0));
    CHECK.with(|c| c.set(saved.1));
    let here = caller.code.get(i)?.pc;
    let n = got.filter(|n| on_lines_of(walk.m, caller, here, n.end, g))?;
    let mut binds = Vec::new();
    for (a, &src) in srcs.iter().enumerate() {
        let given = n.args.get(a).cloned().flatten();
        match (walk.map.get(src).copied().flatten(), given) {
            (Some(x), Some(Arg::Reg(y, _))) if x == y => {}
            // a parameter of the helper nothing read yet: what the copy shows
            (None, Some(Arg::Reg(y, at))) if src < helper.numparams && walk.args[src].is_none() && !walk.assigned[src] => {
                binds.push((src, y, at));
            }
            _ => return None,
        }
    }
    if n.args.len() > srcs.len() && n.args[srcs.len()..].iter().any(Option::is_some) {
        return None;
    }
    // each capture is what the copy reads for that upvalue
    let mut ups = Vec::new();
    for (u, cap) in caps.iter().enumerate() {
        let seen = n.upvals.get(u).copied().flatten()?;
        match cap.a() {
            2 => ups.push((cap.b(), seen)),
            _ => match (walk.map.get(cap.b()).copied().flatten(), seen) {
                (Some(x), Up::Reg(y)) if x == y => {}
                (None, Up::Reg(y)) if cap.b() < helper.numparams && walk.args[cap.b()].is_none() && !walk.assigned[cap.b()] => {
                    binds.push((cap.b(), y, caller.code.get(i)?.pc));
                }
                _ => return None,
            },
        }
    }
    let result_fits = match call.c() {
        1 => true,
        2 | 0 => n.result.is_some() && n.results == 1,
        _ => false,
    };
    result_fits.then_some(Nested { call: n, skip: j - k, binds, ups })
}

/// Is some instruction of `code_of` from pc `from` up to `to` on a line of
/// function `g`'s own? Copied-in code keeps the copied function's lines (and
/// copies within it theirs), so a copy found by the walk alone is a copy only
/// so: a lone constant elsewhere that the walk took for g is not. (True
/// without lines.)
fn on_lines_of(m: &Module, code_of: &Proto, from: i32, to: i32, g: usize) -> bool {
    let gp = &m.protos[g];
    let (lo, hi) = gp
        .code
        .iter()
        .filter_map(|c| gp.lines.get(c.pc as usize).copied())
        .fold((u32::MAX, 0), |(lo, hi), l| (lo.min(l), hi.max(l)));
    if lo > hi || code_of.lines.is_empty() {
        return true;
    }
    code_of
        .code
        .iter()
        .filter(|c| c.pc >= from && c.pc < to && !ignored(c.op))
        .any(|c| code_of.lines.get(c.pc as usize).is_some_and(|l| (lo..=hi).contains(l)))
}

/// The register the caller's instruction just before a copy (at index
/// `start`) writes, and its pc: an argument made there, on the caller's own
/// line. One register, by a plain write: no jump, nothing more.
fn argument_before(helper: &Proto, caller: &Proto, start: usize) -> Option<(usize, i32)> {
    let prev = caller.code.get(start.checked_sub(1)?)?;
    if prev.jump_target().is_some() || ignored(prev.op) || !outside(helper, caller, prev.pc) {
        return None;
    }
    let written: Vec<usize> = full_effects(prev, caller.maxstack).1.iter().collect();
    match written[..] {
        [r] => Some((r, prev.pc)),
        _ => None,
    }
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
