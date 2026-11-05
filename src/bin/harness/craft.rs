//! Bytecode no compiler writes: random programs are compiled, then each is
//! rewritten into forms the VM runs but the compiler never makes (blocks in
//! another order, branches turned round, jumps through jumps, code nothing
//! reaches, tests that go nowhere, line info gone or wrong). The rewritten
//! bytecode is run as it is, and its decompiled text must print the same.

use std::collections::HashMap;
use std::path::Path;

use crate::bytecode::{insn_len, op};
use crate::gen;
use crate::luau::{decompile_many, par_map, Args, Item, Tools, USUAL_BUDGET};
use crate::rng::Rng;

const DECOMPILE_LIMIT: u64 = 300;
const RUN_LIMIT: u64 = 60;

/// What a copy is rewritten by.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    /// no line info, no function names
    Stripped,
    /// every instruction on one line
    OneLine,
    /// every instruction on a random line
    Scrambled,
    /// NOPs and `JUMP +0` put in
    Padded,
    /// blocks in a random order, joined by jumps
    Shuffled,
    /// conditional jumps turned round, a jump after each
    Inverted,
    /// jumps that go through another jump at the end of the code
    Bounced,
    /// code nothing reaches, after a return or jump, jumping back in
    Dead,
    /// `JUMPIF r +0`: a test whose both ways go to the next instruction
    NoWay,
    /// several of the above at once
    Mixed,
}

pub const KINDS: [Kind; 10] = [
    Kind::Stripped,
    Kind::OneLine,
    Kind::Scrambled,
    Kind::Padded,
    Kind::Shuffled,
    Kind::Inverted,
    Kind::Bounced,
    Kind::Dead,
    Kind::NoWay,
    Kind::Mixed,
];

fn kind_named(name: &str) -> Option<Kind> {
    KINDS.iter().copied().find(|k| format!("{k:?}").eq_ignore_ascii_case(name))
}

// ---- the container, kept as raw as possible ----

struct Reader<'a> {
    d: &'a [u8],
    o: usize,
}

impl Reader<'_> {
    fn u8(&mut self) -> Option<u8> {
        let v = *self.d.get(self.o)?;
        self.o += 1;
        Some(v)
    }
    fn varint(&mut self) -> Option<u64> {
        let mut v = 0u64;
        let mut sh = 0;
        loop {
            let b = self.u8()?;
            if sh < 64 {
                v |= ((b & 0x7F) as u64) << sh;
            }
            sh += 7;
            if b & 0x80 == 0 {
                return Some(v);
            }
        }
    }
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let v = self.d.get(self.o..self.o.checked_add(n)?)?;
        self.o += n;
        Some(v)
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
}

fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

struct RawProto {
    /// maxstack, params, upvalues, vararg, flags and the type info, as written
    head: Vec<u8>,
    code: Vec<u32>,
    /// the constant count and constants, then the child count and children
    consts: Vec<u8>,
    linedefined: u64,
    debugname: u64,
    lines: Option<Vec<u32>>,
    /// the debug info flag and what follows it
    debug: Vec<u8>,
}

struct RawModule {
    /// version, types version, strings, userdata names
    head: Vec<u8>,
    protos: Vec<RawProto>,
    main: u64,
}

fn skip_constants(r: &mut Reader) -> Option<()> {
    for _ in 0..r.varint()? {
        match r.u8()? {
            0 => {}
            1 => {
                r.u8()?;
            }
            2 => {
                r.take(8)?;
            }
            3 | 6 => {
                r.varint()?;
            }
            4 => {
                r.take(4)?;
            }
            5 => {
                for _ in 0..r.varint()? {
                    r.varint()?;
                }
            }
            7 => {
                r.take(16)?;
            }
            8 => {
                for _ in 0..r.varint()? {
                    r.varint()?;
                    r.take(4)?;
                }
            }
            9 => {
                r.u8()?;
                r.varint()?;
            }
            11 => {
                r.take(32)?;
            }
            _ => return None,
        }
    }
    Some(())
}

/// Stock bytecode before version 11 (no proto sizes, no feedback slots).
fn read_module(data: &[u8]) -> Option<RawModule> {
    let mut r = Reader { d: data, o: 0 };
    let version = r.u8()?;
    if !(4..=10).contains(&version) {
        return None;
    }
    let tv = r.u8()?;
    for _ in 0..r.varint()? {
        let n = r.varint()? as usize;
        r.take(n)?;
    }
    if tv == 3 {
        while r.u8()? != 0 {
            r.varint()?;
        }
    }
    let head = data[..r.o].to_vec();
    let mut protos = Vec::new();
    for _ in 0..r.varint()? {
        let start = r.o;
        r.take(5)?;
        let n = r.varint()? as usize;
        r.take(n)?;
        let head = data[start..r.o].to_vec();
        let sizecode = r.varint()? as usize;
        let code = (0..sizecode).map(|_| r.u32()).collect::<Option<Vec<u32>>>()?;
        let at = r.o;
        skip_constants(&mut r)?;
        for _ in 0..r.varint()? {
            r.varint()?;
        }
        let consts = data[at..r.o].to_vec();
        let linedefined = r.varint()?;
        let debugname = r.varint()?;
        let lines = if r.u8()? != 0 {
            let gap = r.u8()? as u32;
            if gap > 31 || sizecode == 0 {
                return None;
            }
            let offsets = r.take(sizecode)?.to_vec();
            let intervals = ((sizecode - 1) >> gap) + 1;
            let mut bases = Vec::with_capacity(intervals);
            let mut last = 0i32;
            for _ in 0..intervals {
                last = last.wrapping_add(r.u32()? as i32);
                bases.push(last);
            }
            let mut off = 0u8;
            let mut lines = Vec::with_capacity(sizecode);
            for (pc, d) in offsets.iter().enumerate() {
                off = off.wrapping_add(*d);
                lines.push(bases[pc >> gap].wrapping_add(off as i32) as u32);
            }
            Some(lines)
        } else {
            None
        };
        let at = r.o;
        if r.u8()? != 0 {
            for _ in 0..r.varint()? {
                r.varint()?;
                r.varint()?;
                r.varint()?;
                r.u8()?;
            }
            for _ in 0..r.varint()? {
                r.varint()?;
            }
        }
        let debug = data[at..r.o].to_vec();
        protos.push(RawProto { head, code, consts, linedefined, debugname, lines, debug });
    }
    let main = r.varint()?;
    (r.o == data.len()).then_some(RawModule { head, protos, main })
}

fn write_module(m: &RawModule) -> Vec<u8> {
    let mut out = m.head.clone();
    put_varint(&mut out, m.protos.len() as u64);
    for p in &m.protos {
        out.extend_from_slice(&p.head);
        put_varint(&mut out, p.code.len() as u64);
        for w in &p.code {
            out.extend_from_slice(&w.to_le_bytes());
        }
        out.extend_from_slice(&p.consts);
        put_varint(&mut out, p.linedefined);
        put_varint(&mut out, p.debugname);
        match &p.lines {
            Some(lines) if !lines.is_empty() => {
                // a base line per instruction word: every offset is 0
                out.push(1);
                out.push(0);
                out.extend(std::iter::repeat_n(0u8, lines.len()));
                let mut last = 0u32;
                for l in lines {
                    out.extend_from_slice(&(l.wrapping_sub(last) as i32).to_le_bytes());
                    last = *l;
                }
            }
            _ => out.push(0),
        }
        out.extend_from_slice(&p.debug);
    }
    put_varint(&mut out, m.main);
    out
}

// ---- instructions with their jumps as targets ----

#[derive(Clone, Debug)]
struct Ins {
    id: usize,
    w: u32,
    aux: Option<u32>,
    line: u32,
    /// the id of the instruction it jumps to (or a FASTCALL's call)
    to: Option<usize>,
}

impl Ins {
    fn op(&self) -> u8 {
        (self.w & 0xFF) as u8
    }
}

fn is_fastcall(o: u8) -> bool {
    matches!(o, op::FASTCALL | op::FASTCALL1 | op::FASTCALL2 | op::FASTCALL2K | op::FASTCALL3)
}

/// Where the jump field of an instruction is, if it has one at this word.
enum Field {
    D,
    C,
    E,
}

fn field(o: u8, w: u32) -> Option<Field> {
    match o {
        op::JUMPX => Some(Field::E),
        op::LOADB => ((w >> 24) != 0).then_some(Field::C),
        o if is_fastcall(o) => Some(Field::C),
        op::JUMP | op::JUMPBACK | 25..=32 | 77..=80 => Some(Field::D),
        op::FORNPREP | op::FORNLOOP | op::FORGLOOP | op::FORGPREP_INEXT | op::FORGPREP_NEXT | op::FORGPREP => {
            Some(Field::D)
        }
        _ => None,
    }
}

/// Never runs on to the next instruction.
fn ends(i: &Ins) -> bool {
    let o = i.op();
    matches!(o, op::RETURN | op::JUMP | op::JUMPBACK | op::JUMPX | op::FORGPREP_INEXT | op::FORGPREP_NEXT | op::FORGPREP)
        || (o == op::LOADB && i.to.is_some())
}

fn is_cond(o: u8) -> bool {
    matches!(o, 25..=32 | 77..=80)
}

struct Func {
    code: Vec<Ins>,
    next_id: usize,
    maxstack: u32,
}

fn decode(p: &RawProto) -> Option<Func> {
    let mut code = Vec::new();
    let mut word_to_id = HashMap::new();
    let mut pc = 0;
    while pc < p.code.len() {
        let w = p.code[pc];
        let o = (w & 0xFF) as u8;
        let n = insn_len(o) as usize;
        let aux = if n == 2 { Some(*p.code.get(pc + 1)?) } else { None };
        let line = p.lines.as_ref().map_or(0, |l| l[pc]);
        word_to_id.insert(pc, code.len());
        code.push((pc, Ins { id: code.len(), w, aux, line, to: None }));
        pc += n;
    }
    for (pc, ins) in &mut code {
        let o = ins.op();
        let off = match field(o, ins.w) {
            None => continue,
            Some(Field::D) => (ins.w >> 16) as u16 as i16 as i64,
            Some(Field::C) => (ins.w >> 24) as i64,
            Some(Field::E) => ((ins.w as i32) >> 8) as i64,
        };
        let t = *pc as i64 + 1 + off;
        ins.to = Some(*word_to_id.get(&usize::try_from(t).ok()?)?);
    }
    let next_id = code.len();
    Some(Func { code: code.into_iter().map(|c| c.1).collect(), next_id, maxstack: p.head[0] as u32 })
}

fn encode(f: &Func, p: &mut RawProto) -> Option<()> {
    let mut pos = HashMap::new();
    let mut at = 0usize;
    for i in &f.code {
        pos.insert(i.id, at);
        at += 1 + i.aux.is_some() as usize;
    }
    let mut words = Vec::with_capacity(at);
    let mut lines = Vec::with_capacity(at);
    for i in &f.code {
        let mut w = i.w;
        if let Some(to) = i.to {
            let here = pos[&i.id] as i64;
            let off = *pos.get(&to)? as i64 - here - 1;
            let mut o = i.op();
            if o == op::JUMP && off < 0 {
                o = op::JUMPBACK; // a loop's way back: the VM checks for interrupts there
            }
            w = (w & !0xFF) | o as u32;
            match field(i.op(), i.w)? {
                Field::D => {
                    let d = i16::try_from(off).ok()?;
                    w = (w & 0xFFFF) | ((d as u16 as u32) << 16);
                }
                Field::C => {
                    let c = u8::try_from(off).ok().filter(|c| *c > 0 || is_fastcall(o))?;
                    w = (w & 0x00FF_FFFF) | ((c as u32) << 24);
                }
                Field::E => {
                    if !(-(1 << 23)..(1 << 23)).contains(&off) {
                        return None;
                    }
                    w = (w & 0xFF) | (((off as i32) << 8) as u32);
                }
            }
        }
        words.push(w);
        lines.push(i.line);
        if let Some(aux) = i.aux {
            words.push(aux);
            lines.push(i.line);
        }
    }
    p.code = words;
    if p.lines.is_some() {
        p.lines = Some(lines);
    }
    if p.debug.first().is_some_and(|b| *b != 0) {
        p.debug = vec![0]; // its pcs no longer hold
    }
    Some(())
}

impl Func {
    fn fresh(&mut self, w: u32, line: u32, to: Option<usize>) -> Ins {
        self.next_id += 1;
        Ins { id: self.next_id - 1, w, aux: None, line, to }
    }

    fn jump(&mut self, to: usize, line: u32) -> Ins {
        self.fresh(op::JUMP as u32, line, Some(to))
    }

    /// Positions an instruction may go before: not the first, not inside a
    /// NAMECALL and its CALL, a closure and its CAPTUREs, or a FASTCALL and
    /// the call it skips to.
    fn open_positions(&self) -> Vec<bool> {
        let n = self.code.len();
        let mut open = vec![true; n + 1];
        open[0] = false;
        for (k, i) in self.code.iter().enumerate() {
            let o = i.op();
            if o == op::NAMECALL {
                open[k + 1] = false;
            }
            if o == op::CAPTURE {
                open[k] = false;
            }
            if is_fastcall(o) {
                if let Some(end) = i.to.and_then(|t| self.code.iter().position(|j| j.id == t)) {
                    for p in open.iter_mut().take(end + 1).skip(k + 1) {
                        *p = false;
                    }
                }
            }
        }
        open
    }

    fn padded(&mut self, rng: &mut Rng) {
        let open = self.open_positions();
        let mut out = Vec::new();
        let old = std::mem::take(&mut self.code);
        for (k, i) in old.into_iter().enumerate() {
            if open[k] && rng.below(4) == 0 {
                let line = i.line;
                let pad = if rng.below(2) == 0 { self.fresh(op::NOP as u32, line, None) } else { self.jump(i.id, line) };
                // jumps to i go to the padding first
                let pid = pad.id;
                let from = i.id;
                for j in out.iter_mut().filter(|j: &&mut Ins| j.to == Some(from)) {
                    if rng.below(2) == 0 {
                        j.to = Some(pid);
                    }
                }
                out.push(pad);
            }
            out.push(i);
        }
        self.code = out;
    }

    fn no_way(&mut self, rng: &mut Rng) {
        let open = self.open_positions();
        let old = std::mem::take(&mut self.code);
        let mut out = Vec::new();
        for (k, i) in old.into_iter().enumerate() {
            if open[k] && self.maxstack > 0 && rng.below(5) == 0 {
                let r = rng.below(self.maxstack as usize) as u32;
                let o = if rng.below(2) == 0 { op::JUMPIF } else { op::JUMPIFNOT };
                let t = self.fresh(o as u32 | (r << 8), i.line, Some(i.id));
                out.push(t);
            }
            out.push(i);
        }
        self.code = out;
    }

    fn inverted(&mut self, rng: &mut Rng) {
        let old = std::mem::take(&mut self.code);
        let mut out: Vec<Ins> = Vec::new();
        let n = old.len();
        for k in 0..n {
            let mut i = old[k].clone();
            let o = i.op();
            if is_cond(o) && k + 1 < n && rng.below(2) == 0 {
                let target = i.to.expect("a conditional jump has a target");
                let flipped = match o {
                    op::JUMPIF => Some(op::JUMPIFNOT),
                    op::JUMPIFNOT => Some(op::JUMPIF),
                    op::JUMPIFEQ => Some(op::JUMPIFNOTEQ),
                    op::JUMPIFNOTEQ => Some(op::JUMPIFEQ),
                    op::JUMPIFLE => Some(op::JUMPIFNOTLE),
                    op::JUMPIFNOTLE => Some(op::JUMPIFLE),
                    op::JUMPIFLT => Some(op::JUMPIFNOTLT),
                    op::JUMPIFNOTLT => Some(op::JUMPIFLT),
                    _ => None,
                };
                match flipped {
                    Some(f) => i.w = (i.w & !0xFF) | f as u32,
                    None => i.aux = i.aux.map(|a| a ^ 0x8000_0000),
                }
                i.to = Some(old[k + 1].id);
                let line = i.line;
                out.push(i);
                let j = self.jump(target, line);
                out.push(j);
                continue;
            }
            out.push(i);
        }
        self.code = out;
    }

    fn bounced(&mut self, rng: &mut Rng) {
        let mut tail = Vec::new();
        for k in 0..self.code.len() {
            let i = &self.code[k];
            let o = i.op();
            if (o == op::JUMP || o == op::JUMPBACK || is_cond(o)) && rng.below(2) == 0 {
                let (to, line) = (i.to.expect("a jump has a target"), i.line);
                let j = self.jump(to, line);
                self.code[k].to = Some(j.id);
                tail.push(j);
            }
        }
        self.code.extend(tail);
    }

    fn dead(&mut self, rng: &mut Rng) {
        let open = self.open_positions();
        let entries: Vec<usize> = (1..self.code.len()).filter(|k| open[*k]).map(|k| self.code[k].id).collect();
        let plain: Vec<Ins> = self
            .code
            .iter()
            .filter(|i| {
                let o = i.op();
                i.to.is_none()
                    && !matches!(o, op::NAMECALL | op::NEWCLOSURE | op::DUPCLOSURE | op::CAPTURE | op::PREPVARARGS)
                    && !is_fastcall(o)
            })
            .cloned()
            .collect();
        let old = std::mem::take(&mut self.code);
        let mut out = Vec::new();
        for i in old {
            let after = ends(&i);
            let line = i.line;
            out.push(i);
            if after && rng.below(3) == 0 {
                for _ in 0..rng.below(4) {
                    if plain.is_empty() {
                        break;
                    }
                    let mut c = plain[rng.below(plain.len())].clone();
                    self.next_id += 1;
                    c.id = self.next_id - 1;
                    out.push(c);
                }
                if entries.is_empty() {
                    let r = self.fresh(op::RETURN as u32 | (1 << 16), line, None);
                    out.push(r);
                } else {
                    let to = entries[rng.below(entries.len())];
                    let j = self.jump(to, line);
                    out.push(j);
                }
            }
        }
        self.code = out;
    }

    fn shuffled(&mut self, rng: &mut Rng) {
        // a LOADB that skips becomes a LOADB and a jump, so blocks may go anywhere
        let old = std::mem::take(&mut self.code);
        let mut code = Vec::with_capacity(old.len());
        for mut i in old {
            if i.op() == op::LOADB && i.to.is_some() {
                let to = i.to.take().expect("checked");
                i.w &= 0x00FF_FFFF;
                let line = i.line;
                code.push(i);
                let j = self.jump(to, line);
                code.push(j);
            } else {
                code.push(i);
            }
        }
        self.code = code;
        let open = self.open_positions();
        let targets: std::collections::HashSet<usize> = self.code.iter().filter_map(|i| i.to).collect();
        let n = self.code.len();
        let mut cuts = Vec::new();
        for k in 1..n {
            let leader = targets.contains(&self.code[k].id) || ends(&self.code[k - 1]) || is_cond(self.code[k - 1].op());
            if open[k] && (leader || rng.below(8) == 0) {
                cuts.push(k);
            }
        }
        cuts.push(n);
        let mut chunks: Vec<Vec<Ins>> = Vec::new();
        let mut from = 0;
        for c in cuts {
            chunks.push(self.code[from..c].to_vec());
            from = c;
        }
        // each chunk that runs on gets a jump to what came after it
        let firsts: Vec<usize> = chunks.iter().map(|c| c[0].id).collect();
        for k in 0..chunks.len() {
            let last = chunks[k].last().expect("chunks are not empty").clone();
            if !ends(&last) {
                let Some(next) = firsts.get(k + 1) else { return };
                let j = self.jump(*next, last.line);
                chunks[k].push(j);
            }
        }
        let mut rest: Vec<Vec<Ins>> = chunks.drain(1..).collect();
        for k in (1..rest.len()).rev() {
            rest.swap(k, rng.below(k + 1));
        }
        chunks.extend(rest);
        let mut code: Vec<Ins> = chunks.into_iter().flatten().collect();
        // a jump to the very next instruction is dropped, most of the time
        let mut k = 0;
        while k + 1 < code.len() {
            if code[k].op() == op::JUMP && code[k].to == Some(code[k + 1].id) && rng.below(4) != 0 {
                let gone = code.remove(k);
                let next = code[k].id;
                for j in code.iter_mut().filter(|j| j.to == Some(gone.id)) {
                    j.to = Some(next);
                }
                continue;
            }
            k += 1;
        }
        self.code = code;
    }
}

/// A rewritten copy, or None where this one cannot be rewritten (a jump too far).
pub fn craft(bc: &[u8], kind: Kind, rng: &mut Rng) -> Option<Vec<u8>> {
    let mut m = read_module(bc)?;
    let kinds: Vec<Kind> = if kind == Kind::Mixed {
        let mut picked: Vec<Kind> = KINDS[3..9].iter().copied().filter(|_| rng.below(2) == 0).collect();
        if picked.is_empty() {
            picked.push(Kind::Shuffled);
        }
        picked.push(KINDS[rng.below(3)]);
        picked
    } else {
        vec![kind]
    };
    for p in &mut m.protos {
        let mut f = decode(p)?;
        for k in &kinds {
            match k {
                Kind::Padded => f.padded(rng),
                Kind::Shuffled => f.shuffled(rng),
                Kind::Inverted => f.inverted(rng),
                Kind::Bounced => f.bounced(rng),
                Kind::Dead => f.dead(rng),
                Kind::NoWay => f.no_way(rng),
                _ => {}
            }
        }
        encode(&f, p)?;
        for k in &kinds {
            match k {
                Kind::Stripped => {
                    p.lines = None;
                    p.debugname = 0;
                }
                Kind::OneLine => {
                    if let Some(l) = &mut p.lines {
                        l.iter_mut().for_each(|x| *x = 1);
                    }
                }
                Kind::Scrambled => {
                    if let Some(l) = &mut p.lines {
                        l.iter_mut().for_each(|x| *x = 1 + rng.below(400) as u32);
                    }
                }
                _ => {}
            }
        }
    }
    Some(write_module(&m))
}

// ---- the test ----

pub fn main(args: &Args) -> u8 {
    let tools = Tools::find();
    let binary = args.opt("--bin").map_or(tools.decompiler.clone(), String::from);
    let kinds: Vec<Kind> = match args.opt("--kind") {
        Some(names) => names.split(',').filter_map(kind_named).collect(),
        None => KINDS.to_vec(),
    };
    let o = Options {
        count: args.num("--count", 100),
        seed: args.num("--seed", 1),
        size: args.num("--size", 25),
        per: args.num("--per", 1),
        kinds,
        keep: args.opt("--keep").map(String::from),
        write: args.opt("--write").map(String::from),
        binary,
    };
    u8::from(!run(&tools, &o))
}

pub struct Options {
    pub count: u64,
    pub seed: u64,
    pub size: usize,
    pub per: usize,
    pub kinds: Vec<Kind>,
    pub keep: Option<String>,
    pub write: Option<String>,
    pub binary: String,
}

#[derive(Default)]
struct Tally {
    copies: usize,
    unwritable: usize,
    changed: usize,
    skipped: usize,
    same: usize,
    differ: usize,
    refused: usize,
    crashed: usize,
    hung: usize,
}

struct Copy {
    program: usize,
    kind: Kind,
    data: Vec<u8>,
}

/// Prints what came of it; true when every copy decompiled and behaved.
pub fn run(tools: &Tools, o: &Options) -> bool {
    let seeds: Vec<u64> = (o.seed..o.seed + o.count).collect();
    let srcs: Vec<Item> = par_map(&seeds, |s| Item::new(format!("g{s}.luau"), gen::program(*s, o.size)));
    let code: Vec<Option<Vec<u8>>> = tools.compile(&srcs).into_iter().map(Result::ok).collect();
    let mut tally: Vec<Tally> = o.kinds.iter().map(|_| Tally::default()).collect();
    let mut copies: Vec<Copy> = Vec::new();
    for (p, bc) in code.iter().enumerate() {
        let Some(bc) = bc else { continue };
        for (ki, kind) in o.kinds.iter().enumerate() {
            for n in 0..o.per {
                let mut rng = Rng::new(seeds[p].wrapping_mul(1_000_003) ^ (ki as u64) << 40 ^ n as u64);
                tally[ki].copies += 1;
                match craft(bc, *kind, &mut rng) {
                    Some(data) => copies.push(Copy { program: p, kind: *kind, data }),
                    None => tally[ki].unwritable += 1,
                }
            }
        }
    }
    if let Some(dir) = &o.write {
        // only the copies, for a look at them one at a time
        std::fs::create_dir_all(dir).expect("a folder for the copies");
        for (c, copy) in copies.iter().enumerate() {
            let stem = format!("{}_{:?}_{c}", srcs[copy.program].name.trim_end_matches(".luau"), copy.kind);
            let _ = std::fs::write(Path::new(dir).join(format!("{stem}.bc")), &copy.data);
        }
        println!("{} copies written to {dir}", copies.len());
        return true;
    }
    let originals: Vec<Item> = code
        .iter()
        .enumerate()
        .filter_map(|(p, bc)| Some(Item::new(format!("o{p}"), bc.clone()?)))
        .collect();
    let crafted: Vec<Item> = copies.iter().enumerate().map(|(c, x)| Item::new(format!("c{c}"), x.data.clone())).collect();
    let all: Vec<Item> = originals.into_iter().chain(crafted).collect();
    let ran = tools.bytecode_outputs(&all, RUN_LIMIT, USUAL_BUDGET);
    let blobs: Vec<Vec<u8>> = copies.iter().map(|c| c.data.clone()).collect();
    let texts = decompile_many(&o.binary, &blobs, DECOMPILE_LIMIT);
    let decs: Vec<Item> = texts
        .iter()
        .enumerate()
        .filter_map(|(c, t)| Some(Item::new(format!("d{c}"), t.as_ref().ok()?.clone())))
        .collect();
    let dec_ran = tools.outputs(&decs, RUN_LIMIT, USUAL_BUDGET);
    let mut failures: Vec<(usize, String)> = Vec::new();
    for (c, copy) in copies.iter().enumerate() {
        let t = &mut tally[o.kinds.iter().position(|k| *k == copy.kind).expect("a kind asked for")];
        let orig = ran.get(&format!("o{}", copy.program));
        let mine = ran.get(&format!("c{c}"));
        if orig != mine {
            t.changed += 1; // the rewrite itself is wrong: a harness bug
            failures.push((c, "the rewritten bytecode behaves differently from the original".into()));
            continue;
        }
        if mine.is_none_or(|m| m.ends_with("budget exceeded")) {
            t.skipped += 1;
            continue;
        }
        match &texts[c] {
            // a crash leaves only the status; a caught refusal says why and a
            // single-file fallback exits 1 (see decompile_many and run_stream)
            Err(why) if why == "timed out" => {
                t.hung += 1;
                failures.push((c, "the decompiler hung".into()));
            }
            Err(why) if why.starts_with("exit status ") && !why.starts_with("exit status 1:") => {
                t.crashed += 1;
                failures.push((c, format!("crashed: {why}")));
            }
            // valid-but-pathological bytecode (blocks no compiler lays out) may
            // be refused: a clean refusal is an acceptable answer, not a bug
            Err(_) => t.refused += 1,
            Ok(_) => {
                if dec_ran.get(&format!("d{c}")) == mine {
                    t.same += 1;
                } else {
                    t.differ += 1;
                    failures.push((c, "the decompiled program behaves differently".into()));
                }
            }
        }
    }
    println!("{} programs, {} rewritten copies", o.count, copies.len());
    println!("{:<10} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}", "kind", "copies", "none", "wrong", "skip", "same", "DIFFER", "REFUSE", "CRASH", "HANG");
    let mut clean = true;
    for (k, t) in o.kinds.iter().zip(&tally) {
        println!(
            "{:<10} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}",
            format!("{k:?}"),
            t.copies,
            t.unwritable,
            t.changed,
            t.skipped,
            t.same,
            t.differ,
            t.refused,
            t.crashed,
            t.hung
        );
        // a refusal is an acceptable answer for bytecode no compiler writes;
        // a crash, a hang, wrong output, or a rewrite the VM runs differently
        // (a harness bug) are not
        clean &= t.changed + t.differ + t.crashed + t.hung == 0;
    }
    for (c, why) in failures.iter().take(12) {
        println!("  {} {:?}: {why}", srcs[copies[*c].program].name, copies[*c].kind);
    }
    if let Some(keep) = &o.keep {
        std::fs::create_dir_all(keep).expect("a folder for failures");
        for (c, why) in &failures {
            let copy = &copies[*c];
            let stem = format!("{}_{:?}_{c}", srcs[copy.program].name.trim_end_matches(".luau"), copy.kind);
            let at = Path::new(keep).join(stem).to_string_lossy().into_owned();
            let _ = std::fs::write(format!("{at}.src.luau"), &srcs[copy.program].data);
            let _ = std::fs::write(format!("{at}.bc"), &copy.data);
            if let Ok(text) = &texts[*c] {
                let _ = std::fs::write(format!("{at}.dec.luau"), text);
            }
            let shown = |o: Option<&String>| o.map_or("None".to_string(), String::clone);
            let note = format!(
                "{why}\n--- bytecode output\n{}\n--- decompiled output\n{}\n",
                shown(ran.get(&format!("c{c}"))),
                shown(dec_ran.get(&format!("d{c}")))
            );
            let _ = std::fs::write(format!("{at}.why.txt"), note);
        }
        if !failures.is_empty() {
            println!("  failing copies written to {keep}");
        }
    }
    clean
}
