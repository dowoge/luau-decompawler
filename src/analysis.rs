//! What each instruction reads and writes, register liveness, and reaching
//! definitions, which split a register into variables ("webs").
//!
//! Luau reuses a register for unrelated locals once a scope ends. The writes
//! that reach a common read are one variable.

use crate::bytecode::{is_for, op, Insn, Proto};

/// A function has at most 255 registers. What bad bytecode names past that,
/// with a range running on from a high register, is dropped.
const WORDS: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RegSet(pub [u64; WORDS]);

impl RegSet {
    pub const EMPTY: RegSet = RegSet([0; WORDS]);

    #[inline]
    pub fn add(&mut self, r: usize) {
        if r < WORDS * 64 {
            self.0[r / 64] |= 1 << (r % 64);
        }
    }
    #[inline]
    pub fn remove(&mut self, r: usize) {
        if r < WORDS * 64 {
            self.0[r / 64] &= !(1u64 << (r % 64));
        }
    }

    pub fn intersects(&self, other: &RegSet) -> bool {
        self.0.iter().zip(other.0.iter()).any(|(a, b)| a & b != 0)
    }

    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|w| *w == 0)
    }

    pub fn has(&self, r: usize) -> bool {
        r < WORDS * 64 && self.0[r / 64] >> (r % 64) & 1 != 0
    }
    pub fn add_range(&mut self, lo: usize, hi: usize) {
        let hi = hi.min(WORDS * 64);
        let mut r = lo;
        while r < hi {
            let word = r / 64;
            let end = hi.min((word + 1) * 64);
            self.0[word] |= (!0u64 >> (64 - (end - r))) << (r % 64);
            r = end;
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.0.iter().enumerate().flat_map(|(i, &w)| {
            let mut m = w;
            std::iter::from_fn(move || {
                if m == 0 {
                    return None;
                }
                let b = m.trailing_zeros() as usize;
                m &= m - 1;
                Some(i * 64 + b)
            })
        })
    }
}

/// (reads, writes, barrier) of one instruction; `top` is the end of a
/// multi-value list. FOR instructions only report the barrier here.
pub fn effects(ins: &Insn, top: usize) -> (RegSet, RegSet, bool) {
    let (a, b, c) = (ins.a(), ins.b(), ins.c());
    let (mut r, mut w) = (RegSet::EMPTY, RegSet::EMPTY);
    let mut barrier = false;
    match ins.op {
        op::LOADNIL | op::LOADB | op::LOADN | op::LOADK | op::LOADKX | op::GETGLOBAL | op::GETUPVAL
        | op::GETIMPORT | op::NEWCLOSURE | op::DUPCLOSURE | op::NEWTABLE | op::DUPTABLE => w.add(a),
        op::MOVE | op::GETTABLEKS | op::GETTABLEN | op::NOT | op::MINUS | op::LENGTH | op::ADDK
        | op::SUBK | op::MULK | op::DIVK | op::MODK | op::POWK | op::IDIVK | op::ANDK | op::ORK => {
            r.add(b);
            w.add(a);
        }
        op::GETTABLE | op::ADD | op::SUB | op::MUL | op::DIV | op::MOD | op::POW | op::IDIV | op::AND
        | op::OR => {
            r.add(b);
            r.add(c);
            w.add(a);
        }
        op::SUBRK | op::DIVRK => {
            r.add(c);
            w.add(a);
        }
        op::CONCAT => {
            r.add_range(b, c + 1);
            w.add(a);
        }
        op::NAMECALL => {
            r.add(b);
            w.add(a);
            w.add(a + 1);
        }
        op::CALL | op::CALLFB => {
            r.add_range(a, if b != 0 { a + b } else { top });
            if c != 0 {
                w.add_range(a, a + c - 1);
            } else {
                w.add(a);
            }
        }
        op::RETURN => r.add_range(a, if b != 0 { a + b - 1 } else { top }),
        op::SETGLOBAL | op::SETUPVAL | op::JUMPIF | op::JUMPIFNOT | op::JUMPXEQKNIL | op::JUMPXEQKB
        | op::JUMPXEQKN | op::JUMPXEQKS => r.add(a),
        op::JUMPIFEQ | op::JUMPIFLE | op::JUMPIFLT | op::JUMPIFNOTEQ | op::JUMPIFNOTLE
        | op::JUMPIFNOTLT => {
            r.add(a);
            r.add((ins.aux & 0xFF) as usize);
        }
        op::SETTABLEKS | op::SETTABLEN => {
            r.add(a);
            r.add(b);
        }
        op::SETTABLE => {
            r.add(a);
            r.add(b);
            r.add(c);
        }
        op::SETLIST => {
            r.add(a);
            r.add_range(b, if c != 0 { b + c - 1 } else { top });
        }
        op::CAPTURE => {
            if a != 2 {
                r.add(b);
            }
        }
        op::GETVARARGS => {
            if b != 0 {
                w.add_range(a, a + b - 1);
            } else {
                w.add(a);
            }
        }
        o if is_for(o) => barrier = true,
        _ => {}
    }
    (r, w, barrier)
}

/// effects() with FOR loops' register use spelled out (for the analyses).
pub fn full_effects(ins: &Insn, top: usize) -> (RegSet, RegSet) {
    let a = ins.a();
    match ins.op {
        op::FORNPREP | op::FORGPREP | op::FORGPREP_INEXT | op::FORGPREP_NEXT => {
            let mut r = RegSet::EMPTY;
            r.add_range(a, a + 3);
            (r, RegSet::EMPTY)
        }
        op::FORNLOOP => {
            let (mut r, mut w) = (RegSet::EMPTY, RegSet::EMPTY);
            r.add_range(a, a + 3);
            w.add(a + 2);
            (r, w)
        }
        op::FORGLOOP => {
            let (mut r, mut w) = (RegSet::EMPTY, RegSet::EMPTY);
            r.add_range(a, a + 3);
            let nv = ((ins.aux & 0xFF) as usize).max(1);
            w.add_range(a + 2, a + 3 + nv);
            (r, w)
        }
        _ => {
            let (r, w, _) = effects(ins, top);
            (r, w)
        }
    }
}

/// Does this instruction read register r through two operands?
pub fn twice(ins: &Insn, r: usize) -> bool {
    let (a, b, c) = (ins.a(), ins.b(), ins.c());
    match ins.op {
        op::GETTABLE | op::ADD | op::SUB | op::MUL | op::DIV | op::MOD | op::POW | op::IDIV | op::AND
        | op::OR => b == r && c == r,
        op::SETTABLE => [a, b, c].iter().filter(|x| **x == r).count() > 1,
        op::JUMPIFEQ | op::JUMPIFLE | op::JUMPIFLT | op::JUMPIFNOTEQ | op::JUMPIFNOTLE
        | op::JUMPIFNOTLT => a == r && (ins.aux & 0xFF) as usize == r,
        op::SETTABLEKS | op::SETTABLEN => a == r && b == r,
        _ => false,
    }
}

/// Facts about one function that never change while it is printed.
#[derive(Default)]
pub struct Flow {
    /// pc -> index into `code`, -1 inside a two-word instruction or outside.
    pub pos: Vec<i32>,
    pub nregs: usize,
    /// Per instruction: its (register, web root) writes and reads.
    def_at: Vec<u32>,
    defs: Vec<(u32, u32)>,
    use_at: Vec<u32>,
    uses: Vec<(u32, u32)>,
    /// Web root of each register's value on entry.
    pub entry: Vec<u32>,
    /// Per register: pcs that write it, ascending.
    pub def_pcs: Vec<Vec<i32>>,
    pub ndefs: usize,
    /// Webs a closure refers to (by reference or by value).
    pub byref_webs: Vec<bool>,
    /// Webs shared with a closure by reference.
    pub byref_only: Vec<bool>,
    /// Number of writes per web root.
    pub web_defs: Vec<u32>,
    /// Webs some instruction reads.
    pub used_web: Vec<bool>,
    /// Per web: first write, first read and last touch, as pcs.
    pub web_span: Vec<(i32, i32, i32)>,
    /// live_in and live_out of every instruction, `words` words each: most
    /// functions use one word of registers, and RegSet holds four.
    live: Vec<u64>,
    /// effects() of every instruction, the end of a list being the stack's
    /// (reads and writes as `live`). The printer's look-aheads ask again and
    /// again, for the same few.
    eff: Vec<u64>,
    barriers: Vec<bool>,
    words: usize,
}

fn find(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        let g = parent[parent[x as usize] as usize];
        parent[x as usize] = g;
        x = g;
    }
    x
}

fn union(parent: &mut [u32], a: u32, b: u32) {
    let (a, b) = (find(parent, a), find(parent, b));
    if a != b {
        parent[b as usize] = a;
    }
}

impl Flow {
    #[inline]
    pub fn idx(&self, pc: i32) -> Option<usize> {
        if pc < 0 {
            return None;
        }
        match self.pos.get(pc as usize) {
            Some(&i) if i >= 0 => Some(i as usize),
            _ => None,
        }
    }

    #[inline]
    pub fn def_web(&self, pc: i32, r: usize) -> Option<u32> {
        let i = self.idx(pc)?;
        let s = &self.defs[self.def_at[i] as usize..self.def_at[i + 1] as usize];
        s.iter().find(|d| d.0 as usize == r).map(|d| d.1)
    }

    #[inline]
    pub fn use_web(&self, pc: i32, r: usize) -> Option<u32> {
        let i = self.idx(pc)?;
        let s = &self.uses[self.use_at[i] as usize..self.use_at[i + 1] as usize];
        s.iter().find(|d| d.0 as usize == r).map(|d| d.1)
    }

    /// Reaching definitions and liveness for one function.
    /// `foreign` says a source line is not this function's: a helper's, whose
    /// code the compiler copied in.
    pub fn new(p: &Proto, foreign: &dyn Fn(u32) -> bool) -> Flow {
        let code = &p.code;
        let n = code.len();
        let mut pos = vec![-1i32; p.sizecode.max(0) as usize + 1];
        for (i, ins) in code.iter().enumerate() {
            pos[ins.pc as usize] = i as i32;
        }
        let at = |pc: i32| -> Option<usize> {
            if pc < 0 {
                return None;
            }
            pos.get(pc as usize).filter(|i| **i >= 0).map(|i| *i as usize)
        };

        // reads and writes; a variable-length list ends at the last
        // multi-value result
        let mut reads = Vec::with_capacity(n);
        let mut writes = Vec::with_capacity(n);
        let mut top: Option<usize> = None;
        for ins in code {
            let (r, w) = full_effects(ins, top.unwrap_or(p.maxstack));
            let (a, b, c) = (ins.a(), ins.b(), ins.c());
            let call = matches!(ins.op, op::CALL | op::CALLFB);
            if (call && c == 0) || (ins.op == op::GETVARARGS && b == 0) {
                top = Some(a + 1);
            } else if call || matches!(ins.op, op::RETURN | op::SETLIST) {
                top = None;
            }
            reads.push(r);
            writes.push(w);
        }

        // successors, as instruction indices
        let succ_of = |i: usize| -> ([usize; 2], usize) {
            let (s, c) = code[i].successors();
            let mut out = [0usize; 2];
            let mut m = 0;
            for t in &s[..c] {
                if let Some(x) = at(*t) {
                    out[m] = x;
                    m += 1;
                }
            }
            (out, m)
        };

        // basic blocks
        let mut leader = vec![false; n + 1];
        if n > 0 {
            leader[0] = true;
        }
        for i in 0..n {
            let (s, c) = succ_of(i);
            if !(c == 1 && s[0] == i + 1) {
                leader[i + 1] = true;
                for t in &s[..c] {
                    leader[*t] = true;
                }
            }
        }
        let mut block_of = vec![0u32; n];
        let mut starts: Vec<usize> = Vec::new();
        for i in 0..n {
            if leader[i] {
                starts.push(i);
            }
            block_of[i] = starts.len() as u32 - 1;
        }
        let nb = starts.len();
        let bend = |b: usize| if b + 1 < nb { starts[b + 1] } else { n };

        // liveness: the least fixed point. Blocks first, summed up as what each
        // reads before writing it and what it writes: few instructions end one
        let mut block_reads = vec![RegSet::EMPTY; nb];
        let mut block_writes = vec![RegSet::EMPTY; nb];
        for b in 0..nb {
            let (mut rd, mut wr) = (RegSet::EMPTY, RegSet::EMPTY);
            for i in (starts[b]..bend(b)).rev() {
                for w in 0..WORDS {
                    rd.0[w] = reads[i].0[w] | (rd.0[w] & !writes[i].0[w]);
                    wr.0[w] |= writes[i].0[w];
                }
            }
            block_reads[b] = rd;
            block_writes[b] = wr;
        }
        let mut block_in = block_reads.clone();
        let mut block_out = vec![RegSet::EMPTY; nb];
        let mut changed = true;
        while changed {
            changed = false;
            for b in (0..nb).rev() {
                let (s, c) = succ_of(bend(b) - 1);
                let mut o = RegSet::EMPTY;
                for t in &s[..c] {
                    let there = &block_in[block_of[*t] as usize];
                    for w in 0..WORDS {
                        o.0[w] |= there.0[w];
                    }
                }
                if o != block_out[b] {
                    block_out[b] = o;
                    for w in 0..WORDS {
                        block_in[b].0[w] = block_reads[b].0[w] | (o.0[w] & !block_writes[b].0[w]);
                    }
                    changed = true;
                }
            }
        }
        let mut live_in = vec![RegSet::EMPTY; n];
        let mut live_out = vec![RegSet::EMPTY; n];
        for b in 0..nb {
            let mut o = block_out[b];
            for i in (starts[b]..bend(b)).rev() {
                live_out[i] = o;
                for w in 0..WORDS {
                    o.0[w] = reads[i].0[w] | (o.0[w] & !writes[i].0[w]);
                }
                live_in[i] = o;
            }
        }

        let effs: Vec<(RegSet, RegSet, bool)> = code.iter().map(|ins| effects(ins, p.maxstack)).collect();
        let used = |s: &RegSet| s.0.iter().rposition(|w| *w != 0).map_or(0, |k| k + 1);
        let set_words = live_in
            .iter()
            .chain(&live_out)
            .chain(effs.iter().flat_map(|e| [&e.0, &e.1]))
            .map(used)
            .max()
            .unwrap_or(0)
            .max(1);
        let mut live = Vec::with_capacity(2 * n * set_words);
        for (i, o) in live_in.iter().zip(&live_out) {
            live.extend_from_slice(&i.0[..set_words]);
            live.extend_from_slice(&o.0[..set_words]);
        }
        drop((live_in, live_out));
        let mut eff = Vec::with_capacity(2 * effs.len() * set_words);
        for e in &effs {
            eff.extend_from_slice(&e.0 .0[..set_words]);
            eff.extend_from_slice(&e.1 .0[..set_words]);
        }
        let barriers: Vec<bool> = effs.iter().map(|e| e.2).collect();
        drop(effs);

        // definitions; those of one register get adjacent bits, in order
        let nregs = p.maxstack.max(1) + 1;
        let mut max_reg = nregs;
        for w in &writes {
            for r in w.iter() {
                max_reg = max_reg.max(r + 1);
            }
        }
        let mut def_at = Vec::with_capacity(n + 1);
        let mut def_list: Vec<(u32, u32)> = Vec::new(); // (register, def id)
        let mut reg_defs: Vec<Vec<u32>> = vec![Vec::new(); max_reg];
        let mut def_idx: Vec<u32> = Vec::new(); // def id -> instruction
        for (i, w) in writes.iter().enumerate() {
            def_at.push(def_list.len() as u32);
            for r in w.iter() {
                let id = def_idx.len() as u32;
                def_list.push((r as u32, id));
                reg_defs[r].push(id);
                def_idx.push(i as u32);
            }
        }
        def_at.push(def_list.len() as u32);
        let first_entry = def_idx.len() as u32;
        let ndefs = first_entry as usize + nregs;
        for (r, list) in reg_defs.iter_mut().enumerate().take(nregs) {
            list.push(first_entry + r as u32);
        }
        let mut reg_base = vec![0usize; max_reg + 1];
        for r in 0..max_reg {
            reg_base[r + 1] = reg_base[r] + reg_defs[r].len();
        }
        let mut bit_of = vec![0u32; ndefs];
        for (r, list) in reg_defs.iter().enumerate() {
            for (k, id) in list.iter().enumerate() {
                bit_of[*id as usize] = (reg_base[r] + k) as u32;
            }
        }
        let words = ndefs / 64 + 1;

        let mut preds: Vec<Vec<u32>> = vec![Vec::new(); nb];
        // the registers each block writes, with the bit of its last write
        let mut gen: Vec<Vec<(u32, u32)>> = vec![Vec::new(); nb];
        let mut last_write = vec![u32::MAX; max_reg];
        for b in 0..nb {
            let mut touched: Vec<u32> = Vec::new();
            for i in starts[b]..bend(b) {
                let (ds, de) = (def_at[i] as usize, def_at[i + 1] as usize);
                for d in &def_list[ds..de] {
                    if last_write[d.0 as usize] == u32::MAX {
                        touched.push(d.0);
                    }
                    last_write[d.0 as usize] = d.1;
                }
            }
            for r in touched {
                gen[b].push((r, bit_of[last_write[r as usize] as usize]));
                last_write[r as usize] = u32::MAX;
            }
            let (s, c) = succ_of(bend(b) - 1);
            for t in &s[..c] {
                preds[block_of[*t] as usize].push(b as u32);
            }
        }

        // reaching definitions over blocks
        let mut bits_in = vec![0u64; nb * words];
        let mut bits_out = vec![0u64; nb * words];
        let mut scratch = vec![0u64; words];
        let mut changed = true;
        while changed {
            changed = false;
            for b in 0..nb {
                scratch.iter_mut().for_each(|w| *w = 0);
                if b == 0 {
                    for id in first_entry..ndefs as u32 {
                        let bit = bit_of[id as usize] as usize;
                        scratch[bit / 64] |= 1 << (bit % 64);
                    }
                }
                for q in &preds[b] {
                    let o = &bits_out[*q as usize * words..(*q as usize + 1) * words];
                    for (s, v) in scratch.iter_mut().zip(o) {
                        *s |= *v;
                    }
                }
                if scratch.as_slice() != &bits_in[b * words..(b + 1) * words] {
                    bits_in[b * words..(b + 1) * words].copy_from_slice(&scratch);
                }
                for (r, bit) in &gen[b] {
                    let (lo, hi) = (reg_base[*r as usize], reg_base[*r as usize + 1]);
                    clear_bits(&mut scratch, lo, hi);
                    scratch[*bit as usize / 64] |= 1 << (*bit % 64);
                }
                let o = &mut bits_out[b * words..(b + 1) * words];
                if o != scratch.as_slice() {
                    o.copy_from_slice(&scratch);
                    changed = true;
                }
            }
        }

        // the writes reaching a common read are one variable
        let mut parent: Vec<u32> = (0..ndefs as u32).collect();
        let mut use_at = Vec::with_capacity(n + 1);
        let mut use_first: Vec<(u32, u32)> = Vec::new();
        let mut cur = vec![u32::MAX; max_reg];
        let mut stamp = vec![u32::MAX; max_reg];
        let mut ids: Vec<u32> = Vec::new();
        let mut captures: Vec<(usize, usize, Vec<u32>)> = Vec::new();
        let mut replaced: Vec<(u32, u32, usize)> = Vec::new();
        for b in 0..nb {
            let live = &bits_in[b * words..(b + 1) * words];
            for i in starts[b]..bend(b) {
                use_at.push(use_first.len() as u32);
                // `local x = f()` gets the call's own register, even one taken
                // over from a dead local if that is the topmost. So a call moved
                // down from the register right above was assigned to a variable
                // that exists: the writes reaching here are that variable. From
                // further up it may be a new local in a dead one's register, and
                // in a helper's code it is the result going where it was reserved
                let assigned = (i > 0).then(|| (&code[i], &code[i - 1])).and_then(|(m, call)| {
                    let line = p.lines.get(m.pc as usize).copied();
                    (m.op == op::MOVE
                        && matches!(call.op, op::CALL | op::CALLFB)
                        && call.c() == 2
                        && call.a() == m.b()
                        && m.a() + 1 == m.b()
                        && line.is_some_and(|l| !foreign(l)))
                    .then(|| m.a())
                });
                // the writes this one replaces, for what is decided after the pass
                for d in &def_list[def_at[i] as usize..def_at[i + 1] as usize] {
                    let r = d.0 as usize;
                    if r < max_reg && !reads[i].has(r) {
                        let top = if stamp[r] == b as u32 { Some(cur[r]) } else { None };
                        let (lo, hi) = (reg_base[r], reg_base[r + 1]);
                        let mut each = |x: u32| replaced.push((d.1, x, r));
                        match top {
                            Some(x) => each(x),
                            None => {
                                for bit in lo..hi {
                                    if live[bit / 64] >> (bit % 64) & 1 != 0 {
                                        each(reg_defs[r][bit - lo]);
                                    }
                                }
                            }
                        }
                    }
                }
                for r in reads[i].iter().chain(assigned) {
                    ids.clear();
                    if r < max_reg {
                        if stamp[r] == b as u32 {
                            ids.push(cur[r]);
                        } else {
                            let (lo, hi) = (reg_base[r], reg_base[r + 1]);
                            if lo < hi {
                                for w in lo / 64..=(hi - 1) / 64 {
                                    let mut m = live[w];
                                    if w == lo / 64 {
                                        m &= !0u64 << (lo % 64);
                                    }
                                    if w == (hi - 1) / 64 {
                                        m &= !0u64 >> (63 - (hi - 1) % 64);
                                    }
                                    while m != 0 {
                                        let bit = w * 64 + m.trailing_zeros() as usize;
                                        m &= m - 1;
                                        ids.push(reg_defs[r][bit - lo]);
                                    }
                                }
                            }
                        }
                    }
                    if code[i].op == op::CAPTURE && code[i].a() == 1 {
                        captures.push((i, r, ids.clone()));
                    }
                    if ids.is_empty() {
                        continue;
                    }
                    if assigned == Some(r) {
                        // no read: the old value is only replaced. Not the value
                        // a register holds on entry, unless it is a parameter's
                        let own = def_list[def_at[i] as usize..def_at[i + 1] as usize].iter().find(|d| d.0 as usize == r);
                        for x in ids.iter().filter(|x| **x < first_entry || r < p.numparams) {
                            if let Some(d) = own {
                                union(&mut parent, d.1, *x);
                            }
                        }
                        continue;
                    }
                    for x in &ids[1..] {
                        union(&mut parent, ids[0], *x);
                    }
                    use_first.push((r as u32, ids[0]));
                    if writes[i].has(r) {
                        let d = def_list[def_at[i] as usize..def_at[i + 1] as usize]
                            .iter()
                            .find(|d| d.0 as usize == r)
                            .unwrap();
                        union(&mut parent, ids[0], d.1);
                    }
                }
                for d in &def_list[def_at[i] as usize..def_at[i + 1] as usize] {
                    cur[d.0 as usize] = d.1;
                    stamp[d.0 as usize] = b as u32;
                }
            }
        }
        use_at.push(use_first.len() as u32);

        // A local given a constant and never assigned has no register: the
        // compiler folds its uses. So a constant no one reads went into a
        // variable that is assigned as well: what replaces it, or, if it is the
        // last word, what it replaced
        let mut read_web = vec![false; ndefs];
        for u in &use_first {
            read_web[find(&mut parent, u.1) as usize] = true;
        }
        let closes: Vec<usize> =
            code.iter().enumerate().filter(|(_, i)| i.op == op::CLOSEUPVALS).map(|(i, _)| i).collect();
        let mut targets = vec![false; n + 1];
        for c in code.iter() {
            // forward only: a loop's body starts where its back jump lands
            if let Some(t) = c.jump_target().filter(|t| *t > c.pc) {
                targets[code.partition_point(|x| x.pc < t)] = true;
            }
        }
        // the second half of `x = a == b` is no constant, though it loads one
        let constant = |id: u32| {
            if id >= first_entry {
                return false;
            }
            let i = def_idx[id as usize] as usize;
            matches!(code[i].op, op::LOADNIL | op::LOADN | op::LOADK | op::LOADKX | op::LOADB)
                && !targets[i]
                && code[i].jump_target().is_none()
        };
        // and in an arm of a test it may be that arm of a value
        let alone = |id: u32| {
            let i = def_idx[id as usize] as usize;
            let branches = |c: &crate::bytecode::Insn| c.jump_target().is_some();
            !(i > 0 && branches(&code[i - 1])) && !code.get(i + 1).is_some_and(branches)
        };
        let mut merged = Vec::new();
        for (new, old, r) in replaced {
            let entry_value = old >= first_entry;
            if entry_value && r >= p.numparams {
                continue;
            }
            // what a closure held ended here, and the register is free again
            let from = if entry_value { 0 } else { def_idx[old as usize] as usize };
            let to = def_idx[new as usize] as usize;
            if closes.iter().any(|c| from < *c && *c < to && code[*c].a() <= r) {
                continue;
            }
            let unread = |parent: &mut Vec<u32>, x: u32| !read_web[find(parent, x) as usize];
            let dead_old = constant(old) && unread(&mut parent, old);
            let dead_new = constant(new) && alone(new) && unread(&mut parent, new);
            if dead_old || dead_new {
                merged.push((new, old));
            }
        }
        for (new, old) in merged {
            union(&mut parent, new, old);
        }

        // a register a closure captures by reference is one variable shared
        // with the closure from the capture until its upvalue is closed
        // (CLOSEUPVALS at or below it, or the function's end); after that the
        // register is free for unrelated values. Every write that can run in
        // between is to that variable: in a loop, the ones before the capture
        // too, which the next pass runs after it
        let mut byref_groups: Vec<u32> = Vec::new();
        let mut visited = vec![usize::MAX; n];
        let mut pending: Vec<usize> = Vec::new();
        for (k, (c, r, mut group)) in captures.into_iter().enumerate() {
            if r < max_reg {
                let (s, m) = succ_of(c);
                pending.extend_from_slice(&s[..m]);
                while let Some(i) = pending.pop() {
                    if visited[i] == k {
                        continue;
                    }
                    visited[i] = k;
                    if code[i].op == op::CLOSEUPVALS && code[i].a() <= r {
                        continue;
                    }
                    group.extend(def_list[def_at[i] as usize..def_at[i + 1] as usize].iter().filter(|d| d.0 as usize == r).map(|d| d.1));
                    let (s, m) = succ_of(i);
                    pending.extend_from_slice(&s[..m]);
                }
                // and what comes into that stretch from outside it, even if only
                // to be replaced: a loop's cell is declared before the loop
                let (lo, hi) = (reg_base[r], reg_base[r + 1]);
                for b in 0..nb {
                    let entered = visited[starts[b]] == k
                        && (b == 0 || preds[b].iter().any(|q| visited[bend(*q as usize) - 1] != k));
                    if !entered {
                        continue;
                    }
                    let live = &bits_in[b * words..(b + 1) * words];
                    group.extend((lo..hi).filter(|bit| live[bit / 64] >> (bit % 64) & 1 != 0).map(|bit| reg_defs[r][bit - lo]));
                }
            }
            if group.is_empty() {
                continue; // nothing written to it yet: nothing to share
            }
            for x in &group[1..] {
                union(&mut parent, group[0], *x);
            }
            byref_groups.push(group[0]);
        }

        let mut byref_only = vec![false; ndefs];
        for g in &byref_groups {
            let root = find(&mut parent, *g);
            byref_only[root as usize] = true;
        }
        let mut byref_webs = byref_only.clone();
        // a closure refers to the variables it captures (even by value), so
        // in register-named output those variables keep names of their own
        for i in 0..n {
            if code[i].op == op::CAPTURE && code[i].a() == 0 {
                let r = code[i].b() as u32;
                let found = use_first[use_at[i] as usize..use_at[i + 1] as usize]
                    .iter()
                    .find(|u| u.0 == r)
                    .map(|u| u.1);
                if let Some(first) = found {
                    let root = find(&mut parent, first);
                    byref_webs[root as usize] = true;
                }
            }
        }
        let mut web_defs = vec![0u32; ndefs];
        let mut defs = Vec::with_capacity(def_list.len());
        for (r, id) in &def_list {
            let root = find(&mut parent, *id);
            web_defs[root as usize] += 1;
            defs.push((*r, root));
        }
        let mut used_web = vec![false; ndefs];
        let mut uses = Vec::with_capacity(use_first.len());
        for (r, first) in &use_first {
            let root = find(&mut parent, *first);
            used_web[root as usize] = true;
            uses.push((*r, root));
        }
        let mut web_span = vec![(i32::MAX, i32::MAX, -1); ndefs];
        for (i, ins) in code.iter().enumerate() {
            for d in &defs[def_at[i] as usize..def_at[i + 1] as usize] {
                let s = &mut web_span[d.1 as usize];
                s.0 = s.0.min(ins.pc);
                s.2 = s.2.max(ins.pc);
            }
            for u in &uses[use_at[i] as usize..use_at[i + 1] as usize] {
                let s = &mut web_span[u.1 as usize];
                s.1 = s.1.min(ins.pc);
                s.2 = s.2.max(ins.pc);
            }
        }
        let entry: Vec<u32> = (0..nregs as u32).map(|r| find(&mut parent, first_entry + r)).collect();
        let mut def_pcs: Vec<Vec<i32>> = vec![Vec::new(); max_reg];
        for (i, ins) in code.iter().enumerate() {
            for d in &def_list[def_at[i] as usize..def_at[i + 1] as usize] {
                def_pcs[d.0 as usize].push(ins.pc);
            }
        }
        Flow {
            pos,
            nregs,
            def_at,
            defs,
            use_at,
            uses,
            entry,
            def_pcs,
            ndefs,
            byref_webs,
            byref_only,
            web_defs,
            used_web,
            web_span,
            live,
            eff,
            barriers,
            words: set_words,
        }
    }

    #[inline]
    fn set_at(&self, sets: &[u64], k: usize) -> RegSet {
        let mut s = RegSet::EMPTY;
        s.0[..self.words].copy_from_slice(&sets[k * self.words..(k + 1) * self.words]);
        s
    }

    /// Registers live before instruction i (an index into the code).
    #[inline]
    pub fn live_in(&self, i: usize) -> RegSet {
        self.set_at(&self.live, 2 * i)
    }

    /// Registers live after instruction i.
    #[inline]
    pub fn live_out(&self, i: usize) -> RegSet {
        self.set_at(&self.live, 2 * i + 1)
    }

    /// effects() of instruction i.
    #[inline]
    pub fn effects(&self, i: usize) -> (RegSet, RegSet, bool) {
        (self.set_at(&self.eff, 2 * i), self.set_at(&self.eff, 2 * i + 1), self.barriers[i])
    }

    /// Every (register, web root, write pc) in pc order; pc -1 is the entry.
    pub fn all_defs(&self, code: &[Insn]) -> Vec<(usize, i32, u32)> {
        let mut out = Vec::with_capacity(self.defs.len() + self.entry.len());
        for (i, ins) in code.iter().enumerate() {
            for d in &self.defs[self.def_at[i] as usize..self.def_at[i + 1] as usize] {
                out.push((d.0 as usize, ins.pc, d.1));
            }
        }
        for (r, w) in self.entry.iter().enumerate() {
            out.push((r, -1, *w));
        }
        out
    }
}

fn clear_bits(words: &mut [u64], lo: usize, hi: usize) {
    if lo >= hi {
        return;
    }
    let (wl, wh) = (lo / 64, (hi - 1) / 64);
    let lm = !0u64 << (lo % 64);
    let hm = !0u64 >> (63 - (hi - 1) % 64);
    if wl == wh {
        words[wl] &= !(lm & hm);
    } else {
        words[wl] &= !lm;
        words[wl + 1..wh].iter_mut().for_each(|w| *w = 0);
        words[wh] &= !hm;
    }
}
