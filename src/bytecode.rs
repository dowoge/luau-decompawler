//! Bytecode container: versions 3..14, plus stock Luau output.
//!
//!   [u8 version][u8 typesversion] strings [userdata type remap (tv 3)]
//!   [varint nprotos] protos [varint main proto]
//!   proto (v14): [varint byte size] [maxstack nparams nups vararg] [flags]
//!       [varint typeinfo len][typeinfo] [varint sizecode][u32 code...]
//!       [varint sizek][consts] [varint sizep][varint child...] [varint linedefined]
//!       [varint debugname] [lineinfo] [debuginfo] [v11+ extras covered by the size]
//!   [varint main proto] then, on most game scripts, a 24-byte trailer.
//!
//! Opcodes are stored scrambled: a stored byte maps to T1[byte], and
//! T2[T1[byte]] is the upstream Luau opcode number.

const T1: [u8; 256] = [
    0x8d, 0x19, 0xe8, 0x10, 0x13, 0xda, 0x07, 0x5b, 0x3b, 0x11, 0x2b, 0xf6, 0x17, 0x53, 0x09, 0x5d,
    0xbe, 0xd7, 0x85, 0x1d, 0x95, 0xad, 0x33, 0x54, 0x91, 0x31, 0xf0, 0x29, 0x0f, 0xe5, 0xe2, 0x96,
    0x57, 0x61, 0x28, 0xc4, 0xc6, 0x94, 0x0b, 0x2f, 0x1c, 0x6b, 0x55, 0xb9, 0x1b, 0x0c, 0x03, 0xe9,
    0x40, 0xc0, 0x48, 0x35, 0xf4, 0xfe, 0x3f, 0x9d, 0xfc, 0xd8, 0x75, 0xa0, 0xfd, 0x79, 0xed, 0x4b,
    0x39, 0x4f, 0x51, 0x26, 0x00, 0x47, 0x68, 0xcc, 0x05, 0x24, 0x65, 0x88, 0x44, 0xe3, 0x43, 0x63,
    0x36, 0xa1, 0x56, 0x73, 0x23, 0xf3, 0x20, 0xca, 0x8e, 0x6f, 0x7e, 0x3e, 0x71, 0xf7, 0xcd, 0x7c,
    0x99, 0x87, 0x8b, 0x93, 0x66, 0xd1, 0x30, 0x81, 0x77, 0xe6, 0x90, 0x89, 0x58, 0x04, 0x7d, 0x7b,
    0x01, 0xee, 0x1f, 0xb5, 0x4c, 0x32, 0x97, 0xe4, 0x41, 0xf9, 0x21, 0xdf, 0x7f, 0xab, 0x15, 0xbf,
    0x0d, 0xd4, 0xa9, 0xd2, 0xde, 0x52, 0x2d, 0xc7, 0x83, 0x82, 0x46, 0xb8, 0xf1, 0x70, 0xe1, 0xb7,
    0x64, 0x80, 0x0a, 0xbd, 0x3d, 0xa5, 0x49, 0x9b, 0x5f, 0x14, 0xa6, 0xe7, 0x0e, 0xbc, 0xa7, 0xdb,
    0x67, 0x08, 0xa3, 0xcf, 0x4d, 0x74, 0xd5, 0x3a, 0x2a, 0x9c, 0x9f, 0xeb, 0x69, 0xdd, 0xc3, 0xba,
    0xfa, 0x60, 0xb3, 0xbb, 0x45, 0xc5, 0x12, 0xd9, 0x86, 0xd6, 0xa2, 0x06, 0xc8, 0xb6, 0xb4, 0xef,
    0x2c, 0x4e, 0xb1, 0xc9, 0xe0, 0xae, 0x25, 0xf5, 0x76, 0x92, 0x59, 0x9e, 0x98, 0xf8, 0x1a, 0xb2,
    0xce, 0x16, 0x8f, 0x1e, 0xf2, 0xa8, 0xc1, 0x5a, 0x34, 0x02, 0x38, 0x62, 0x6e, 0x7a, 0x8c, 0x22,
    0xb0, 0x18, 0x3c, 0x2e, 0xd0, 0xac, 0xaf, 0x8a, 0xa4, 0x78, 0xdc, 0xfb, 0xd3, 0xaa, 0xff, 0x42,
    0x84, 0x72, 0xcb, 0xea, 0x37, 0x9a, 0x6c, 0x6a, 0x5c, 0x50, 0x27, 0x5e, 0x6d, 0x4a, 0xec, 0xc2,
];
const T2: [u8; 256] = [
    0xec, 0xd0, 0x13, 0x7a, 0x6f, 0x18, 0x49, 0xc2, 0xab, 0x1a, 0xc6, 0x22, 0xaf, 0x80, 0xb4, 0x34,
    0x61, 0x23, 0x52, 0x2c, 0x53, 0xea, 0xbb, 0x84, 0x6b, 0xcb, 0x5a, 0xe4, 0xb8, 0x11, 0x51, 0x66,
    0x32, 0xbe, 0xd5, 0x9c, 0xe3, 0x02, 0x21, 0x3e, 0xf6, 0x69, 0x38, 0xee, 0x40, 0x42, 0x01, 0xed,
    0xe2, 0xd3, 0xc7, 0x72, 0x48, 0x71, 0x70, 0x7c, 0xde, 0xc0, 0x6d, 0x58, 0x36, 0x5c, 0x29, 0xd2,
    0x10, 0x28, 0x85, 0xda, 0x44, 0xbc, 0x6e, 0xb7, 0xa6, 0xf2, 0x9f, 0xf5, 0xfc, 0x0c, 0x0b, 0x8b,
    0x73, 0x56, 0x77, 0x4f, 0x3d, 0x4e, 0x06, 0x60, 0xa4, 0x2e, 0x7d, 0x8d, 0xa8, 0xe5, 0x09, 0x88,
    0x5b, 0x2b, 0xa9, 0xa5, 0x30, 0xae, 0x4c, 0xe0, 0x82, 0x64, 0xdd, 0x83, 0x12, 0xd4, 0x74, 0x93,
    0xcf, 0xf4, 0x1b, 0xd1, 0xd7, 0xfe, 0x98, 0x78, 0xc3, 0x5f, 0x3f, 0x05, 0x55, 0x3a, 0x5e, 0x54,
    0xfb, 0xad, 0xa3, 0xd8, 0x50, 0x46, 0xe8, 0xeb, 0x79, 0xd9, 0x2d, 0xb6, 0x0a, 0x00, 0xc8, 0x86,
    0x0e, 0x08, 0x63, 0x81, 0x57, 0xdc, 0x95, 0x92, 0xc4, 0x20, 0x47, 0xbd, 0x03, 0x9d, 0xf9, 0xce,
    0xc9, 0x3b, 0x7e, 0x76, 0xf8, 0x27, 0x1e, 0x4a, 0xe7, 0x16, 0xef, 0x1f, 0x97, 0xa7, 0x37, 0x62,
    0xa0, 0xd6, 0x25, 0x26, 0xaa, 0x31, 0xdf, 0x65, 0x39, 0x19, 0xc5, 0xf1, 0x7f, 0x91, 0xb0, 0xb5,
    0xdb, 0xb2, 0x35, 0xfa, 0xc1, 0x87, 0x8c, 0x0d, 0x14, 0xa1, 0xfd, 0xe6, 0x4d, 0x8a, 0xf0, 0x41,
    0xcc, 0x17, 0xe1, 0x24, 0x4b, 0xa2, 0xb3, 0x7b, 0x33, 0x1d, 0xf7, 0x15, 0x8e, 0x2f, 0xac, 0x89,
    0x6c, 0x9a, 0xca, 0x0f, 0x5d, 0xff, 0x43, 0xe9, 0x96, 0x45, 0xb1, 0x99, 0x6a, 0x2a, 0x9b, 0x75,
    0x9e, 0x04, 0x1c, 0x67, 0x3c, 0xcd, 0xb9, 0xbf, 0x8f, 0xf3, 0x90, 0x59, 0x68, 0x94, 0x07, 0xba,
];

pub const OPNAMES: [&str; 91] = [
    "NOP", "BREAK", "LOADNIL", "LOADB", "LOADN", "LOADK", "MOVE", "GETGLOBAL", "SETGLOBAL",
    "GETUPVAL", "SETUPVAL", "CLOSEUPVALS", "GETIMPORT", "GETTABLE", "SETTABLE", "GETTABLEKS",
    "SETTABLEKS", "GETTABLEN", "SETTABLEN", "NEWCLOSURE", "NAMECALL", "CALL", "RETURN", "JUMP",
    "JUMPBACK", "JUMPIF", "JUMPIFNOT", "JUMPIFEQ", "JUMPIFLE", "JUMPIFLT", "JUMPIFNOTEQ",
    "JUMPIFNOTLE", "JUMPIFNOTLT", "ADD", "SUB", "MUL", "DIV", "MOD", "POW", "ADDK", "SUBK", "MULK",
    "DIVK", "MODK", "POWK", "AND", "OR", "ANDK", "ORK", "CONCAT", "NOT", "MINUS", "LENGTH",
    "NEWTABLE", "DUPTABLE", "SETLIST", "FORNPREP", "FORNLOOP", "FORGLOOP", "FORGPREP_INEXT",
    "FASTCALL3", "FORGPREP_NEXT", "NATIVECALL", "GETVARARGS", "DUPCLOSURE", "PREPVARARGS",
    "LOADKX", "JUMPX", "FASTCALL", "COVERAGE", "CAPTURE", "SUBRK", "DIVRK", "FASTCALL1",
    "FASTCALL2", "FASTCALL2K", "FORGPREP", "JUMPXEQKNIL", "JUMPXEQKB", "JUMPXEQKN", "JUMPXEQKS",
    "IDIV", "IDIVK", "GETUDATAKS", "SETUDATAKS", "NAMECALLUDATA", "NEWCLASSMEMBER", "CALLFB",
    "CMPPROTO", "FASTPCALL", "NEWCLASS",
];

pub mod op {
    pub const GETUDATAKS: u8 = 83;
    pub const SETUDATAKS: u8 = 84;
    pub const NAMECALLUDATA: u8 = 85;
    pub const NOP: u8 = 0;
    pub const BREAK: u8 = 1;
    pub const LOADNIL: u8 = 2;
    pub const LOADB: u8 = 3;
    pub const LOADN: u8 = 4;
    pub const LOADK: u8 = 5;
    pub const MOVE: u8 = 6;
    pub const GETGLOBAL: u8 = 7;
    pub const SETGLOBAL: u8 = 8;
    pub const GETUPVAL: u8 = 9;
    pub const SETUPVAL: u8 = 10;
    pub const CLOSEUPVALS: u8 = 11;
    pub const GETIMPORT: u8 = 12;
    pub const GETTABLE: u8 = 13;
    pub const SETTABLE: u8 = 14;
    pub const GETTABLEKS: u8 = 15;
    pub const SETTABLEKS: u8 = 16;
    pub const GETTABLEN: u8 = 17;
    pub const SETTABLEN: u8 = 18;
    pub const NEWCLOSURE: u8 = 19;
    pub const NAMECALL: u8 = 20;
    pub const CALL: u8 = 21;
    pub const RETURN: u8 = 22;
    pub const JUMP: u8 = 23;
    pub const JUMPBACK: u8 = 24;
    pub const JUMPIF: u8 = 25;
    pub const JUMPIFNOT: u8 = 26;
    pub const JUMPIFEQ: u8 = 27;
    pub const JUMPIFLE: u8 = 28;
    pub const JUMPIFLT: u8 = 29;
    pub const JUMPIFNOTEQ: u8 = 30;
    pub const JUMPIFNOTLE: u8 = 31;
    pub const JUMPIFNOTLT: u8 = 32;
    pub const ADD: u8 = 33;
    pub const SUB: u8 = 34;
    pub const MUL: u8 = 35;
    pub const DIV: u8 = 36;
    pub const MOD: u8 = 37;
    pub const POW: u8 = 38;
    pub const ADDK: u8 = 39;
    pub const SUBK: u8 = 40;
    pub const MULK: u8 = 41;
    pub const DIVK: u8 = 42;
    pub const MODK: u8 = 43;
    pub const POWK: u8 = 44;
    pub const AND: u8 = 45;
    pub const OR: u8 = 46;
    pub const ANDK: u8 = 47;
    pub const ORK: u8 = 48;
    pub const CONCAT: u8 = 49;
    pub const NOT: u8 = 50;
    pub const MINUS: u8 = 51;
    pub const LENGTH: u8 = 52;
    pub const NEWTABLE: u8 = 53;
    pub const DUPTABLE: u8 = 54;
    pub const SETLIST: u8 = 55;
    pub const FORNPREP: u8 = 56;
    pub const FORNLOOP: u8 = 57;
    pub const FORGLOOP: u8 = 58;
    pub const FORGPREP_INEXT: u8 = 59;
    pub const FASTCALL3: u8 = 60;
    pub const FORGPREP_NEXT: u8 = 61;
    pub const NATIVECALL: u8 = 62;
    pub const GETVARARGS: u8 = 63;
    pub const DUPCLOSURE: u8 = 64;
    pub const PREPVARARGS: u8 = 65;
    pub const LOADKX: u8 = 66;
    pub const JUMPX: u8 = 67;
    pub const FASTCALL: u8 = 68;
    pub const COVERAGE: u8 = 69;
    pub const CAPTURE: u8 = 70;
    pub const SUBRK: u8 = 71;
    pub const DIVRK: u8 = 72;
    pub const FASTCALL1: u8 = 73;
    pub const FASTCALL2: u8 = 74;
    pub const FASTCALL2K: u8 = 75;
    pub const FORGPREP: u8 = 76;
    pub const JUMPXEQKNIL: u8 = 77;
    pub const JUMPXEQKB: u8 = 78;
    pub const JUMPXEQKN: u8 = 79;
    pub const JUMPXEQKS: u8 = 80;
    pub const IDIV: u8 = 81;
    pub const IDIVK: u8 = 82;
    pub const CALLFB: u8 = 87;
    pub const FASTPCALL: u8 = 89;
}

/// Instruction name; unknown opcodes print as `OP<n>`.
pub fn op_name(o: u8) -> String {
    match OPNAMES.get(o as usize) {
        Some(n) => (*n).to_string(),
        None => format!("OP{o}"),
    }
}

/// Two words for these upstream opcode numbers.
pub fn insn_len(o: u8) -> i32 {
    match o {
        7 | 8 | 12 | 15 | 16 | 20 | 27..=32 | 53 | 55 | 58 | 60 | 66 | 74 | 75 | 77..=80
        | 83..=88 | 90 | 91 | 93 | 94 => 2,
        _ => 1,
    }
}

pub fn is_cond_jump(o: u8) -> bool {
    matches!(o, 25..=32 | 77..=80)
}

pub fn is_for(o: u8) -> bool {
    matches!(o, 56 | 57 | 58 | 59 | 61 | 76)
}

pub fn is_forgprep(o: u8) -> bool {
    matches!(o, 59 | 61 | 76)
}

/// Every instruction whose name starts with JUMP.
fn is_jump_family(o: u8) -> bool {
    matches!(o, 23..=32 | 67 | 77..=80)
}

#[derive(Clone, Copy, Debug)]
pub struct Insn {
    pub pc: i32,
    pub op: u8,
    pub w: u32,
    pub aux: u32,
    pub n: i32,
}

impl Insn {
    #[inline]
    pub fn a(&self) -> usize {
        ((self.w >> 8) & 0xFF) as usize
    }
    #[inline]
    pub fn b(&self) -> usize {
        ((self.w >> 16) & 0xFF) as usize
    }
    #[inline]
    pub fn c(&self) -> usize {
        ((self.w >> 24) & 0xFF) as usize
    }
    #[inline]
    pub fn d(&self) -> i32 {
        (self.w >> 16) as u16 as i16 as i32
    }

    /// Where the instruction can jump, if anywhere.
    pub fn jump_target(&self) -> Option<i32> {
        if self.op == op::JUMPX {
            let e = (self.w >> 8) as i32;
            return Some(self.pc + 1 + if e & 0x800000 != 0 { e - (1 << 24) } else { e });
        }
        if self.op == op::LOADB {
            let c = self.c() as i32;
            return if c != 0 { Some(self.pc + 1 + c) } else { None };
        }
        if is_jump_family(self.op) || is_for(self.op) {
            return Some(self.pc + 1 + self.d());
        }
        None
    }

    /// Successor pcs, in the order the analyses expect (at most two).
    pub fn successors(&self) -> ([i32; 2], usize) {
        let t = self.jump_target();
        if self.op == op::RETURN {
            return ([0, 0], 0);
        }
        if matches!(self.op, op::JUMP | op::JUMPBACK | op::JUMPX) || is_forgprep(self.op) {
            return ([t.unwrap(), 0], 1);
        }
        if self.op == op::LOADB {
            return ([t.unwrap_or(self.pc + self.n), 0], 1);
        }
        match t {
            Some(t) => ([self.pc + self.n, t], 2),
            None => ([self.pc + self.n, 0], 1),
        }
    }
}

#[derive(Clone, Debug)]
pub enum K {
    Nil,
    Bool(bool),
    Number(f64),
    /// Index into the string table, or none for the empty reference.
    Str(Option<usize>),
    Import(u32),
    Closure(usize),
    Vector([f64; 3]),
    TableK(Vec<(u64, i32)>),
    Integer(i128),
    Class,
}

#[derive(Debug)]
pub struct Proto {
    pub maxstack: usize,
    pub numparams: usize,
    pub nups: usize,
    pub vararg: u8,
    pub code: Vec<Insn>,
    pub sizecode: i32,
    pub k: Vec<K>,
    pub children: Vec<usize>,
    pub name: Option<String>,
    /// (name, start pc, register) of debug locals that are named.
    pub locals: Vec<(String, u64, usize)>,
    /// Source line the function starts at, and of each pc (empty if stripped).
    pub linedefined: u32,
    pub lines: Vec<u32>,
    /// The type the compiler recorded for each parameter (its annotation),
    /// one byte each; empty when none was.
    pub param_types: Vec<u8>,
    /// Luau's proto flags: 1 native module, 2 cold, 4 has a native function,
    /// 8 inlinable, 16 uses export.
    pub flags: u8,
}

#[derive(Debug)]
pub struct Module {
    pub strings: Vec<Vec<u8>>,
    /// The string table decoded the way names are used (invalid UTF-8 replaced).
    pub text: Vec<String>,
    pub protos: Vec<Proto>,
    pub main: usize,
    /// Names of the host types a type byte 64 + i refers to.
    pub userdata: Vec<Option<String>>,
}

/// An annotation as the source wrote it, from a recorded type byte; None for
/// those that name no type a source could have spelled that way (`any`, a
/// table or function type, nil, plain userdata).
pub fn type_name(m: &Module, t: u8) -> Option<String> {
    let base = t & 0x7F;
    let name = match base {
        1 => "boolean".to_string(),
        2 => "number".to_string(),
        3 => "string".to_string(),
        6 => "thread".to_string(),
        8 => "Vector3".to_string(), // the host's vector type
        9 => "buffer".to_string(),
        64..=95 => m.userdata.get((base - 64) as usize)?.clone()?,
        _ => return None,
    };
    Some(if t & 0x80 != 0 { format!("{name}?") } else { name })
}

/// Parameter types from a function's type information: the function type
/// `[5 nparams type...]`, first in it from types version 2 on (after the
/// sizes of it, the upvalue types and the local types).
fn param_types(blob: &[u8], tv: u8) -> Vec<u8> {
    let mut r = Reader { d: blob, o: 0 };
    let func: &[u8] = if tv >= 2 {
        let Ok(n) = r.varint() else { return Vec::new() };
        if r.varint().is_err() || r.varint().is_err() {
            return Vec::new();
        }
        r.take(n as usize).unwrap_or(&[])
    } else {
        blob
    };
    match func {
        [5, n, rest @ ..] => rest.iter().take(*n as usize).copied().collect(),
        _ => Vec::new(),
    }
}

struct Reader<'a> {
    d: &'a [u8],
    o: usize,
}

type R<T> = Result<T, String>;

impl Reader<'_> {
    fn u8(&mut self) -> R<u8> {
        let v = *self.d.get(self.o).ok_or("truncated")?;
        self.o += 1;
        Ok(v)
    }
    fn varint(&mut self) -> R<u64> {
        let mut v = 0u64;
        let mut sh = 0u32;
        loop {
            let b = self.u8()?;
            if sh < 64 {
                v |= ((b & 0x7F) as u64) << sh;
            }
            sh += 7;
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
    }
    fn take(&mut self, n: usize) -> R<&[u8]> {
        let end = self.o.checked_add(n).ok_or("truncated")?;
        let v = self.d.get(self.o..end).ok_or("truncated")?;
        self.o = end;
        Ok(v)
    }
    fn u32(&mut self) -> R<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> R<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f64(&mut self) -> R<f64> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> R<f32> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
}

/// How a blob stores its opcodes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Opcodes {
    /// Roblox wire bytecode: through the two tables above
    Wire,
    /// stock Luau output
    Stock,
    /// multiplied by a key: the opcode is the stored byte times it, mod 256
    /// (203 for the bytecode a Roblox client keeps in memory)
    Key(u8),
}

/// Bytecode saved as text (a `--` line, then base64), as exporters write it;
/// None when the data is no such text. Bytecode itself starts with its version.
pub fn saved_text(data: &[u8]) -> Option<Vec<u8>> {
    if data.first().is_none_or(|b| (1..=14).contains(b)) || data[0] == 0 {
        return None;
    }
    let mut b64 = Vec::with_capacity(data.len());
    for line in data.split(|c| *c == b'\n') {
        if line.starts_with(b"--") {
            continue;
        }
        b64.extend(line.iter().filter(|c| !c.is_ascii_whitespace()));
    }
    let value = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    while b64.last() == Some(&b'=') {
        b64.pop();
    }
    if b64.is_empty() || b64.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(b64.len() * 3 / 4);
    for chunk in b64.chunks(4) {
        let mut n = 0u32;
        for (i, c) in chunk.iter().enumerate() {
            n |= value(*c)? << (18 - 6 * i);
        }
        out.extend_from_slice(&n.to_be_bytes()[1..chunk.len()]);
    }
    Some(out)
}

/// Reads a whole bytecode blob whose opcodes are stored as `opcodes` says.
/// A Roblox client writes its own scripts with a low version byte but each
/// proto led by its byte size, as v12+ does; such a blob reads with sizes
/// (the strict end, nothing but the main proto and a trailer, tells which).
pub fn parse_as(data: &[u8], opcodes: Opcodes) -> R<Module> {
    let sized = data.first().is_some_and(|v| *v >= 12);
    parse_layout(data, opcodes, sized).or_else(|e| {
        if sized { Err(e) } else { parse_layout(data, opcodes, true).map_err(|_| e) }
    })
}

fn parse_layout(data: &[u8], opcodes: Opcodes, sized: bool) -> R<Module> {
    let mut r = Reader { d: data, o: 0 };
    let version = r.u8()?;
    if version == 0 {
        return Err(format!("compile error: {}", String::from_utf8_lossy(&data[1..])));
    }
    if !(3..=14).contains(&version) {
        return Err(format!("unsupported bytecode version {version}"));
    }
    let tv = if version >= 4 { r.u8()? } else { 0 };
    let nstrings = r.varint()? as usize;
    let mut strings = Vec::with_capacity(nstrings.min(1 << 16));
    for _ in 0..nstrings {
        let n = r.varint()? as usize;
        strings.push(r.take(n)?.to_vec());
    }
    let text: Vec<String> = strings.iter().map(|s| String::from_utf8_lossy(s).into_owned()).collect();
    let mut userdata: Vec<Option<String>> = Vec::new();
    if tv == 3 {
        let mut idx = r.u8()?;
        while idx != 0 {
            let s = r.varint()? as usize;
            let at = idx as usize - 1;
            if userdata.len() <= at {
                userdata.resize(at + 1, None);
            }
            userdata[at] = s.checked_sub(1).and_then(|s| text.get(s)).cloned();
            idx = r.u8()?;
        }
    }
    let nprotos = r.varint()? as usize;
    let mut protos = Vec::with_capacity(nprotos.min(1 << 16));
    for _ in 0..nprotos {
        let size = if sized { Some(r.varint()? as usize) } else { None };
        let start = r.o;
        let head = r.take(4)?;
        let (maxstack, numparams, nups, vararg) =
            (head[0] as usize, head[1] as usize, head[2] as usize, head[3]);
        let flags = if version >= 4 { r.u8()? } else { 0 };
        let mut types = Vec::new();
        if tv != 0 {
            let n = r.varint()? as usize;
            types = param_types(r.take(n)?, tv);
        }
        let sizecode = r.varint()? as usize;
        let raw = r.take(sizecode.checked_mul(4).ok_or("truncated")?)?;
        let word = |i: usize| u32::from_le_bytes(raw[i * 4..i * 4 + 4].try_into().unwrap());
        // First byte of each instruction is an opcode; the next word of a
        // two-word instruction is AUX and stays as is.
        let mut code = Vec::with_capacity(sizecode);
        let mut pc = 0usize;
        while pc < sizecode {
            let w = word(pc);
            let b = (w & 0xFF) as usize;
            let o = match opcodes {
                Opcodes::Wire => T2[T1[b] as usize],
                Opcodes::Stock => b as u8,
                Opcodes::Key(k) => (b as u8).wrapping_mul(k),
            };
            let n = insn_len(o);
            let aux = if n == 2 && pc + 1 < sizecode { word(pc + 1) } else { 0 };
            code.push(Insn { pc: pc as i32, op: o, w, aux, n });
            pc += n as usize;
        }
        let nk = r.varint()? as usize;
        let mut k = Vec::with_capacity(nk.min(1 << 16));
        for _ in 0..nk {
            let at = r.o;
            k.push(match r.u8()? {
                0 => K::Nil,
                1 => K::Bool(r.u8()? != 0),
                2 => K::Number(r.f64()?),
                3 => {
                    let i = r.varint()? as usize;
                    if i > strings.len() {
                        return Err("string index out of range".into());
                    }
                    K::Str(if i != 0 { Some(i - 1) } else { None })
                }
                4 => K::Import(r.u32()?),
                5 => {
                    // keys only: stores after DUPTABLE give each its value
                    let n = r.varint()? as usize;
                    let mut v = Vec::with_capacity(n.min(1 << 16));
                    for _ in 0..n {
                        v.push((r.varint()?, -1));
                    }
                    K::TableK(v)
                }
                6 => K::Closure(r.varint()? as usize),
                7 => {
                    let v = [r.f32()?, r.f32()?, r.f32()?, r.f32()?];
                    K::Vector([v[0] as f64, v[1] as f64, v[2] as f64])
                }
                8 => {
                    let n = r.varint()? as usize;
                    let mut v = Vec::with_capacity(n.min(1 << 16));
                    for _ in 0..n {
                        let key = r.varint()?;
                        v.push((key, r.i32()?));
                    }
                    K::TableK(v)
                }
                9 => {
                    let neg = r.u8()?;
                    let v = r.varint()? as i128;
                    K::Integer(if neg != 0 { -v } else { v })
                }
                10 => {
                    // class shape: names and member constants; kept opaque
                    r.varint()?;
                    let b = r.u8()? as i64;
                    let c = r.u8()? as i64;
                    let d = r.varint()? as i64;
                    let e = r.varint()? as i64;
                    let total = e + d - c;
                    for _ in 0..(d - b).max(0) {
                        r.varint()?;
                    }
                    for _ in 0..(total - d).max(0) {
                        r.varint()?;
                    }
                    K::Class
                }
                11 => {
                    let v = [r.f64()?, r.f64()?, r.f64()?, r.f64()?];
                    K::Vector([v[0], v[1], v[2]])
                }
                t => return Err(format!("unknown constant type {t} at {at}")),
            });
        }
        let nchildren = r.varint()? as usize;
        let mut children = Vec::with_capacity(nchildren.min(1 << 16));
        for _ in 0..nchildren {
            children.push(r.varint()? as usize);
        }
        let linedefined = r.varint()? as u32;
        let dn = r.varint()? as usize;
        let name = if dn != 0 {
            Some(text.get(dn - 1).ok_or("string index out of range")?.clone())
        } else {
            None
        };
        let mut lines = Vec::new();
        if r.u8()? != 0 {
            let gap = r.u8()? as u32;
            let offsets = r.take(sizecode)?.to_vec();
            let intervals = (sizecode.wrapping_sub(1) >> gap.min(63)) + 1;
            let absolute = r.take(intervals.checked_mul(4).ok_or("truncated")?)?;
            // both are stored as running differences
            let mut bases = Vec::with_capacity(intervals);
            let mut last = 0i32;
            for w in absolute.chunks_exact(4) {
                last = last.wrapping_add(i32::from_le_bytes([w[0], w[1], w[2], w[3]]));
                bases.push(last);
            }
            let mut offset = 0u8;
            for (pc, d) in offsets.iter().enumerate() {
                offset = offset.wrapping_add(*d);
                lines.push(bases[pc >> gap.min(63)].wrapping_add(offset as i32) as u32);
            }
        }
        let mut locals = Vec::new();
        if r.u8()? != 0 {
            for _ in 0..r.varint()? {
                let n = r.varint()? as usize;
                let s = r.varint()?;
                r.varint()?;
                let reg = r.u8()? as usize;
                if n != 0 {
                    let t = text.get(n - 1).ok_or("string index out of range")?;
                    if !t.is_empty() {
                        locals.push((t.clone(), s, reg));
                    }
                }
            }
            for _ in 0..r.varint()? {
                r.varint()?;
            }
        }
        if let Some(size) = size {
            r.o = start + size; // the size is authoritative
        }
        protos.push(Proto {
            maxstack,
            numparams,
            nups,
            vararg,
            code,
            sizecode: sizecode as i32,
            k,
            children,
            name,
            locals,
            linedefined,
            lines,
            param_types: types,
            flags,
        });
    }
    let main = r.varint()? as usize;
    // Scripts the server signs end with a fixed 24-byte trailer.
    let trailer = data.len() as i64 - r.o as i64;
    if trailer != 0 && trailer != 24 {
        return Err(format!("{trailer} unexpected bytes after the main proto"));
    }
    Ok(Module { strings, text, protos, main, userdata })
}

/// What no compiler makes, refused before anything is analysed (a damaged
/// file otherwise sends the analyses round jumps that go nowhere): unknown
/// opcodes, an instruction cut off, a jump outside the code or into an
/// instruction, code that runs off its end, a register at or past maxstack,
/// constants, upvalues and children out of range or of the wrong kind, and
/// a child that does not come before its parent (Luau writes children
/// first, so a cycle is damage). `regs_fit` says whether an instruction's
/// registers all lie below maxstack (the analyses know which it reads and
/// writes; the harness reads bytecode without them).
pub fn validate(m: &Module, regs_fit: impl Fn(&Insn, usize) -> bool) -> R<()> {
    let np = m.protos.len();
    if m.main >= np {
        return Err("the main proto is out of range".into());
    }
    for (pi, p) in m.protos.iter().enumerate() {
        let bad = |what: &str, pc: i32| -> R<()> { Err(format!("proto {pi}, pc {pc}: {what}")) };
        if p.numparams > p.maxstack {
            return bad("more parameters than registers", -1);
        }
        if p.children.iter().any(|&c| c >= pi) {
            return bad("a child that does not come before it", -1);
        }
        let Some(last) = p.code.last() else { return bad("no code", 0) };
        if !matches!(last.op, op::RETURN | op::JUMP | op::JUMPBACK | op::JUMPX) {
            return bad("the code runs off its end", last.pc);
        }
        let starts: Vec<i32> = p.code.iter().map(|i| i.pc).collect();
        let nk = p.k.len();
        let k_of = |i: i64| (0..nk as i64).contains(&i).then(|| &p.k[i as usize]);
        for ins in &p.code {
            let pc = ins.pc;
            if ins.op as usize >= OPNAMES.len() {
                return bad("an unknown opcode", pc);
            }
            if ins.n == 2 && pc + 1 >= p.sizecode {
                return bad("an instruction cut off", pc);
            }
            if let Some(t) = ins.jump_target() {
                if starts.binary_search(&t).is_err() {
                    return bad("a jump that lands outside the code", pc);
                }
            }
            if !regs_fit(ins, p.maxstack) {
                return bad("a register past maxstack", pc);
            }
            let k_index: Option<i64> = match ins.op {
                op::LOADK | op::GETIMPORT => Some(ins.d() as i64),
                op::LOADKX | op::GETGLOBAL | op::SETGLOBAL | op::GETTABLEKS | op::SETTABLEKS | op::NAMECALL
                | op::FASTCALL2K => Some(ins.aux as i64),
                op::JUMPXEQKN | op::JUMPXEQKS => Some((ins.aux & 0xFF_FFFF) as i64),
                39..=44 | 47 | 48 | op::IDIVK => Some(ins.c() as i64),
                op::SUBRK | op::DIVRK => Some(ins.b() as i64),
                _ => None,
            };
            if k_index.is_some_and(|i| k_of(i).is_none()) {
                return bad("a constant out of range", pc);
            }
            let fits = match ins.op {
                op::GETUPVAL | op::SETUPVAL => ins.b() < p.nups,
                op::NEWCLOSURE => (0..p.children.len() as i32).contains(&ins.d()),
                op::DUPCLOSURE => matches!(k_of(ins.d() as i64), Some(K::Closure(c)) if *c < np),
                op::DUPTABLE => matches!(k_of(ins.d() as i64), Some(K::TableK(_))),
                op::GETIMPORT => {
                    // its path: up to three names, ten bits each
                    let count = (ins.aux >> 30) as i64;
                    (0..count).all(|j| matches!(k_of(((ins.aux >> (20 - 10 * j)) & 1023) as i64), Some(K::Str(_))))
                }
                op::CAPTURE => ins.a() <= 2,
                _ => true,
            };
            if !fits {
                return bad("an operand of the wrong kind", pc);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One vararg proto that returns: version 6, types version 3.
    fn blob(sized: bool, trailer: usize) -> Vec<u8> {
        let proto = [1, 0, 0, 1, 0, 0, 1, 22, 0, 1, 0, 0, 0, 0, 0, 0, 0];
        let mut b = vec![6, 3, 0, 0, 1];
        if sized {
            b.push(proto.len() as u8);
        }
        b.extend(proto);
        b.push(0);
        b.extend(std::iter::repeat_n(0xAB, trailer));
        b
    }

    #[test]
    fn low_version_reads_with_or_without_proto_sizes() {
        for (sized, trailer) in [(false, 0), (true, 0), (true, 24)] {
            let m = parse_as(&blob(sized, trailer), Opcodes::Stock).unwrap();
            assert_eq!((m.protos.len(), m.main), (1, 0));
            let p = &m.protos[0];
            assert_eq!((p.maxstack, p.vararg, p.code.len(), p.code[0].op), (1, 1, 1, 22));
        }
        // neither reading leaves nothing but a trailer
        assert!(parse_as(&blob(true, 5), Opcodes::Stock).is_err());
    }
}
