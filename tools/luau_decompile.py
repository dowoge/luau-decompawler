#!/usr/bin/env python3
"""luau_decompile.py - Roblox game-script bytecode (wire v14) to readable Lua.

Input is a `.wire` file: the decoded
RSB1 payload a script's Source hash names. The format, versions 3..14:

  [u8 version][u8 typesversion] strings [userdata type remap (tv 3)]
  [varint nprotos] protos [varint main proto]
  proto (v14): [varint byte size] [maxstack nparams nups vararg] [flags]
      [varint typeinfo len][typeinfo] [varint sizecode][u32 code...]
      [varint sizek][consts] [varint sizep][varint child...] [varint linedefined]
      [varint debugname] [lineinfo] [debuginfo] [v11+: varint n, n x u8 kind
      (+ varint when kind <= 1)] [v12+: varint when flags & 8]
  [varint main proto] then, on most game scripts, a 24-byte trailer.
  Opcodes are stored scrambled: a stored byte maps to T1[byte], and
  T2[T1[byte]] is the upstream Luau opcode number.

Output is Luau source that compiles (tools/luau_check.cpp checks that), and
tools/luau_roundtrip.py measures how close it is: recompiled at -O2 it gives
back the original instruction sequence for ~60% of functions.
tools/luau_difftest.py is the ground-truth test: it generates random
programs, compiles them, decompiles them and runs both (tools/luau_run.cpp);
~99% behave identically. Run it after changing anything here.

  * Values: register liveness and reaching definitions over the real control
    flow decide what is inlined into one expression and what is a variable;
    each live range of a register is its own variable (Luau reuses registers).
  * Control flow: if/elseif/else, numeric and generic for, while, break,
    continue; short-circuit conditions (`a and (b or not c)`) are rebuilt as
    one expression from the graph of their jumps. Jumps left over are mostly
    early returns of inlined functions: `repeat ... until true` with `break`,
    or a search loop's default moved before it. What still does not nest is
    written as a numbered-block dispatcher (well under 1% of functions).
  * Source shape: Luau if-expressions, `+=`, `` `{interpolation}` ``,
    `for k, v in t do`, `local function`, `function T:m()` with `self`,
    comparisons written constant-last (`x > 0`, not `0 < x`).
  * -O2 inlines small local functions; where such a helper's body appears
    with its parameters filled in, the call is put back (`F32(x)`).
  * Names: from what a value is (GetService/require/WaitForChild/fields,
    `Instance.new("Part")` -> part, `X.Changed:Connect` -> changedConnection),
    loop variables from their container (`wheels` -> wheel), parameters of
    well-known event handlers (`PlayerAdded` -> player), pcall results; the
    module table after its script (`local Car = {}`); `self` for
    setmetatable; the field a value is stored into; getter results; what a
    value is used as (`.UserId` -> player); parameters from what callers
    pass, across all scripts of a place in --dir mode.
    Everything else keeps a generated name: vR_D is register R in a function
    D levels deep, vRxK_D the register's K-th unrelated variable.

Usage:
  luau_decompile.py FILE.wire              readable Lua to stdout
  luau_decompile.py --disasm FILE.wire     annotated disassembly
  --upstream                               input is standard Luau bytecode
                                           (luau_compile output), not Roblox's
  luau_decompile.py --dir DIR [--out OUT]  every *.wire in DIR (index.tsv
                                           paths name the outputs)
"""
import os
import re
import struct
import sys

_T1 = """8d 19 e8 10 13 da 07 5b 3b 11 2b f6 17 53 09 5d be d7 85 1d 95 ad 33 54 91 31 f0 29 0f e5 e2 96
57 61 28 c4 c6 94 0b 2f 1c 6b 55 b9 1b 0c 03 e9 40 c0 48 35 f4 fe 3f 9d fc d8 75 a0 fd 79 ed 4b
39 4f 51 26 00 47 68 cc 05 24 65 88 44 e3 43 63 36 a1 56 73 23 f3 20 ca 8e 6f 7e 3e 71 f7 cd 7c
99 87 8b 93 66 d1 30 81 77 e6 90 89 58 04 7d 7b 01 ee 1f b5 4c 32 97 e4 41 f9 21 df 7f ab 15 bf
0d d4 a9 d2 de 52 2d c7 83 82 46 b8 f1 70 e1 b7 64 80 0a bd 3d a5 49 9b 5f 14 a6 e7 0e bc a7 db
67 08 a3 cf 4d 74 d5 3a 2a 9c 9f eb 69 dd c3 ba fa 60 b3 bb 45 c5 12 d9 86 d6 a2 06 c8 b6 b4 ef
2c 4e b1 c9 e0 ae 25 f5 76 92 59 9e 98 f8 1a b2 ce 16 8f 1e f2 a8 c1 5a 34 02 38 62 6e 7a 8c 22
b0 18 3c 2e d0 ac af 8a a4 78 dc fb d3 aa ff 42 84 72 cb ea 37 9a 6c 6a 5c 50 27 5e 6d 4a ec c2"""
_T2 = """ec d0 13 7a 6f 18 49 c2 ab 1a c6 22 af 80 b4 34 61 23 52 2c 53 ea bb 84 6b cb 5a e4 b8 11 51 66
32 be d5 9c e3 02 21 3e f6 69 38 ee 40 42 01 ed e2 d3 c7 72 48 71 70 7c de c0 6d 58 36 5c 29 d2
10 28 85 da 44 bc 6e b7 a6 f2 9f f5 fc 0c 0b 8b 73 56 77 4f 3d 4e 06 60 a4 2e 7d 8d a8 e5 09 88
5b 2b a9 a5 30 ae 4c e0 82 64 dd 83 12 d4 74 93 cf f4 1b d1 d7 fe 98 78 c3 5f 3f 05 55 3a 5e 54
fb ad a3 d8 50 46 e8 eb 79 d9 2d b6 0a 00 c8 86 0e 08 63 81 57 dc 95 92 c4 20 47 bd 03 9d f9 ce
c9 3b 7e 76 f8 27 1e 4a e7 16 ef 1f 97 a7 37 62 a0 d6 25 26 aa 31 df 65 39 19 c5 f1 7f 91 b0 b5
db b2 35 fa c1 87 8c 0d 14 a1 fd e6 4d 8a f0 41 cc 17 e1 24 4b a2 b3 7b 33 1d f7 15 8e 2f ac 89
6c 9a ca 0f 5d ff 43 e9 96 45 b1 99 6a 2a 9b 75 9e 04 1c 67 3c cd b9 bf 8f f3 90 59 68 94 07 ba"""
T1 = bytes(int(x, 16) for x in _T1.split())
T2 = bytes(int(x, 16) for x in _T2.split())
assert len(T1) == 256 and len(T2) == 256

OPNAMES = [
    "NOP", "BREAK", "LOADNIL", "LOADB", "LOADN", "LOADK", "MOVE", "GETGLOBAL",
    "SETGLOBAL", "GETUPVAL", "SETUPVAL", "CLOSEUPVALS", "GETIMPORT", "GETTABLE",
    "SETTABLE", "GETTABLEKS", "SETTABLEKS", "GETTABLEN", "SETTABLEN",
    "NEWCLOSURE", "NAMECALL", "CALL", "RETURN", "JUMP", "JUMPBACK", "JUMPIF",
    "JUMPIFNOT", "JUMPIFEQ", "JUMPIFLE", "JUMPIFLT", "JUMPIFNOTEQ",
    "JUMPIFNOTLE", "JUMPIFNOTLT", "ADD", "SUB", "MUL", "DIV", "MOD", "POW",
    "ADDK", "SUBK", "MULK", "DIVK", "MODK", "POWK", "AND", "OR", "ANDK", "ORK",
    "CONCAT", "NOT", "MINUS", "LENGTH", "NEWTABLE", "DUPTABLE", "SETLIST",
    "FORNPREP", "FORNLOOP", "FORGLOOP", "FORGPREP_INEXT", "FASTCALL3",
    "FORGPREP_NEXT", "NATIVECALL", "GETVARARGS", "DUPCLOSURE", "PREPVARARGS",
    "LOADKX", "JUMPX", "FASTCALL", "COVERAGE", "CAPTURE", "SUBRK", "DIVRK",
    "FASTCALL1", "FASTCALL2", "FASTCALL2K", "FORGPREP", "JUMPXEQKNIL",
    "JUMPXEQKB", "JUMPXEQKN", "JUMPXEQKS", "IDIV", "IDIVK", "GETUDATAKS",
    "SETUDATAKS", "NAMECALLUDATA", "NEWCLASSMEMBER", "CALLFB", "CMPPROTO",
    "FASTPCALL", "NEWCLASS",
]
OP = {n: i for i, n in enumerate(OPNAMES)}


def op_name(op):
    return OPNAMES[op] if op < len(OPNAMES) else f"OP{op}"


def insn_len(op):
    """Two words for these upstream opcode numbers."""
    two = {7, 8, 12, 15, 16, 20, 27, 28, 29, 30, 31, 32, 53, 55, 58, 60, 66,
           74, 75, 77, 78, 79, 80, 83, 84, 85, 86, 87, 88, 90, 91, 93, 94}
    return 2 if op in two else 1


class Reader:
    def __init__(self, data, off=0):
        self.d = data
        self.o = off

    def u8(self):
        v = self.d[self.o]
        self.o += 1
        return v

    def varint(self):
        v = sh = 0
        while True:
            b = self.u8()
            v |= (b & 0x7F) << sh
            sh += 7
            if not b & 0x80:
                return v

    def take(self, n):
        v = self.d[self.o:self.o + n]
        if len(v) != n:
            raise ValueError("truncated")
        self.o += n
        return v

    def u32(self):
        return struct.unpack("<I", self.take(4))[0]

    def i32(self):
        return struct.unpack("<i", self.take(4))[0]

    def f64(self):
        return struct.unpack("<d", self.take(8))[0]

    def f32(self):
        return struct.unpack("<f", self.take(4))[0]


class Proto:
    pass


def parse(data, scrambled=True):
    r = Reader(data)
    version = r.u8()
    if version == 0:
        raise ValueError("compile error: " + data[1:].decode("utf-8", "replace"))
    if not 3 <= version <= 14:
        raise ValueError(f"unsupported bytecode version {version}")
    tv = r.u8() if version >= 4 else 0
    strings = [bytes(r.take(r.varint())) for _ in range(r.varint())]
    if tv == 3:
        idx = r.u8()
        while idx:
            r.varint()
            idx = r.u8()
    protos = []
    for _ in range(r.varint()):
        p = Proto()
        size = r.varint() if version >= 12 else None
        start = r.o
        p.maxstack, p.numparams, p.nups, p.vararg = r.take(4)
        p.flags = r.u8() if version >= 4 else 0
        if tv:
            r.take(r.varint())
        sizecode = r.varint()
        words = [r.u32() for _ in range(sizecode)]
        # Unscramble: first byte of each instruction is an opcode, the next
        # word of a two-word instruction is AUX and stays as is.
        p.code = []
        pc = 0
        while pc < sizecode:
            w = words[pc]
            op = T2[T1[w & 0xFF]] if scrambled else w & 0xFF
            n = insn_len(op)
            p.code.append((pc, op, w, words[pc + 1] if n == 2 and pc + 1 < sizecode else 0, n))
            pc += n
        p.sizecode = sizecode
        p.k = []
        for _ in range(r.varint()):
            t = r.u8()
            if t == 0:
                p.k.append(("nil", None))
            elif t == 1:
                p.k.append(("bool", bool(r.u8())))
            elif t == 2:
                p.k.append(("number", r.f64()))
            elif t == 3:
                i = r.varint()
                p.k.append(("string", strings[i - 1] if i else None))
            elif t == 4:
                p.k.append(("import", r.u32()))
            elif t == 5:
                p.k.append(("table", [r.varint() for _ in range(r.varint())]))
            elif t == 6:
                p.k.append(("closure", r.varint()))
            elif t == 7:
                p.k.append(("vector", struct.unpack("<4f", r.take(16))))
            elif t == 8:
                n = r.varint()
                p.k.append(("tablek", [(r.varint(), r.i32()) for _ in range(n)]))
            elif t == 9:
                neg = r.u8()
                v = r.varint()
                p.k.append(("integer", -v if neg else v))
            elif t == 10:
                # class shape: names and member constants; kept opaque
                a = r.varint()
                b = r.u8()
                c = r.u8()
                d = r.varint()
                e = r.varint()
                total = e + d - c
                for _ in range(d - b):
                    r.varint()
                for _ in range(total - d):
                    r.varint()
                p.k.append(("class", a))
            elif t == 11:
                p.k.append(("vector64", struct.unpack("<4d", r.take(32))))
            else:
                raise ValueError(f"unknown constant type {t} at {r.o - 1}")
        p.children = [r.varint() for _ in range(r.varint())]
        p.linedefined = r.varint()
        dn = r.varint()
        p.name = strings[dn - 1].decode("utf-8", "replace") if dn else None
        p.lines = None
        if r.u8():
            gap = r.u8()
            deltas = r.take(sizecode)
            intervals = ((sizecode - 1) >> gap) + 1
            abs_ = [r.i32() for _ in range(intervals)]
            lines = []
            acc = 0
            for i in range(sizecode):
                acc = (acc + deltas[i]) & 0xFF
                base = abs_[i >> gap]
                lines.append(base + (acc if acc < 128 else acc - 256))
            p.lines = lines
        p.locals = []
        p.upnames = []
        if r.u8():
            for _ in range(r.varint()):
                n = r.varint()
                s = r.varint()
                e = r.varint()
                reg = r.u8()
                p.locals.append((strings[n - 1].decode("utf-8", "replace") if n else None, s, e, reg))
            for _ in range(r.varint()):
                n = r.varint()
                p.upnames.append(strings[n - 1].decode("utf-8", "replace") if n else None)
        if size is not None:
            r.o = start + size  # v12+: the size is authoritative
        protos.append(p)
    main = r.varint()
    # Scripts the server signs end with a fixed 24-byte trailer.
    trailer = len(data) - r.o
    if trailer not in (0, 24):
        raise ValueError(f"{trailer} unexpected bytes after the main proto")
    return {"version": version, "tv": tv, "strings": strings, "protos": protos,
            "main": main, "end": r.o, "size": len(data), "trailer": trailer}


# ---------------------------------------------------------------- formatting

IDENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
KEYWORDS = {"and", "break", "do", "else", "elseif", "end", "false", "for",
            "function", "if", "in", "local", "nil", "not", "or", "repeat",
            "return", "then", "true", "until", "while", "continue"}


def lua_string(b):
    out = []
    for c in b:
        if c == 0x5C:
            out.append("\\\\")
        elif c == 0x22:
            out.append('\\"')
        elif c == 0x0A:
            out.append("\\n")
        elif c == 0x0D:
            out.append("\\r")
        elif c == 0x09:
            out.append("\\t")
        elif 32 <= c < 127:
            out.append(chr(c))
        else:
            out.append("\\%d" % c)
    return '"' + "".join(out) + '"'


def fmt_number(v):
    if v != v:
        return "0/0"
    if v == 0 and struct.pack(">d", v)[0] & 0x80:
        return "-0"  # negative zero prints differently
    if v in (float("inf"), float("-inf")):
        return "math.huge" if v > 0 else "-math.huge"
    if v == int(v) and abs(v) < 1e15:
        return str(int(v))
    r = repr(v)
    return {"inf": "math.huge", "-inf": "-math.huge", "nan": "0/0"}.get(r, r)


class Fn:
    """One proto decompiled to Lua-like source."""

    def __init__(self, mod, idx):
        self.mod = mod
        self.p = mod["protos"][idx]
        self.idx = idx
        p = self.p
        self.by_pc = {c[0]: c for c in p.code}
        self.order = [c[0] for c in p.code]

    # -- constants
    def kstr(self, i):
        t, v = self.p.k[i]
        if t == "string":
            return v.decode("utf-8", "replace") if v is not None else ""
        return None

    def kexpr(self, i):
        if i >= len(self.p.k):
            return f"K{i}"
        t, v = self.p.k[i]
        if t == "nil":
            return "nil"
        if t == "bool":
            return "true" if v else "false"
        if t in ("number",):
            return fmt_number(v)
        if t == "integer":
            return str(v)
        if t == "string":
            return lua_string(v) if v is not None else '""'
        if t == "import":
            return self.import_expr(v)
        if t in ("vector", "vector64"):
            return "Vector3.new(%s)" % ", ".join(fmt_number(x) for x in v[:3])
        if t == "closure":
            return f"<function {v}>"
        if t in ("table", "tablek"):
            return "{" + ", ".join(self.template_fields(i)) + "}"
        return f"K{i}"

    def template_fields(self, i):
        """Fields a DUPTABLE template already holds (type 8 keeps values)."""
        t, v = self.p.k[i]
        if t != "tablek":
            return []
        out = []
        for key, val in v:
            name = self.kstr(key) if key < len(self.p.k) else None
            if name is None or val < 0:
                continue
            field = name if IDENT.match(name) and name not in KEYWORDS else "[" + lua_string(name.encode()) + "]"
            out.append(f"{field} = {self.kexpr(val)}")
        return out

    def import_expr(self, iid):
        count = iid >> 30
        ids = [(iid >> 20) & 1023, (iid >> 10) & 1023, iid & 1023][:count]
        parts = [self.kstr(i) or f"K{i}" for i in ids]
        return parts[0] + "".join(self.index_suffix(x) for x in parts[1:])

    @staticmethod
    def index_suffix(name):
        if IDENT.match(name) and name not in KEYWORDS:
            return "." + name
        return "[" + lua_string(name.encode()) + "]"

    def reg(self, r):
        return self.regname.get(r, f"v{r}" + getattr(self, "suffix", ""))

    # -- disassembly
    def disasm(self):
        p = self.p
        out = [f"; function {self.idx} {p.name or '(anon)'} params={p.numparams} "
               f"upvals={p.nups} vararg={p.vararg} maxstack={p.maxstack}"]
        for pc, op, w, aux, n in p.code:
            a, b, c = (w >> 8) & 0xFF, (w >> 16) & 0xFF, (w >> 24) & 0xFF
            d = struct.unpack("<h", struct.pack("<H", w >> 16))[0]
            note = ""
            name = op_name(op)
            if name in ("LOADK", "DUPTABLE", "DUPCLOSURE"):
                note = self.kexpr(d)
            elif name == "LOADKX":
                note = self.kexpr(aux)
            elif name in ("GETGLOBAL", "SETGLOBAL", "GETTABLEKS", "SETTABLEKS", "NAMECALL"):
                note = self.kexpr(aux)
            elif name == "GETIMPORT":
                note = self.import_expr(aux)
            elif name.startswith("JUMP") or name.startswith("FOR"):
                note = f"-> {pc + 1 + d}"
            out.append(f"  {pc:4d} {name:<15} A={a} B={b} C={c} D={d}"
                       + (f" AUX={aux:#x}" if n == 2 else "") + (f"  ; {note}" if note else ""))
        return "\n".join(out)


# --------------------------------------------------------------- decompiler

class Expr:
    """A value with its source text and binding strength (for parentheses)."""

    def __init__(self, text, prec=100, multi=False, pure=False):
        self.text = text
        self.prec = prec
        self.multi = multi
        self.pure = pure  # no side effects, unaffected by any call
        self.hint = None  # a natural local name for this value
        self.fields = None  # table constructor fields, while still being built

    def render(self):
        if self.fields is None:
            return self.text
        if not self.fields:
            return "{}"
        return "{" + ", ".join(self.fields) + "}"

    def wrap(self, prec):
        if prec >= 100 and (self.text.startswith(('"', "{", "function")) or self.text == "..." or
                            self.text[:1].isdigit() or self.text.startswith("-")):
            return f"({self.text})"  # a literal cannot be called or indexed bare
        return self.text if self.prec >= prec else f"({self.text})"


CONST_LIKE = re.compile(r'^(-?[0-9][0-9.e+-]*|-?math\.huge|nil|true|false|"[^"]*"|'
                        r'([A-Za-z_][A-Za-z0-9_]*\.)*[A-Z][A-Z0-9_]+)$')
MIRROR = {"<": ">", "<=": ">=", "==": "==", "~=": "~="}


def make_cmp(lhs, sym, rhs, negated=False):
    """`lhs sym rhs` (or its negation). Written constant-last, as people do."""
    a, b, s = lhs, rhs, sym
    asked = negated
    if CONST_LIKE.match(a.text) and not CONST_LIKE.match(b.text):
        a, b, s = b, a, MIRROR[s]
    if negated and s in ("==", "~="):
        s, negated = ("~=" if s == "==" else "=="), False
    text = f"{a.wrap(4)} {s} {b.wrap(4)}"
    e = Expr(f"not ({text})", 8) if negated else Expr(text, 3)
    e.cmp = (lhs, sym, rhs, asked)  # what it means, not how it is written
    return e


class RegionFail(Exception):
    pass


def is_boolean(e):
    if getattr(e, "cmp", None) is not None or e.text in ("true", "false") or             (e.text.startswith("not ") and e.prec == 8):
        return True
    logic = getattr(e, "logic", None)
    return logic is not None and is_boolean(logic[1]) and is_boolean(logic[2])


def if_expr(d, cond, a, b):
    """`if cond then a else b` as one expression; an else that is itself an
    if-expression continues as elseif. Exact simplifications only."""
    if a.text == "true" and b.text == "false" and is_boolean(cond):
        e = Expr(cond.text, cond.prec)
        e.cmp, e.logic = getattr(cond, "cmp", None), getattr(cond, "logic", None)
    elif a.text == "false" and b.text == "true" and is_boolean(cond):
        e = d.negate(cond)
    else:
        def val(v):
            return f"({v.text})" if v.text.startswith("if ") else v.text
        parts = [(cond, a)]
        tail = b
        while getattr(tail, "ifparts", None):
            parts += tail.ifparts
            tail = tail.ifelse
        # same value in adjacent branches: one branch, conditions or'ed
        merged = []
        for c, v in parts:
            if merged and merged[-1][1].text == v.text:
                merged[-1] = (mk_logic("or", merged[-1][0], c), merged[-1][1])
            else:
                merged.append((c, v))
        parts = merged
        # a last branch giving the else value: drop it (when its test has no call)
        while parts and parts[-1][1].text == tail.text and "(" not in parts[-1][0].text:
            parts.pop()
        # if A then x elseif B then y else x  ->  if not A and B then y else x
        if len(parts) == 2 and parts[0][1].text == tail.text:
            parts = [(mk_logic("and", d.negate(parts[0][0]), parts[1][0]), parts[1][1])]
        if not parts:
            return tail
        if len(parts) == 1 and parts[0][1].text == "true" and tail.text == "false" and is_boolean(parts[0][0]):
            return parts[0][0]
        text = f"if {parts[0][0].text} then {val(parts[0][1])}"
        for c, v in parts[1:]:
            text += f" elseif {c.text} then {val(v)}"
        text += f" else {val(tail)}"
        e = Expr(text, 0)
        e.ifparts, e.ifelse = parts, tail
    e.uses = getattr(a, "uses", set()) | getattr(b, "uses", set()) | {id(a), id(b)}
    return e


def mk_logic(op, left, right):
    """`left and right` / `left or right`, remembering its parts."""
    same = getattr(right, "logic", None) is not None and right.logic[0] == op
    if op == "and":
        # `a and (b and c)` is `a and b and c`: same value, same order
        e = Expr(f"{left.wrap(2)} and {right.text if same else right.wrap(3)}", 2)
    else:
        e = Expr(f"{left.wrap(1)} or {right.text if same else right.wrap(2)}", 1)
    e.logic = (op, left, right)
    return e


BINOPS = {"ADD": ("+", 6), "SUB": ("-", 6), "MUL": ("*", 7), "DIV": ("/", 7),
          "MOD": ("%", 7), "POW": ("^", 9), "IDIV": ("//", 7)}


def effects(insn, maxstack):
    """(reads, writes, barrier) register sets of one instruction."""
    pc, op, w, aux, n = insn
    name = op_name(op)
    a, b, c = (w >> 8) & 0xFF, (w >> 16) & 0xFF, (w >> 24) & 0xFF
    R, W = set(), set()
    barrier = False
    if name in ("LOADNIL", "LOADB", "LOADN", "LOADK", "LOADKX", "GETGLOBAL",
                "GETUPVAL", "GETIMPORT", "NEWCLOSURE", "DUPCLOSURE",
                "NEWTABLE", "DUPTABLE"):
        W.add(a)
    elif name in ("MOVE", "GETTABLEKS", "GETTABLEN", "NOT", "MINUS", "LENGTH",
                  "ADDK", "SUBK", "MULK", "DIVK", "MODK", "POWK", "IDIVK",
                  "ANDK", "ORK"):
        R.add(b); W.add(a)
    elif name in ("GETTABLE", "ADD", "SUB", "MUL", "DIV", "MOD", "POW", "IDIV",
                  "AND", "OR"):
        R |= {b, c}; W.add(a)
    elif name in ("SUBRK", "DIVRK"):
        R.add(c); W.add(a)
    elif name == "CONCAT":
        R |= set(range(b, c + 1)); W.add(a)
    elif name == "NAMECALL":
        R.add(b); W |= {a, a + 1}
    elif name in ("CALL", "CALLFB"):
        R |= set(range(a, (a + b) if b else maxstack))
        W |= set(range(a, a + c - 1)) if c else {a}
    elif name == "RETURN":
        R |= set(range(a, (a + b - 1) if b else maxstack))
    elif name in ("SETGLOBAL", "SETUPVAL", "JUMPIF", "JUMPIFNOT",
                  "JUMPXEQKNIL", "JUMPXEQKB", "JUMPXEQKN", "JUMPXEQKS"):
        R.add(a)
    elif name in ("JUMPIFEQ", "JUMPIFLE", "JUMPIFLT", "JUMPIFNOTEQ",
                  "JUMPIFNOTLE", "JUMPIFNOTLT"):
        R |= {a, aux & 0xFF}
    elif name in ("SETTABLEKS", "SETTABLEN"):
        R |= {a, b}
    elif name == "SETTABLE":
        R |= {a, b, c}
    elif name == "SETLIST":
        R |= {a} | set(range(b, (b + c - 1) if c else maxstack))
    elif name == "CAPTURE":
        if a != 2:
            R.add(b)
    elif name == "GETVARARGS":
        W |= set(range(a, a + b - 1)) if b else {a}
    elif name.startswith("FOR"):
        barrier = True
    return R, W, barrier


def list_texts(exprs, fixed):
    """Texts of an argument/return/constructor list. In a list of fixed
    length the last value was cut to one value; a call or `...` there would
    expand again, so it is parenthesized."""
    texts = [e.text for e in exprs]
    if fixed and exprs and getattr(exprs[-1], "one", False):
        texts[-1] = f"({texts[-1]})"
    return texts


def twice(insn, r):
    """Does this instruction read register r through two operands?"""
    pc, op, w, aux, n = insn
    name = op_name(op)
    a, b, c = (w >> 8) & 0xFF, (w >> 16) & 0xFF, (w >> 24) & 0xFF
    if name in ("GETTABLE", "ADD", "SUB", "MUL", "DIV", "MOD", "POW", "IDIV", "AND", "OR"):
        return b == c == r
    if name in ("SETTABLE",):
        return [a, b, c].count(r) > 1
    if name in ("JUMPIFEQ", "JUMPIFLE", "JUMPIFLT", "JUMPIFNOTEQ", "JUMPIFNOTLE", "JUMPIFNOTLT"):
        return a == (aux & 0xFF) == r
    if name == "SETTABLEKS" or name == "SETTABLEN":
        return a == b == r
    return False


def module_globals(mod):
    """Names the script uses as globals: no local may take one."""
    if "_globals" not in mod:
        names = set()
        for p in mod["protos"]:
            f = Fn(mod, mod["protos"].index(p))
            for pc, op, w, aux, n in p.code:
                name = op_name(op)
                if name in ("GETGLOBAL", "SETGLOBAL"):
                    s = f.kstr(aux)
                    if s:
                        names.add(s)
                elif name == "GETIMPORT":
                    names.add(f.import_expr(aux).split(".")[0])
        mod["_globals"] = names
    return mod["_globals"]


def self_pos(order, pc):
    return order.index(pc)


def jump_target(insn):
    pc, op, w, aux, n = insn
    name = op_name(op)
    if name == "JUMPX":
        e = w >> 8
        return pc + 1 + (e - (1 << 24) if e & 0x800000 else e)
    if name == "LOADB":
        c = (w >> 24) & 0xFF
        return pc + 1 + c if c else None
    d = struct.unpack("<h", struct.pack("<H", w >> 16))[0]
    if name.startswith("JUMP") or name.startswith("FOR"):
        return pc + 1 + d
    return None


def successors(insn):
    pc, op, w, aux, n = insn
    name = op_name(op)
    t = jump_target(insn)
    if name == "RETURN":
        return []
    if name in ("JUMP", "JUMPBACK", "JUMPX") or name.startswith("FORGPREP"):
        return [t]
    if name == "LOADB":
        return [t] if t is not None else [pc + n]
    if t is not None:
        return [pc + n, t]
    return [pc + n]


def full_effects(insn, maxstack):
    """effects() with FOR loops' register use spelled out (for liveness)."""
    pc, op, w, aux, n = insn
    name = op_name(op)
    a = (w >> 8) & 0xFF
    if name in ("FORNPREP", "FORGPREP", "FORGPREP_INEXT", "FORGPREP_NEXT"):
        return {a, a + 1, a + 2}, set()
    if name == "FORNLOOP":
        return {a, a + 1, a + 2}, {a + 2}
    if name == "FORGLOOP":
        nv = max(aux & 0xFF, 1)
        return {a, a + 1, a + 2}, set(range(a + 2, a + 3 + nv))
    R, W, _ = effects(insn, maxstack)
    return R, W


class KeyedDict(dict):
    """A dict keyed by variable; an int key is a register, looked up as the
    variable it holds where the decompiler currently is."""

    def __init__(self, owner, *a):
        super().__init__(*a)
        self.owner = owner

    def _k(self, k):
        return self.owner.var_key(k) if isinstance(k, int) else k

    def __getitem__(self, k):
        return super().__getitem__(self._k(k))

    def __setitem__(self, k, v):
        super().__setitem__(self._k(k), v)

    def __contains__(self, k):
        return super().__contains__(self._k(k))

    def get(self, k, d=None):
        return super().get(self._k(k), d)

    def pop(self, k, *d):
        return super().pop(self._k(k), *d)


class KeyedSet(set):
    """A set of variables, indexed by register like KeyedDict."""

    def __init__(self, owner, *a):
        super().__init__(*a)
        self.owner = owner

    def _k(self, k):
        return self.owner.var_key(k) if isinstance(k, int) else k

    def add(self, k):
        super().add(self._k(k))

    def discard(self, k):
        super().discard(self._k(k))

    def __contains__(self, k):
        return super().__contains__(self._k(k))


class Decompiler(Fn):
    @property
    def regname(self):
        return self._regname

    @regname.setter
    def regname(self, v):
        self._regname = v if isinstance(v, KeyedDict) else KeyedDict(self, v)

    @property
    def declared(self):
        return self._declared

    @declared.setter
    def declared(self, v):
        self._declared = v if isinstance(v, KeyedSet) else KeyedSet(self, v)

    def __init__(self, mod, idx, upvals=None, depth=0):
        super().__init__(mod, idx)
        self.depth = depth
        self.upvals = upvals or [f"upval{i}" for i in range(self.p.nups)]
        self.cur_pc = 0
        self.key_pc = None  # set while naming at an instruction not being emitted
        self.webs_on = True
        self.flat_regs = False  # name by register (bounded count of locals)
        self.compute_webs()
        self.regname = {}
        self.declared = set()
        p = self.p
        # Nested functions get their own name space so captured outer names
        # never collide with the inner function's registers.
        self.suffix = "" if depth < 0 else "_" + str(depth + 1)
        self.param_names = []
        for i in range(p.numparams):
            key = self.entry_key(i)
            self.regname[key] = f"arg{i + 1}{self.suffix}"
            self.declared.add(key)
            self.regname[("r", i)] = f"arg{i + 1}{self.suffix}"  # register-named mode
            self.declared.add(("r", i))
            self.param_names.append(f"arg{i + 1}{self.suffix}")
        # named locals from debug info, when the script kept it
        self.local_at = {}
        for name, s, e, reg in p.locals:
            if name:
                self.local_at.setdefault(s, []).append((reg, name))
        for name, s, e, reg in p.locals:
            if name and reg < p.numparams and s == 0:
                self.regname[reg] = name
        self.once_cache = {}
        self.fresh = 0
        self.cur_pc = 0
        self.hoisted = []  # locals first set in a nested block
        self.exits = []  # where control goes after each range being emitted
        self.loops = []  # (continue pc, break pc) of the loops being emitted
        self.goto_targets = set()  # pcs a printed goto names
        self.label_pcs = set()     # labels to print (from a first pass)
        self.backward = {}         # loop head pc -> pc of its backward jump
        self.repeat_back = {}      # repeat-until head pc -> its conditional jump back
        self.pending = {}  # reg -> Expr not yet written out
        self.used_names = set(self.upvals) | module_globals(mod)
        self.labels = set()
        self.collect_labels()
        self.compute_liveness()
        self.incoming = {}  # jump target -> number of jumps to it
        for insn in self.p.code:
            jt = jump_target(insn)
            if jt is not None and op_name(insn[1]) not in ("FORNLOOP", "FORGLOOP", "FORNPREP")                     and not op_name(insn[1]).startswith("FORGPREP"):
                self.incoming[jt] = self.incoming.get(jt, 0) + 1
        self.consumed = {}  # jumps already folded into expressions

    def compute_webs(self):
        """Reaching definitions; the writes reaching a common read are one
        variable. use_web/def_web map (pc, register) to that variable."""
        p = self.p
        order = self.order
        defs = []           # (pc, register); pc -1 is the function entry
        gen = {}
        reads = {}
        writes = {}
        top = None
        for insn in p.code:
            R, W = full_effects(insn, top if top is not None else p.maxstack)
            qname = op_name(insn[1])
            qa = (insn[2] >> 8) & 0xFF
            if (qname in ("CALL", "CALLFB") and (insn[2] >> 24) & 0xFF == 0) or \
                    (qname == "GETVARARGS" and (insn[2] >> 16) & 0xFF == 0):
                top = qa + 1
            elif qname in ("CALL", "CALLFB", "RETURN", "SETLIST"):
                top = None
            pc = insn[0]
            reads[pc] = R
            writes[pc] = W
            g = 0
            for r in W:
                g |= 1 << len(defs)
                defs.append((pc, r))
            gen[pc] = g
        nregs = max(p.maxstack, 1) + 1
        self.entry_def = {}
        entry_mask = 0
        for r in range(nregs):
            self.entry_def[r] = len(defs)
            entry_mask |= 1 << len(defs)
            defs.append((-1, r))
        of_reg = {}
        for i, (pc, r) in enumerate(defs):
            of_reg[r] = of_reg.get(r, 0) | (1 << i)
        kill = {pc: 0 for pc in order}
        for pc in order:
            for r in writes[pc]:
                kill[pc] |= of_reg.get(r, 0)
        preds = {pc: [] for pc in order}
        for insn in p.code:
            for s in successors(insn):
                if s in preds:
                    preds[s].append(insn[0])
        IN = {pc: 0 for pc in order}
        OUT = {pc: 0 for pc in order}
        changed = True
        while changed:
            changed = False
            for k, pc in enumerate(order):
                i = entry_mask if k == 0 else 0
                for q in preds[pc]:
                    i |= OUT[q]
                o = gen[pc] | (i & ~kill[pc])
                if i != IN[pc] or o != OUT[pc]:
                    IN[pc], OUT[pc] = i, o
                    changed = True
        parent = list(range(len(defs)))

        def find(x):
            while parent[x] != x:
                parent[x] = parent[parent[x]]
                x = parent[x]
            return x

        def union(a, b):
            a, b = find(a), find(b)
            if a != b:
                parent[b] = a

        def bits(m):
            while m:
                low = m & -m
                yield low.bit_length() - 1
                m ^= low

        first_use = {}
        def_index = {d: i for i, d in enumerate(defs)}
        for pc in order:
            for r in reads[pc]:
                ids = list(bits(IN[pc] & of_reg.get(r, 0)))
                if not ids:
                    continue
                for x in ids[1:]:
                    union(ids[0], x)
                first_use[(pc, r)] = ids[0]
                if r in writes[pc]:
                    union(ids[0], def_index[(pc, r)])  # `x = x + 1`: one variable
        byref_groups = []
        # a register a closure captures by reference is one variable shared
        # with the closure from the capture until its upvalue is closed
        # (CLOSEUPVALS at or below it, or the function's end); after that the
        # register is free for unrelated values
        for insn in p.code:
            if op_name(insn[1]) == "CAPTURE" and (insn[2] >> 8) & 0xFF == 1:
                r = (insn[2] >> 16) & 0xFF
                c = insn[0]
                group = list(bits(IN[c] & of_reg.get(r, 0)))
                for q in order[self_pos(order, c) + 1:]:
                    qi = self.by_pc[q]
                    qn = op_name(qi[1])
                    if qn == "CLOSEUPVALS" and (qi[2] >> 8) & 0xFF <= r:
                        break
                    if r in writes[q]:
                        group.append(def_index[(q, r)])
                if not group:
                    continue  # nothing written to it yet: nothing to share
                for x in group[1:]:
                    union(group[0], x)
                byref_groups.append(group[0])
        self.def_web = {d: ("w", find(i)) for i, d in enumerate(defs)}
        self.byref_webs = {("w", find(g)) for g in byref_groups}
        self.byref_only = set(self.byref_webs)  # shared with a closure by reference
        # a closure refers to the variables it captures (even by value), so
        # in register-named output those variables keep names of their own
        for insn in p.code:
            if op_name(insn[1]) == "CAPTURE" and (insn[2] >> 8) & 0xFF == 0:
                r = (insn[2] >> 16) & 0xFF
                if (insn[0], r) in first_use:
                    self.byref_webs.add(("w", find(first_use[(insn[0], r)])))
        self.web_defs = {}
        for d, w in self.def_web.items():
            if d[0] >= 0:
                self.web_defs[w] = self.web_defs.get(w, 0) + 1
        self.use_web = {k: ("w", find(i)) for k, i in first_use.items()}
        self.def_pcs = {}
        for pc, r in defs:
            if pc >= 0:
                self.def_pcs.setdefault(r, []).append(pc)
        self.pos = {pc: k for k, pc in enumerate(order)}

    def entry_key(self, r):
        return self.def_web[(-1, r)] if self.webs_on else ("r", r)

    def nearest_def_key(self, r, pc, strict=False):
        """The variable of the last write to r at (or before) pc."""
        best = None
        k = self.pos.get(pc, None)
        for d in self.def_pcs.get(r, ()):
            dk = self.pos[d]
            if (dk < k if strict else dk <= k) if k is not None else d <= pc:
                if best is None or dk > self.pos[best]:
                    best = d
        return self.def_web[(best, r)] if best is not None else self.def_web[(-1, r)]

    def prev_key(self, r, key):
        """The variable register r held before the variable `key` began."""
        mine = [self.pos[d] for d in self.def_pcs.get(r, ()) if self.def_web[(d, r)] == key]
        if not mine:
            return None
        first = min(mine)
        best = None
        for d in self.def_pcs.get(r, ()):
            k = self.pos[d]
            if k < first and self.def_web[(d, r)] != key and (best is None or k > self.pos[best]):
                best = d
        return self.def_web[(best, r)] if best is not None else self.def_web[(-1, r)]

    def var_key(self, r):
        """The variable register r stands for at the current instruction."""
        if not self.webs_on:
            return ("r", r)
        pc = self.key_pc if self.key_pc is not None else self.cur_pc
        if getattr(self, "flat_regs", False):
            # too many variables for one function: name registers, except
            # variables a closure shares (those must stay apart)
            w = self.use_web.get((pc, r)) or self.def_web.get((pc, r))
            return w if w in self.byref_webs else ("r", r)
        w = self.use_web.get((pc, r)) or self.def_web.get((pc, r))
        if w is not None:
            return w
        if (-1, r) not in self.def_web:
            return ("r", r)
        return self.nearest_def_key(r, pc)

    def compute_liveness(self):
        """live_out[pc]: bit set of registers some path after pc reads before
        writing. Registers captured by reference count as always live."""
        p = self.p
        rw = {}
        self.byref = set()
        top = None  # multi-value reads go up to the last multi-value result
        for insn in p.code:
            R, W = full_effects(insn, top if top is not None else p.maxstack)
            qname = op_name(insn[1])
            qa = (insn[2] >> 8) & 0xFF
            if (qname in ("CALL", "CALLFB") and (insn[2] >> 24) & 0xFF == 0) or \
                    (qname == "GETVARARGS" and (insn[2] >> 16) & 0xFF == 0):
                top = qa + 1
            elif qname in ("CALL", "CALLFB", "RETURN", "SETLIST"):
                top = None
            rw[insn[0]] = (sum(1 << r for r in R), sum(1 << r for r in W))
            if op_name(insn[1]) == "CAPTURE" and (insn[2] >> 8) & 0xFF == 1:
                self.byref.add((insn[2] >> 16) & 0xFF)
        succ = {insn[0]: [s for s in successors(insn) if s in self.by_pc] for insn in p.code}
        live_in = {pc: 0 for pc in self.order}
        live_out = {pc: 0 for pc in self.order}
        changed = True
        while changed:
            changed = False
            for pc in reversed(self.order):
                o = 0
                for s in succ[pc]:
                    o |= live_in[s]
                r, w = rw[pc]
                i = r | (o & ~w)
                if o != live_out[pc] or i != live_in[pc]:
                    live_out[pc], live_in[pc] = o, i
                    changed = True
        self.live_in, self.live_out = live_in, live_out

    def live_after(self, pc, r):
        return self.shared(pc, r) or bool(self.live_out.get(pc, 0) >> r & 1)

    def live_at(self, pc, r):
        return self.use_web.get((pc, r)) in self.byref_only or \
            bool(self.live_in.get(pc, 0) >> r & 1)

    def shared(self, pc, r):
        """Is the variable written or read at pc in r shared with a closure
        (captured by reference, before its upvalue is closed)?"""
        w = self.def_web.get((pc, r)) or self.use_web.get((pc, r))
        return w in self.byref_only

    def collect_labels(self):
        for pc, op, w, aux, n in self.p.code:
            name = op_name(op)
            d = struct.unpack("<h", struct.pack("<H", w >> 16))[0]
            if name in ("JUMP", "JUMPBACK", "JUMPIF", "JUMPIFNOT") or name.startswith("JUMPIF") or name.startswith("JUMPXEQ"):
                self.labels.add(pc + 1 + d)
            elif name == "JUMPX":
                e = w >> 8
                if e & 0x800000:
                    e -= 1 << 24
                self.labels.add(pc + 1 + e)
            elif name == "LOADB" and ((w >> 24) & 0xFF) and (w >> 24) & 0xFF != 1:
                self.labels.add(pc + 1 + ((w >> 24) & 0xFF))
            if name in ("JUMPBACK", "JUMP") and pc + 1 + d <= pc:
                head = pc + 1 + d
                self.backward[head] = max(self.backward.get(head, -1), pc)
            elif self.is_cond_jump(name) and pc + 1 + d <= pc:
                head = pc + 1 + d
                self.repeat_back[head] = max(self.repeat_back.get(head, -1), pc)

    # -- register values
    def get(self, r):
        if r in self.pending:
            e = self.pending.pop(r)
            if e.fields is not None:
                e.text = e.render()
                e.fields = None
            return e
        return Expr(self.reg(r), pure=True)

    def peek(self, r):
        return self.pending.get(r)

    def name_for(self, r):
        if r not in self.regname:
            name = f"v{r}{self.suffix}"
            k = 2
            while name in self.used_names:
                name = f"v{r}x{k}{self.suffix}"
                k += 1
            self.used_names.add(name)
            self.regname[r] = name
        return self.regname[r]

    def flush(self, out, ind, keep=()):
        """Write out pending values that are still live (register stores)."""
        for r in sorted(self.pending):
            if r in keep or getattr(self.pending[r], "method", False):
                continue  # a method lookup waits for its call
            e = self.get(r)
            self.assign(out, ind, r, e)

    NAME_FROM = [
        re.compile(r':GetService\("([A-Za-z_][A-Za-z0-9_]*)"\)$'),
        re.compile(r'^require\(.*[.:](?:WaitForChild|FindFirstChild)\("([A-Za-z_][A-Za-z0-9_]*)"\)\)$'),
        re.compile(r'^require\(.*\.([A-Za-z_][A-Za-z0-9_]*)\)$'),
        re.compile(r'[.:](?:WaitForChild|FindFirstChild)\("([A-Za-z_][A-Za-z0-9_]*)"\)$'),
    ]

    FIELD_TAIL = re.compile(r'^[A-Za-z_][A-Za-z0-9_.]*\.([A-Za-z_][A-Za-z0-9_]*)$')

    def nice_name(self, text, hint=None):
        if hint and IDENT.match(hint) and hint not in KEYWORDS and hint not in self.used_names:
            return hint
        m = self.FIELD_TAIL.match(text)
        if m and m.group(1) not in KEYWORDS and m.group(1) not in self.used_names \
                and not m.group(1)[0].islower():
            return m.group(1)
        for rx in self.NAME_FROM:
            m = rx.search(text)
            if m and m.group(1) not in KEYWORDS and m.group(1) not in self.used_names:
                return m.group(1)
        return None

    def assign(self, out, ind, r, e):
        old_key = self.key_pc
        dk = getattr(e, "def_key", None)
        if dk is not None and self.webs_on and dk[0] == r:
            r_key = dk[1]
            if getattr(self, "flat_regs", False) and r_key not in self.byref_webs:
                r_key = ("r", r)  # named by register in this mode
        else:
            r_key = self.var_key(r)
        try:
            self.key_pc = None
            self.assign_key(out, ind, r, r_key, e)
        finally:
            self.key_pc = old_key

    def assign_key(self, out, ind, r, key, e):
        if self.webs_on and key not in self.regname:
            # x = f(x): a new live range of a variable that is still around
            prev = self.prev_key(r, key)
            pname = dict.get(self.regname, prev) if prev is not None else None
            if pname and prev != key and re.search(rf"(?<![A-Za-z0-9_.:]){re.escape(pname)}(?![A-Za-z0-9_])", e.text):
                dict.__setitem__(self.regname, key, pname)
                if set.__contains__(self.declared, prev):
                    set.add(self.declared, key)
        current = dict.get(self.regname, key)
        generic = current is None or re.match(r"^v\d+(x\d+)?(_\d+)?$", current)
        nice = self.nice_name(e.text, e.hint)
        if nice and self.webs_on and self.web_defs.get(key, 1) > 1 and                 not getattr(e, "folded", False):
            nice = None  # set in several places: one value's name would mislead
        if nice and not set.__contains__(self.declared, key) and generic:
            dict.__setitem__(self.regname, key, nice)
            self.used_names.add(nice)
        if dict.get(self.regname, key) is None:
            name = f"v{r}{self.suffix}"
            k = 2
            while name in self.used_names:
                name = f"v{r}x{k}{self.suffix}"
                k += 1
            self.used_names.add(name)
            dict.__setitem__(self.regname, key, name)
        name = dict.get(self.regname, key)
        if set.__contains__(self.declared, key):
            out.append(f"{ind}{name} = {e.text}")
        elif len(ind) > self.base_indent:
            # first set inside a branch or loop: declared at the top instead,
            # so later code outside the block still sees it
            set.add(self.declared, key)
            self.hoisted.append(name)
            out.append(f"{ind}{name} = {e.text}")
        else:
            set.add(self.declared, key)
            out.append(f"{ind}local {name} = {e.text}")

    def assign_unused(self, out, ind, r, e):
        name = self.name_for(r)
        if r in self.declared:
            out.append(f"{ind}{name} = {e.text}")
        elif len(ind) > self.base_indent:
            # first set inside a branch or loop: declared at the top instead,
            # so later code outside the block still sees it
            self.declared.add(r)
            self.hoisted.append(name)
            out.append(f"{ind}{name} = {e.text}")
        else:
            self.declared.add(r)
            out.append(f"{ind}local {name} = {e.text}")

    def put(self, out, ind, r, e, inline=True):
        """Give register r the value e. Simple values wait to be inlined."""
        if r in self.pending:
            # the old value was never read: it was a statement in its own right
            old = self.pending.pop(r)
            if old.text.endswith(")") and not old.text.startswith(("function", "(")) \
                    and "(" in old.text:
                out.append(f"{ind}{old.text}")
        if inline and (self.read_once(self.cur_pc, r) or (
                getattr(self, "in_region", False) and self.reaches_join_unread(self.cur_pc, r))):
            if self.webs_on:
                # the variable this value was computed for, at this register
                e.def_key = (r, self.def_web.get((self.cur_pc, r)) or self.var_key(r))
            self.pending[r] = e
        else:
            self.flush_impure(out, ind, r)
            self.assign(out, ind, r, e)

    def flush_impure(self, out, ind, r):
        """A statement is about to be written: values computed before it that
        could have side effects (calls, field reads) go out first, so the
        order things happen in is kept."""
        for q in sorted(self.pending):
            e = self.pending[q]
            if q != r and not e.pure and e.fields is None and \
                    not getattr(e, "method", False) and e.text != "<self>" and \
                    not getattr(e, "multret", False):
                self.assign(out, ind, q, self.get(q))

    # -- body
    def decompile(self):
        params, body = self.decompile_once()
        if not self.flat_regs:
            base = "    " * (self.depth + 1)
            count = 0
            for l in "\n".join(body).split("\n"):
                m = re.match(rf"^{base}local (?:function )?([A-Za-z0-9_, ]+?)(?: =|\(|$)", l)
                if m:
                    count += len(m.group(1).split(","))
            if count > 180:
                # Luau allows 200 locals in scope: one variable per register
                # (shared closure variables kept apart) stays under that
                again = Decompiler(self.mod, self.idx, self.upvals, self.depth)
                again.flat_regs = True
                return again.decompile_once()
        return params, body

    def decompile_once(self):
        p = self.p
        params = [self.reg(i) for i in range(p.numparams)]
        if p.vararg:
            params.append("...")
        # Two passes: the first learns which jumps stay gotos, so the second
        # can print their labels.
        saved = (dict(self.regname), set(self.declared), set(self.used_names))
        self.base_indent = 0
        self.emit_range([], "", 0, p.sizecode)
        targets = set(self.goto_targets)
        self.regname, self.declared, self.used_names = saved
        self.pending, self.goto_targets, self.loops = {}, set(), []
        self.consumed = {}
        self.label_pcs = targets
        self.hoisted = []
        out = []
        base = "    " * (self.depth + 1)
        self.base_indent = len(base)
        self.emit_range(out, base, 0, p.sizecode)
        if self.hoisted:
            names = list(dict.fromkeys(self.hoisted))
            out.insert(0, f"{base}local {', '.join(names)}")
        if self.goto_targets and not os.environ.get("LUAU_NOFLAT"):
            fixed = resolve_gotos("\n".join(out).split("\n"))
            if fixed is not None:
                return params, wrap_mid_block_exits(fixed)
            # Luau has no goto: this function's jumps did not all nest, so it
            # is written as blocks dispatched by number instead
            return params, self.decompile_flat(saved, base)
        return params, wrap_mid_block_exits(out)

    def index_of(self, pc):
        return self.order.index(pc)

    def emit_range(self, out, ind, start, end, loop_head=None, exit=None):
        self.exits.append(end if exit is None else exit)
        before = set(self.declared)
        try:
            self.emit_range_body(out, ind, start, end, loop_head)
        finally:
            self.exits.pop()
        if len(ind) > self.base_indent >= 0:
            # a `local` inside this block ends with it; hoisted names live on
            hoisted = set(self.hoisted)
            for r in self.declared - before:
                if self.regname.get(r) not in hoisted:
                    self.declared.discard(r)

    def range_exit(self):
        return self.exits[-1] if self.exits else None

    def emit_range_body(self, out, ind, start, end, loop_head=None):
        pc = start
        while pc < end:
            if pc not in self.by_pc:
                pc += 1
                continue
            if pc in self.labels and pc != start and                     self.consumed.get(pc, 0) < self.incoming.get(pc, 0):
                self.flush(out, ind)
            if pc in self.label_pcs:
                self.flush(out, ind)
                out.append(f"{ind}::L{pc}::")
            rep_back = self.repeat_back.get(pc)
            if rep_back is not None and rep_back < end and pc != loop_head and \
                    self.backward.get(pc) is None:
                # `repeat ... until c`: the loop's last instruction jumps back
                # to its head while c does not hold
                n = self.by_pc[rep_back][4]
                self.flush(out, ind)
                out.append(f"{ind}repeat")
                self.loops.append((rep_back, rep_back + n))
                body = []
                self.emit_range(body, ind + "    ", pc, rep_back, loop_head=pc, exit=rep_back)
                cond, _ = self.cond_expr(rep_back)
                self.flush(body, ind + "    ")
                self.loops.pop()
                out.extend(body)
                out.append(f"{ind}until {self.negate(cond).text}")
                pc = rep_back + n
                continue
            back = self.backward.get(pc)
            if back is not None and back < end and pc != loop_head:
                # a backward jump closes a loop that starts here
                self.flush(out, ind)
                out.append(f"{ind}while true do")
                self.loops.append((pc, back + 1))
                self.emit_range(out, ind + "    ", pc, back + 1, loop_head=pc, exit=pc)
                self.flush(out, ind + "    ")
                self.loops.pop()
                out.append(f"{ind}end")
                pc = back + 1
                continue
            nxt = self.stmt(out, ind, pc, end)
            pc = nxt

    def cond_expr(self, pc):
        """The condition under which the jump at pc is taken, and its target."""
        _, op, w, aux, n = self.by_pc[pc]
        name = op_name(op)
        a = (w >> 8) & 0xFF
        d = struct.unpack("<h", struct.pack("<H", w >> 16))[0]
        target = pc + 1 + d
        self.cur_pc = pc
        if name == "JUMPIF":
            return self.get(a), target
        if name == "JUMPIFNOT":
            return Expr("not " + self.get(a).wrap(8), 8), target
        if name in ("JUMPIFEQ", "JUMPIFLE", "JUMPIFLT", "JUMPIFNOTEQ", "JUMPIFNOTLE", "JUMPIFNOTLT"):
            rhs = self.get(aux & 0xFF)
            lhs = self.get(a)
            sym = {"JUMPIFEQ": "==", "JUMPIFLE": "<=", "JUMPIFLT": "<",
                   "JUMPIFNOTEQ": "~=", "JUMPIFNOTLE": "<=", "JUMPIFNOTLT": "<"}[name]
            return make_cmp(lhs, sym, rhs, name in ("JUMPIFNOTLE", "JUMPIFNOTLT")), target
        if name.startswith("JUMPXEQK"):
            neg = bool(aux >> 31)
            if name == "JUMPXEQKNIL":
                k = "nil"
            elif name == "JUMPXEQKB":
                k = "true" if aux & 1 else "false"
            else:
                k = self.kexpr(aux & 0xFFFFFF)
            lhs = self.get(a)
            return make_cmp(lhs, "~=" if neg else "==", Expr(k, pure=True)), target
        return None, None

    def snapshot(self):
        return (dict(self.pending), set(self.declared), dict(self.regname),
                set(self.used_names), self.fresh)

    def restore(self, s):
        (self.pending, self.declared, self.regname, self.used_names, self.fresh) = (
            dict(s[0]), set(s[1]), dict(s[2]), set(s[3]), s[4])

    def value_of(self, start, stop, r, ind, end):
        """Run [start, stop) as pure expression code; the value it leaves in r."""
        q = start
        scratch = []
        old_join = getattr(self, "join_ok", None)
        self.join_ok = stop  # the value meets the other branch's there
        try:
            return self.value_of_body(q, stop, r, ind, end, scratch)
        finally:
            self.join_ok = old_join

    def value_of_body(self, q, stop, r, ind, end, scratch):
        while q < stop:
            if q not in self.by_pc:
                return None
            qname = op_name(self.by_pc[q][1])
            if self.is_cond_jump(qname) or qname in ("JUMP", "JUMPBACK", "JUMPX", "RETURN") \
                    or qname.startswith("FOR"):
                folded = self.bool_value(q, q + self.by_pc[q][4], scratch, ind)                     if self.is_cond_jump(qname) else None
                if folded is None or folded > stop:
                    return None
                q = folded
                continue
            q = self.stmt(scratch, ind, q, end)
            if scratch:
                return None
        if q != stop or r not in self.pending:
            return None
        return self.pending.pop(r)

    def and_or_value(self, pc, nxt, end):
        """`x = a or b`: JUMPIF R -> J over code computing b into R.
        `x = a and b`: the same with JUMPIFNOT.
        `x = c and a or b`: jump c -> M; a into R; JUMP/JUMPIF R -> J; M: b into R."""
        _, op, w, aux, n = self.by_pc[pc]
        name = op_name(op)
        a = (w >> 8) & 0xFF
        d = struct.unpack("<h", struct.pack("<H", w >> 16))[0]
        target = pc + 1 + d
        if not nxt < target <= end:
            return None
        ind = ""
        if name in ("JUMPIF", "JUMPIFNOT") and a in self.pending:
            # P1: the tested register is the result
            s = self.snapshot()
            first = self.pending.pop(a)
            rest = self.value_of(nxt, target, a, ind, end)
            if rest is not None:
                sym, prec = ("or", 1) if name == "JUMPIF" else ("and", 2)
                self.pending[a] = Expr(f"{first.wrap(prec)} {sym} {rest.wrap(prec + 1)}", prec)
                self.consumed[target] = self.consumed.get(target, 0) + 1
                return target
            self.restore(s)
        # P2: the jump skips a branch that ends by jumping on to the join
        last = self.last_insn_before(target)
        if last is None or last <= pc:
            return None
        _, lop, lw, laux, ln = self.by_pc[last]
        lname = op_name(lop)
        if lname not in ("JUMP", "JUMPIF"):
            return None
        ld = struct.unpack("<h", struct.pack("<H", lw >> 16))[0]
        join = last + 1 + ld
        if not target < join <= end:
            return None
        s = self.snapshot()
        cond, _ = self.cond_expr(pc)
        enter = self.negate(cond)  # the branch [nxt, last) runs when this holds
        reg = (lw >> 8) & 0xFF if lname == "JUMPIF" else None
        if reg is None:
            # JUMP form: find the register both branches write last
            cands = [r for r in range(self.p.maxstack)]
            reg = None
            for r in cands:
                s2 = self.snapshot()
                x = self.value_of(nxt, last, r, ind, end)
                self.restore(s2)
                if x is not None:
                    reg = r
                    break
            if reg is None:
                self.restore(s)
                return None
        x = self.value_of(nxt, last, reg, ind, end)
        if x is None:
            self.restore(s)
            return None
        y = self.value_of(target, join, reg, ind, end)
        if y is None:
            self.restore(s)
            return None
        self.pending[reg] = Expr(f"{enter.wrap(3)} and {x.wrap(3)} or {y.wrap(2)}", 1)
        self.consumed[join] = self.consumed.get(join, 0) + 1
        return join

    def bool_value(self, pc, nxt, out=None, ind="", dry=False):
        """`local x = a == b` compiles to a jump over two LOADBs:
            JUMPcond -> T ; LOADB r X skip-1 ; T: LOADB r Y
        so r = Y when the jump's condition holds, else X."""
        _, op, w, aux, n = self.by_pc[pc]
        d = struct.unpack("<h", struct.pack("<H", w >> 16))[0]
        target = pc + 1 + d
        if target != nxt + 1 or nxt not in self.by_pc or target not in self.by_pc:
            return None
        _, op1, w1, _, _ = self.by_pc[nxt]
        _, op2, w2, _, _ = self.by_pc[target]
        if op_name(op1) != "LOADB" or op_name(op2) != "LOADB":
            return None
        r1, x, skip = (w1 >> 8) & 0xFF, (w1 >> 16) & 0xFF, (w1 >> 24) & 0xFF
        r2, y, skip2 = (w2 >> 8) & 0xFF, (w2 >> 16) & 0xFF, (w2 >> 24) & 0xFF
        if r1 != r2 or skip != 1 or skip2 != 0 or bool(x) == bool(y):
            return None
        if dry:
            return target + 1
        cond, _ = self.cond_expr(pc)
        self.cur_pc = target  # the value is complete after the second LOADB
        self.put(out if out is not None else [], ind, r1, cond if y else self.negate(cond))
        return target + 1

    def read_once(self, pc, r):
        """True when the value written to r at pc has exactly one use: the
        first read on the straight-line path after pc, with r dead after it
        (liveness covers every branch). No other path may join in between,
        and no loop may start there (it would be computed on every pass)."""
        join = getattr(self, "join_ok", None)
        key = (pc, r, join)
        if key in self.once_cache:
            return self.once_cache[key]
        result = False
        if not self.shared(pc, r):
            top = None  # first register past the last multi-value result
            i = self.order.index(pc) + 1
            while i < len(self.order):
                q = self.order[i]
                insn = self.by_pc[q]
                if (q in self.labels and q != join) or q in self.backward:
                    break  # another path joins here
                R, W, barrier = effects(insn, top if top is not None else self.p.maxstack)
                qname = op_name(insn[1])
                qa = (insn[2] >> 8) & 0xFF
                if r in R and twice(insn, r):
                    break  # `x * x`: two reads in one instruction
                if (qname == "FORNPREP" or qname.startswith("FORGPREP")) and                         qa <= r <= qa + 2:
                    result = True  # a loop's start/limit/step or iterator: its header
                    break
                if r in R:
                    if qname in ("JUMPIF", "JUMPIFNOT") and qa == r:
                        # `x = a or b` reads a here and keeps it on the jump
                        # path; and_or_value folds that, and the plain if
                        # path writes it out first. The fall-through path
                        # must not need it.
                        result = not self.live_at(q + insn[4], r)
                    else:
                        result = r in W or not self.live_after(q, r)
                    break
                if barrier or r in W:
                    break
                if (qname in ("CALL", "CALLFB") and (insn[2] >> 24) & 0xFF == 0) or \
                        (qname == "GETVARARGS" and (insn[2] >> 16) & 0xFF == 0):
                    top = qa + 1
                elif qname in ("CALL", "CALLFB", "RETURN", "SETLIST"):
                    top = None
                succ = successors(insn)
                if succ != [q + insn[4]]:
                    # a branch: fine only if no other way out still needs r
                    if qname == "RETURN" or any(
                            s != q + insn[4] and self.live_at(s, r) for s in succ) or \
                            q + insn[4] not in succ:
                        break
                i += 1
        self.once_cache[key] = result
        return result

    def finish_table(self, out, ind, r, table, pc):
        """A constructor whose table is used more than once is complete:
        it becomes the table's variable."""
        if getattr(table, "build_end", None) == pc and self.pending.get(r) is table:
            self.pending.pop(r)
            table.text = table.render()
            table.fields = None
            self.flush_impure(out, ind, r)
            self.assign(out, ind, r, table)

    def table_fills(self, pc, r):
        """(keyed stores, array stores, pc of the last one) into the new
        table at r before its first other use."""
        keyed = listed = False
        last = None
        i = self.order.index(pc) + 1
        while i < len(self.order):
            insn = self.by_pc[self.order[i]]
            qname = op_name(insn[1])
            qa, qb = (insn[2] >> 8) & 0xFF, (insn[2] >> 16) & 0xFF
            if qname in ("SETTABLEKS", "SETTABLE", "SETTABLEN") and qb == r and qa != r:
                keyed = True
                last = self.order[i]
            elif qname == "SETLIST" and qa == r:
                listed = True
                last = self.order[i]
            else:
                R, W, barrier = effects(insn, self.p.maxstack)
                if r in R or r in W or barrier or self.order[i] in self.labels:
                    break
            i += 1
        return keyed, listed, last

    def table_filled_then_read(self, pc, r):
        """A table set up field by field, then used once (and dead after
        that use, on every path): still one value, its constructor."""
        if self.shared(pc, r):
            return False
        i = self.order.index(pc) + 1
        while i < len(self.order):
            q = self.order[i]
            insn = self.by_pc[q]
            qname = op_name(insn[1])
            qa, qb = (insn[2] >> 8) & 0xFF, (insn[2] >> 16) & 0xFF
            if q in self.labels or q in self.backward:
                return False  # another path joins while it is being built
            if qname in ("SETTABLEKS", "SETTABLEN") and qb == r and qa != r:
                i += 1
                continue
            if qname == "SETLIST" and qa == r:
                i += 1
                continue
            R, W, barrier = effects(insn, self.p.maxstack)
            if r in R:
                return not twice(insn, r) and (r in W or not self.live_after(q, r))
            if barrier or r in W or successors(insn) != [q + insn[4]]:
                return False
            i += 1
        return False

    def shared_tail(self, target, end):
        """The block starting at target, when it is short, straight-line and
        flows on to where the current block exits anyway."""
        exit_pc = self.range_exit()
        if exit_pc is None or not (end <= target < exit_pc) or self.loops and \
                exit_pc in (self.loops[-1][0], self.loops[-1][1]) and False:
            return None
        count = 0
        q = target
        while q < exit_pc:
            if q not in self.by_pc:
                q += 1
                continue
            insn = self.by_pc[q]
            qname = op_name(insn[1])
            if qname in ("JUMP", "JUMPBACK", "JUMPX") or qname.startswith("FOR") or \
                    qname in ("NEWCLOSURE", "DUPCLOSURE") or self.is_cond_jump(qname):
                return None
            count += 1
            if count > 12:
                return None
            q += insn[4]
        return exit_pc

    def decompile_flat(self, saved, base):
        p = self.p
        self.regname, self.declared, self.used_names = ({}, set(), set(saved[2]))
        for i, n in enumerate(self.param_names):
            key = self.entry_key(i)
            self.regname[key] = n
            self.declared.add(key)
        self.pending, self.loops, self.exits = {}, [], []
        self.consumed = {}
        self.flat = True
        self.base_indent = -1  # no renaming or hoisting: every name is fixed
        # every variable is declared up front (blocks are siblings, so no
        # block can declare one another reads); Luau allows 200 locals
        nvars = len({k for (dpc, r), k in self.def_web.items() if dpc >= 0})
        self.flat_regs = self.flat_regs or nvars > 150
        if self.flat_regs:
            regs = []
            for i, n in enumerate(self.param_names):
                self.regname[("r", i)] = n  # parameters, named by register here too
                self.declared.add(("r", i))
            for r in range(p.numparams, max(p.maxstack, 1)):
                key = ("r", r)
                name = f"v{r}{self.suffix}"
                self.used_names.add(name)
                self.regname[key] = name
                self.declared.add(key)
                regs.append(name)
            for key in sorted(self.byref_webs):
                rr = next(r for (dpc, r), k in self.def_web.items() if k == key)
                name = f"v{rr}x{len(regs)}{self.suffix}"
                self.used_names.add(name)
                self.regname[key] = name
                self.declared.add(key)
                regs.append(name)
        regs = regs if self.flat_regs else []
        for (dpc, r), key in sorted(self.def_web.items(), key=lambda kv: (kv[0][1], kv[0][0])):
            if self.flat_regs or key in self.declared or key in self.regname:
                continue
            name = f"v{r}{self.suffix}"
            k = 2
            while name in self.used_names:
                name = f"v{r}x{k}{self.suffix}"
                k += 1
            self.used_names.add(name)
            self.regname[key] = name
            self.declared.add(key)
            if dpc >= 0 or any(w == key for w in self.use_web.values()):
                regs.append(name)
        leaders = {0}
        for pc, op, w, aux, n in p.code:
            name = op_name(op)
            d = struct.unpack("<h", struct.pack("<H", w >> 16))[0]
            if self.is_cond_jump(name) or name in ("JUMP", "JUMPBACK", "FORNPREP",
                                                   "FORNLOOP", "FORGLOOP", "FORGPREP",
                                                   "FORGPREP_INEXT", "FORGPREP_NEXT"):
                leaders.add(pc + 1 + d)
                if name in ("FORNPREP", "FORGPREP", "FORGPREP_INEXT", "FORGPREP_NEXT"):
                    leaders.add(pc + n)  # the loop body is jumped back to
            elif name == "JUMPX":
                e = w >> 8
                leaders.add(pc + 1 + (e - (1 << 24) if e & 0x800000 else e))
            elif name == "LOADB" and (w >> 24) & 0xFF:
                leaders.add(pc + 1 + ((w >> 24) & 0xFF))
        starts = sorted(x for x in leaders if x in self.by_pc)
        out = []
        if regs:
            out.append(f"{base}local {', '.join(regs)}")
        out.append(f"{base}local block = 0")
        out.append(f"{base}while true do")
        ind1, ind2 = base + "    ", base + "        "
        for i, s in enumerate(starts):
            e = starts[i + 1] if i + 1 < len(starts) else p.sizecode
            out.append(f"{ind1}{'if' if i == 0 else 'elseif'} block == {s} then")
            body = []
            pc = s
            ended = False
            while pc < e and not ended:
                if pc not in self.by_pc:
                    pc += 1
                    continue
                insn = self.by_pc[pc]
                pc, ended = self.flat_stmt(body, ind2, insn, e)
            self.flush(body, ind2)
            if not ended:
                if e < p.sizecode:
                    body.append(f"{ind2}block = {e}")
                else:
                    body.append(f"{ind2}return")
            out.extend(body if body else [f"{ind2}-- (empty)"])
        out.append(f"{ind1}end")
        out.append(f"{base}end")
        return wrap_mid_block_exits(out)

    def flat_stmt(self, out, ind, insn, end):
        """One instruction in dispatcher form. Returns (next pc, block ended)."""
        pc, op, w, aux, n = insn
        name = op_name(op)
        a, b, c = (w >> 8) & 0xFF, (w >> 16) & 0xFF, (w >> 24) & 0xFF
        d = struct.unpack("<h", struct.pack("<H", w >> 16))[0]
        nxt = pc + n
        self.cur_pc = pc
        go = lambda target: f"block = {target} continue"
        if self.is_cond_jump(name):
            if name in ("JUMPIF", "JUMPIFNOT") and a in self.pending and                     self.live_at(pc + 1 + d, a):
                self.assign(out, ind, a, self.pending.pop(a))  # the target reads it too
            cond, target = self.cond_expr(pc)
            self.flush(out, ind)
            out.append(f"{ind}if {cond.text} then {go(target)} end")
            return nxt, False
        if name in ("JUMP", "JUMPBACK", "JUMPX"):
            if name == "JUMPX":
                e = w >> 8
                d = e - (1 << 24) if e & 0x800000 else e
            self.flush(out, ind)
            out.append(f"{ind}{go(pc + 1 + d)}")
            return nxt, True
        if name == "LOADB" and c:
            self.put(out, ind, a, Expr("true" if b else "false", pure=True), inline=False)
            self.flush(out, ind)
            out.append(f"{ind}{go(pc + 1 + c)}")
            return nxt, True
        lim, step, idx = self.reg(a), self.reg(a + 1), self.reg(a + 2)
        within = f"({step} > 0 and {idx} <= {lim} or {step} <= 0 and {idx} >= {lim})"
        if name == "FORNPREP":
            self.flush(out, ind)
            out.append(f"{ind}if not {within} then {go(pc + 1 + d)} end")
            return nxt, False
        if name == "FORNLOOP":
            self.flush(out, ind)
            out.append(f"{ind}{idx} = {idx} + {step}")
            out.append(f"{ind}if {within} then {go(pc + 1 + d)} end")
            return nxt, False
        if name in ("FORGPREP", "FORGPREP_INEXT", "FORGPREP_NEXT"):
            self.flush(out, ind)
            gen, state, ctrl = self.reg(a), self.reg(a + 1), self.reg(a + 2)
            out.append(f"{ind}if type({gen}) == \"table\" then {gen}, {state}, {ctrl} = next, {gen}, nil end")
            out.append(f"{ind}{go(pc + 1 + d)}")
            return nxt, True
        if name == "FORGLOOP":
            self.flush(out, ind)
            nvars = max(aux & 0xFF, 1)
            gen, state, ctrl = self.reg(a), self.reg(a + 1), self.reg(a + 2)
            vars_ = [self.reg(a + 3 + i) for i in range(nvars)]
            out.append(f"{ind}{', '.join(vars_)} = {gen}({state}, {ctrl})")
            out.append(f"{ind}if {vars_[0]} ~= nil then {ctrl} = {vars_[0]} {go(pc + 1 + d)} end")
            return nxt, False
        if name == "RETURN":
            self.stmt(out, ind, pc, end)
            if not out or not out[-1].lstrip().startswith("return"):
                out.append(f"{ind}return")
            return nxt, True
        return self.stmt(out, ind, pc, end), False

    def read_once_from(self, J, r):
        """read_once for a value that arrives at J (from several paths)."""
        prev = self.labels
        self.labels = set(prev) - {J}
        try:
            self.once_cache.pop(("from", J, r), None)
            # read_once scans from the instruction after its pc; start just
            # before J by scanning from a pretend predecessor
            insn = self.by_pc[J]
            R, W, barrier = effects(insn, self.p.maxstack)
            if r in R:
                return not twice(insn, r) and (r in W or not self.live_after(J, r))
            if r in W or barrier:
                return False
            return self.read_once(J, r)
        finally:
            self.labels = prev

    def reaches_join_unread(self, pc, r):
        """Inside a value region: does the value written to r at pc go
        straight to the region's join, unread on the way?"""
        J = getattr(self, "region_join", None)
        q = pc + self.by_pc[pc][4]
        steps = 0
        while J is not None and steps < 64:
            steps += 1
            if q == J:
                return True
            if q not in self.by_pc or q > J:
                return False
            insn = self.by_pc[q]
            nm = op_name(insn[1])
            R, W = full_effects(insn, self.p.maxstack)
            if r in R or r in W:
                return False
            if nm in ("JUMP", "JUMPX"):
                t = jump_target(insn)
                if t <= q:
                    return False
                q = t
                continue
            if self.is_cond_jump(nm) or nm == "RETURN" or nm.startswith("FOR") or                     (nm == "LOADB" and (insn[2] >> 24) & 0xFF):
                return False
            q += insn[4]
        return False

    # -- values computed through conditions ------------------------------
    def value_region(self, pc, end):
        """A run of tests whose every path only computes one value into the
        same register, meeting at a join: one Luau if-expression
        (`if a then x elseif b then y else z`). Inlined validators and
        `a and b or c` chains of any shape compile to this."""
        if getattr(self, "in_region", False) or getattr(self, "flat", False):
            return None
        joins = []
        q = pc
        while q < end and len(joins) < 8 and q - pc < 300:
            if q not in self.by_pc:
                break
            insn = self.by_pc[q]
            nm = op_name(insn[1])
            if nm in ("RETURN",) or nm.startswith("FOR") or q in self.backward or \
                    q in self.repeat_back:
                break
            t = jump_target(insn)
            if t is not None and t > pc and t <= end and t not in joins and \
                    (self.is_cond_jump(nm) or nm in ("JUMP", "JUMPX") or nm == "LOADB"):
                joins.append(t)
            q += insn[4]
        for J in sorted(joins):
            saved = self.snapshot()
            before = dict(self.pending)
            self.in_region = True
            self.region_steps = 0
            self.region_written = set()
            self.region_join = J
            try:
                got = self.walk_value(pc, J, before)
            except RegionFail:
                got = None
            finally:
                self.in_region = False
            if got is not None:
                R, expr = got
                tested = getattr(self, "region_tested", set())
                if self.live_at(J, R) and not any(
                        self.live_at(J, r) for r in range(self.p.maxstack)
                        if r != R and r in self.region_written and r not in before):
                    defs = [x for x in self.order if pc <= x < J and
                            R in full_effects(self.by_pc[x], self.p.maxstack)[1]]
                    if defs and self.webs_on:
                        expr.def_key = (R, self.def_web.get((defs[0], R)) or self.var_key(R))
                    if self.read_once_from(J, R):
                        self.pending[R] = expr
                    else:
                        # used more than once after the join: a variable
                        region_out = []
                        self.cur_pc = defs[0] if defs else pc
                        self.flush_impure(region_out, self.region_ind, R)
                        self.assign(region_out, self.region_ind, R, expr)
                        self.region_lines = region_out
                    inside = sum(1 for x in self.order if pc <= x < J and
                                 jump_target(self.by_pc[x]) == J)
                    self.consumed[J] = self.consumed.get(J, 0) + inside
                    return J
            self.restore(saved)
        return None

    def walk_value(self, q, J, before):
        """Follow the code from q to J; returns (register, value) when every
        path computes exactly one new value, in the same register."""
        while True:
            self.region_steps += 1
            if self.region_steps > 400:
                raise RegionFail()
            if q == J:
                new = [r for r, e in self.pending.items() if before.get(r) is not e]
                if len(new) != 1:
                    raise RegionFail()
                R = new[0]
                return R, self.pending.pop(R)
            if q > J or q not in self.by_pc or q in self.backward or q in self.repeat_back:
                raise RegionFail()
            insn = self.by_pc[q]
            nm = op_name(insn[1])
            a = (insn[2] >> 8) & 0xFF
            n = insn[4]
            if self.is_cond_jump(nm):
                tested = self.pending.get(a) if nm in ("JUMPIF", "JUMPIFNOT") else None
                cond, T = self.cond_expr(q)
                F = q + n
                if T <= q or T > J or cond is None:
                    raise RegionFail()
                if tested is not None and self.live_at(F, a):
                    raise RegionFail()  # the other path would read the tested value
                snap = self.snapshot()
                if tested is not None:
                    self.pending[a] = tested  # the jump keeps it in its register
                rT, vT = self.walk_value(T, J, before)
                self.restore(snap)
                rF, vF = self.walk_value(F, J, before)
                if rT != rF:
                    raise RegionFail()
                if tested is not None and vT is tested:
                    if nm == "JUMPIF":
                        return rT, mk_logic("or", tested, vF)
                    return rT, mk_logic("and", tested, vF)
                if tested is not None and (getattr(vT, "uses", set()) | getattr(vF, "uses", set())) \
                        & {id(tested)}:
                    raise RegionFail()
                return rT, if_expr(self, cond, vT, vF)
            if nm in ("JUMP", "JUMPX"):
                t = jump_target(insn)
                if t <= q:
                    raise RegionFail()
                q = t
                continue
            if nm == "LOADB" and (insn[2] >> 24) & 0xFF:
                b = (insn[2] >> 16) & 0xFF
                self.pending[a] = Expr("true" if b else "false", pure=True)
                self.region_written.add(a)
                q = q + 1 + ((insn[2] >> 24) & 0xFF)
                continue
            if nm in ("RETURN", "SETTABLEKS", "SETTABLE", "SETTABLEN", "SETGLOBAL", "SETUPVAL",
                      "NEWCLOSURE", "DUPCLOSURE", "CAPTURE", "SETLIST", "NEWTABLE", "DUPTABLE") \
                    or nm.startswith("FOR"):
                raise RegionFail()
            scratch = []
            W = full_effects(insn, self.p.maxstack)[1]
            q2 = self.stmt(scratch, "", q, J)
            if scratch:
                raise RegionFail()
            self.region_written |= set(W)
            q = q2

    def cond_block(self, s):
        """The conditional jump ending the pure expression code at s, if the
        code from s up to it only computes values the condition consumes."""
        q = s
        while q in self.by_pc:
            qname = op_name(self.by_pc[q][1])
            if q != s and q in self.labels:
                return None  # another path joins inside: not one block
            if self.is_cond_jump(qname):
                if self.bool_value(q, q + self.by_pc[q][4], dry=True) is not None:
                    return None
                return q
            if qname in ("JUMP", "JUMPBACK", "JUMPX", "RETURN", "NEWCLOSURE", "DUPCLOSURE",
                         "SETTABLEKS", "SETTABLE", "SETTABLEN", "SETGLOBAL", "SETUPVAL",
                         "NEWTABLE", "DUPTABLE", "SETLIST", "CAPTURE", "LOADB") \
                    or qname.startswith("FOR"):
                return None
            q += self.by_pc[q][4]
        return None

    def compound_condition(self, pc, end):
        """Short-circuit conditions (`a and (b or not c)`) compile to a run of
        conditional jumps. Treat the run as a graph of tests, each with a
        taken and a fall-through exit, and merge a test into the one before
        it while they share an exit:
            X(c) taken -> B, B(b) shares X's fall-through  =>  c and b
            X(c) falls -> B, B(b) shares X's taken exit    =>  c or b
        (with `not b` when B's exits are the other way round). When the run
        reduces to one test whose fall-through is the code right after it,
        that test is the if's condition. Returns (taken condition, taken
        target, then-start) or None."""
        if getattr(self, "flat", False):
            return None
        # the contiguous run of test blocks starting at pc
        blocks = [(pc, pc)]  # (start, jump pc)
        q = pc + self.by_pc[pc][4]
        while q < end and len(blocks) < 16:
            j = self.cond_block(q)
            if j is None or j >= end:
                break
            blocks.append((q, j))
            q = j + self.by_pc[j][4]
        if len(blocks) < 2:
            return None
        for count in range(len(blocks), 1, -1):
            saved = self.snapshot()
            got = self.reduce_tests(blocks[:count], end)
            if got is not None:
                return got
            self.restore(saved)
        return None

    def reduce_tests(self, blocks, end):
        nodes = {}
        order = []
        for start, j in blocks:
            scratch = []
            q = start
            while q < j:
                q = self.stmt(scratch, "", q, end)
            if scratch:
                return None  # the code before a test was a statement
            jname = op_name(self.by_pc[j][1])
            ja = (self.by_pc[j][2] >> 8) & 0xFF
            if jname in ("JUMPIF", "JUMPIFNOT") and any(
                    self.live_at(s, ja) for s in successors(self.by_pc[j])):
                return None  # `x and v or w`: the tested value is the result
            cond, taken = self.cond_expr(j)
            if cond is None:
                return None
            nodes[start] = [cond, taken, j + self.by_pc[j][4]]
            order.append(start)
        region_end = blocks[-1][1] + self.by_pc[blocks[-1][1]][4]
        preds = {}
        for s in nodes:
            for t in nodes[s][1:]:
                preds[t] = preds.get(t, 0) + 1
        for s in nodes:
            # an edge from outside the run makes a test shared: not mergeable
            if s != order[0] and self.incoming.get(s, 0) + (1 if any(
                    nodes[x][2] == s for x in nodes) else 0) != preds.get(s, 0):
                return None
        changed = True
        while changed and len(nodes) > 1:
            changed = False
            for x in list(order):
                if x not in nodes:
                    continue
                c, T, F = nodes[x]
                for slot, b in (("T", T), ("F", F)):
                    if b not in nodes or b == x or preds.get(b, 0) != 1:
                        continue
                    bc, bT, bF = nodes[b]
                    new = None
                    if slot == "T" and bF == F:
                        new = (mk_logic("and", c, bc), bT, F, F)
                    elif slot == "T" and bT == F:
                        new = (mk_logic("and", c, self.negate(bc)), bF, F, F)
                    elif slot == "F" and bT == T:
                        new = (mk_logic("or", c, bc), T, bF, T)
                    elif slot == "F" and bF == T:
                        new = (mk_logic("or", c, self.negate(bc)), T, bT, T)
                    if new is None:
                        continue
                    expr, nT, nF, shared = new
                    nodes[x] = [expr, nT, nF]
                    del nodes[b]
                    order.remove(b)
                    preds[shared] -= 1  # X and B both led there; now only X
                    changed = True
                    break
                if changed:
                    break
        if len(nodes) != 1:
            return None
        c, T, F = nodes[order[0]]
        if F == region_end:
            return c, T, region_end
        if T == region_end:
            return self.negate(c), F, region_end
        return None

    def jump_word(self, target):
        """break / continue when the target is the enclosing loop's edge."""
        if self.loops:
            cont, brk = self.loops[-1]
            if target == brk:
                return "break"
            if target == cont:
                return "continue"
        return None

    def is_cond_jump(self, name):
        return name in ("JUMPIF", "JUMPIFNOT") or (name.startswith("JUMPIF") and name != "JUMPIF") \
            or name.startswith("JUMPXEQK")

    @staticmethod
    def negate(e):
        cmp = getattr(e, "cmp", None)
        if cmp is not None:
            return make_cmp(cmp[0], cmp[1], cmp[2], not cmp[3])
        logic = getattr(e, "logic", None)
        if logic is not None:
            # De Morgan, when it leaves fewer `not`s than wrapping the whole
            op, l, r = logic
            dm = mk_logic("or" if op == "and" else "and",
                          Decompiler.negate(l), Decompiler.negate(r))
            plain = Expr(f"not ({e.text})", 8)
            plain.logic_neg = e
            return dm if dm.text.count("not ") < plain.text.count("not ") else plain
        if getattr(e, "logic_neg", None) is not None:
            return e.logic_neg
        t = e.text
        if t.startswith("not ") and e.prec == 8:
            inner = t[4:]
            if inner.startswith("(") and inner.endswith(")") and balanced(inner[1:-1]):
                return Expr(inner[1:-1], 2)
            return Expr(inner, 100)
        for a, b in ((" == ", " ~= "), (" ~= ", " == ")):
            if a in t and t.count(" ") == 2:
                return Expr(t.replace(a, b), 3)
        return Expr("not " + e.wrap(8), 8)

    def stmt(self, out, ind, pc, end):
        self.cur_pc = pc
        _, op, w, aux, n = self.by_pc[pc]
        name = op_name(op)
        a, b, c = (w >> 8) & 0xFF, (w >> 16) & 0xFF, (w >> 24) & 0xFF
        d = struct.unpack("<h", struct.pack("<H", w >> 16))[0]
        nxt = pc + n
        if pc in self.local_at:
            pass

        # ---- control flow -------------------------------------------------
        if self.is_cond_jump(name) and not getattr(self, "flat", False):
            folded = self.bool_value(pc, nxt, out, ind)
            if folded is not None:
                return folded
            folded = self.and_or_value(pc, nxt, end)
            if folded is not None:
                return folded
            self.region_ind = ind
            self.region_lines = None
            folded = self.value_region(pc, end)
            if folded is not None:
                if self.region_lines:
                    out.extend(self.region_lines)
                return folded
            if name in ("JUMPIF", "JUMPIFNOT") and a in self.pending and \
                    self.live_at(pc + 1 + d, a):
                # `x = a or b` that did not fold: a still flows to the target
                self.assign(out, ind, a, self.pending.pop(a))
            comp = self.compound_condition(pc, end)
            if comp is not None:
                cond, target, nxt = comp
            else:
                cond, target = self.cond_expr(pc)
            word = self.jump_word(target)
            if word:
                self.flush(out, ind)
                out.append(f"{ind}if {cond.text} then {word} end")
                return nxt
            if start_ok(target, nxt, end):
                # `if a and b then`: every part jumps to the same place when it
                # fails. Pure expression code between the parts folds in.
                enter = [self.negate(cond)]
                cur = nxt
                while True:
                    saved = (dict(self.pending), set(self.declared),
                             dict(self.regname), set(self.used_names))
                    q, scratch, pure = cur, [], True
                    while q < target and q in self.by_pc:
                        qname = op_name(self.by_pc[q][1])
                        if self.is_cond_jump(qname):
                            break
                        if qname in ("JUMP", "JUMPBACK", "JUMPX", "RETURN", "NEWCLOSURE",
                                     "DUPCLOSURE", "SETTABLEKS", "SETTABLE", "SETTABLEN",
                                     "SETGLOBAL", "SETUPVAL", "NEWTABLE", "DUPTABLE", "SETLIST") \
                                or qname.startswith("FOR"):
                            pure = False
                            break
                        q = self.stmt(scratch, ind, q, end)
                        if scratch:
                            pure = False
                            break
                    if pure and q < target and q in self.by_pc and \
                            self.is_cond_jump(op_name(self.by_pc[q][1])):
                        qn = q + self.by_pc[q][4]
                        qi = self.by_pc[q]
                        qreg = (qi[2] >> 8) & 0xFF
                        value_test = op_name(qi[1]) in ("JUMPIF", "JUMPIFNOT") and any(
                            self.live_at(s, qreg) for s in successors(qi))
                        if self.bool_value(q, qn, dry=True) is None and not value_test:
                            c2, t2 = self.cond_expr(q)
                            if t2 == target:
                                enter.append(self.negate(c2))
                                cur = qn
                                continue
                            if qn == target and target < t2 <= end:
                                # if A and C2 then (body at t2) else (block at
                                # target, which then falls into t2): the block
                                # runs exactly when the chain fails
                                every = enter + [c2]
                                text = " and ".join(e.wrap(3) for e in every)
                                self.flush(out, ind)
                                out.append(f"{ind}if not ({text}) then")
                                # the block may end by jumping over an else
                                blast = self.last_insn_before(t2)
                                jt = None
                                if blast is not None and blast >= target:
                                    _, bop, bw, _, _ = self.by_pc[blast]
                                    bd = struct.unpack("<h", struct.pack("<H", bw >> 16))[0]
                                    cand = blast + 1 + bd
                                    if op_name(bop) == "JUMP" and t2 < cand and \
                                            (cand <= end or cand == self.range_exit()):
                                        jt = cand
                                if jt is None:
                                    self.emit_range(out, ind + "    ", target, t2)
                                    self.flush(out, ind + "    ")
                                    out.append(f"{ind}end")
                                    return t2
                                else_end = jt if jt <= end else end
                                self.emit_range(out, ind + "    ", target, blast, exit=jt)
                                self.flush(out, ind + "    ")
                                body = []
                                self.emit_range(body, ind + "    ", t2, else_end,
                                                exit=self.range_exit() if else_end == end else else_end)
                                self.flush(body, ind + "    ")
                                out.extend(merge_elseif(body, ind))
                                out.append(f"{ind}end")
                                return else_end
                    (self.pending, self.declared, self.regname, self.used_names) = saved
                    break
                if len(enter) > 1:
                    cond = self.negate(Expr(" and ".join(e.wrap(3) for e in enter), 2))
                    nxt = cur
                # forward conditional: an if (maybe with else)
                then_end = target
                else_end = None
                after = target  # where the then-branch continues
                last = self.last_insn_before(target)
                if last is not None and last >= nxt:
                    _, lop, lw, _, ln = self.by_pc[last]
                    ld = struct.unpack("<h", struct.pack("<H", lw >> 16))[0]
                    jt = last + 1 + ld
                    if op_name(lop) == "JUMP" and jt > target and jt <= end:
                        else_end = jt
                        then_end = last
                        after = jt
                    elif op_name(lop) == "JUMP" and jt > target and jt == self.range_exit():
                        # the branch leaves the enclosing block: everything
                        # after it in this block is the else
                        else_end = end
                        then_end = last
                        after = jt
                self.flush(out, ind)
                out.append(f"{ind}if {self.negate(cond).text} then")
                self.emit_range(out, ind + "    ", nxt, then_end, exit=after)
                self.flush(out, ind + "    ")
                if else_end is not None:
                    body = []
                    self.emit_range(body, ind + "    ", target, else_end,
                                    exit=self.range_exit() if else_end == end else else_end)
                    self.flush(body, ind + "    ")
                    out.extend(merge_elseif(body, ind))
                    out.append(f"{ind}end")
                    return else_end
                out.append(f"{ind}end")
                return target
            if target == self.range_exit() and target >= end:
                # an inner if whose end coincides with the enclosing block's
                # end: the compiler jumps straight there, so the rest of this
                # block is the if's body
                self.flush(out, ind)
                out.append(f"{ind}if {self.negate(cond).text} then")
                self.emit_range(out, ind + "    ", nxt, end, exit=target)
                self.flush(out, ind + "    ")
                out.append(f"{ind}end")
                return end
            tail = self.shared_tail(target, end)
            if tail is not None:
                # `if C then <shared tail> else <rest of block> end`: the tail
                # and the rest both end where this block exits
                self.flush(out, ind)
                out.append(f"{ind}if {cond.text} then")
                saved = self.snapshot()
                self.emit_range(out, ind + "    ", target, tail, exit=tail)
                self.flush(out, ind + "    ")
                self.restore(saved)
                body = []
                self.emit_range(body, ind + "    ", nxt, end, exit=self.range_exit())
                self.flush(body, ind + "    ")
                out.extend(merge_elseif(body, ind))
                out.append(f"{ind}end")
                return end
            self.flush(out, ind)
            self.goto_targets.add(target)
            out.append(f"{ind}if {cond.text} then goto L{target} end")
            return nxt
        if name in ("JUMP", "JUMPBACK", "JUMPX"):
            if name == "JUMPX":
                e = w >> 8
                d = e - (1 << 24) if e & 0x800000 else e
            target = pc + 1 + d
            self.flush(out, ind)
            if self.loops and target == self.loops[-1][0] and pc + n == self.loops[-1][1]:
                return nxt  # the loop's own closing jump
            if target == self.range_exit() and nxt >= end:
                return nxt  # leaves the block the way it would anyway
            word = self.jump_word(target)
            if word:
                out.append(f"{ind}{word}")
            elif target != nxt:
                self.goto_targets.add(target)
                out.append(f"{ind}goto L{target}")
            return nxt
        if name == "FORNPREP":
            target = pc + 1 + d
            loop_pc = self.last_insn_before(target)
            self.flush(out, ind, keep=(a, a + 1, a + 2))  # the header takes those
            var = self.name_for(a + 2)
            was_declared = a + 2 in self.declared
            self.declared.add(a + 2)
            out.append(f"{ind}for {var} = {self.reg(a + 2)}, {self.reg(a)}, {self.reg(a + 1)} do"
                       if False else
                       f"{ind}for {var} = {self.reg_text(a + 2)}, {self.reg_text(a)}, {self.reg_text(a + 1)} do")
            self.loops.append((loop_pc if loop_pc is not None else target, target))
            self.emit_range(out, ind + "    ", nxt, loop_pc if loop_pc is not None else target)
            self.flush(out, ind + "    ")
            self.loops.pop()
            if not was_declared:
                self.declared.discard(a + 2)  # the loop variable ends with the loop
            out.append(f"{ind}end")
            return target
        if name in ("FORGPREP", "FORGPREP_INEXT", "FORGPREP_NEXT"):
            loop_pc = pc + 1 + d  # the FORGLOOP
            if loop_pc in self.by_pc:
                _, lop, lw, laux, ln = self.by_pc[loop_pc]
                nvars = laux & 0xFF
                gen = [self.reg_text(a), self.reg_text(a + 1), self.reg_text(a + 2)]
                iter_call = getattr(self, "iter_call", None)
                self.iter_call = None
                self.flush(out, ind)
                vars_ = []
                fresh_vars = []
                self.key_pc = loop_pc
                for i in range(max(nvars, 1)):
                    r = a + 3 + i
                    vars_.append(self.name_for(r))
                    if r not in self.declared:
                        fresh_vars.append(self.var_key(r))
                    self.declared.add(r)
                self.key_pc = None
                head = gen[0]
                if name == "FORGPREP_INEXT":
                    head = f"ipairs({gen[1]})" if gen[0] in ("ipairs", self.reg(a)) else gen[0]
                elif name == "FORGPREP_NEXT":
                    head = f"pairs({gen[1]})" if gen[0] in ("pairs", "next", self.reg(a)) else gen[0]
                else:
                    head = ", ".join(x for x in gen if x != "nil") if gen[1] != "nil" else gen[0]
                if iter_call and iter_call[0] == a:
                    head = iter_call[1]
                out.append(f"{ind}for {', '.join(vars_)} in {head} do")
                self.loops.append((loop_pc, loop_pc + 2))
                self.emit_range(out, ind + "    ", nxt, loop_pc)
                self.flush(out, ind + "    ")
                self.loops.pop()
                for r in fresh_vars:
                    self.declared.discard(r)  # loop variables end with the loop
                out.append(f"{ind}end")
                return loop_pc + 2
        if name in ("FORNLOOP", "FORGLOOP"):
            return nxt

        # ---- plain statements ---------------------------------------------
        if name in ("NOP", "BREAK", "PREPVARARGS", "COVERAGE", "NATIVECALL"):
            return nxt
        if name in ("FASTCALL", "FASTCALL1", "FASTCALL2", "FASTCALL2K", "FASTCALL3",
                    "FASTPCALL"):
            return nxt  # a fast path; the CALL that follows says it all
        if name == "LOADNIL":
            self.put(out, ind, a, Expr("nil", pure=True))
        elif name == "LOADB":
            self.put(out, ind, a, Expr("true" if b else "false", pure=True))
            if c:
                self.flush(out, ind)
                self.goto_targets.add(pc + 1 + c)
                out.append(f"{ind}goto L{pc + 1 + c}")
        elif name == "LOADN":
            self.put(out, ind, a, Expr(str(d), pure=True))
        elif name == "LOADK":
            self.put(out, ind, a, Expr(self.kexpr(d), pure=True))
        elif name == "LOADKX":
            self.put(out, ind, a, Expr(self.kexpr(aux), pure=True))
        elif name == "MOVE":
            self.put(out, ind, a, self.get(b))
        elif name == "GETGLOBAL":
            self.put(out, ind, a, Expr(self.kstr(aux) or self.kexpr(aux)))
        elif name == "SETGLOBAL":
            self.flush(out, ind, keep=(a,))
            out.append(f"{ind}{self.kstr(aux)} = {self.get(a).text}")
        elif name == "GETUPVAL":
            self.put(out, ind, a, Expr(self.upvals[b] if b < len(self.upvals) else f"upval{b}",
                                       pure=True))
        elif name == "SETUPVAL":
            self.flush(out, ind, keep=(a,))
            out.append(f"{ind}{self.upvals[b] if b < len(self.upvals) else f'upval{b}'} = {self.get(a).text}")
        elif name == "CLOSEUPVALS":
            pass
        elif name == "GETIMPORT":
            self.put(out, ind, a, Expr(self.import_expr(aux), pure=True))
        elif name == "GETTABLE":
            obj = self.get(b)
            key = self.get(c)
            self.put(out, ind, a, Expr(f"{obj.wrap(100)}[{key.text}]"))
        elif name == "SETTABLE" and a != b and c != b and b in self.pending and                 self.pending[b].fields is not None:
            table = self.pending[b]
            key = self.get(c)
            val = self.get(a)
            self.pending[b] = table
            table.fields.append(f"[{key.text}] = {val.text}")
            self.finish_table(out, ind, b, table, pc)
        elif name == "SETTABLE":
            self.flush(out, ind, keep=(a, b, c))
            val = self.get(a)
            key = self.get(c)
            obj = self.get(b)
            self.flush(out, ind)
            out.append(f"{ind}{obj.wrap(100)}[{key.text}] = {val.text}")
        elif name == "GETTABLEKS":
            obj = self.get(b)
            self.put(out, ind, a, Expr(obj.wrap(100) + self.index_suffix(self.kstr(aux) or "?")))
        elif name == "SETTABLEKS" and a != b and b in self.pending and                 self.pending[b].fields is not None:
            key = self.kstr(aux) or "?"
            table = self.pending[b]
            val = self.get(a)
            self.pending[b] = table
            field = (key if IDENT.match(key) and key not in KEYWORDS
                     else "[" + lua_string(key.encode()) + "]")
            table.fields.append(f"{field} = {val.text}")
            self.finish_table(out, ind, b, table, pc)
        elif name == "SETTABLEKS":
            self.flush(out, ind, keep=(a, b))
            val = self.get(a)
            obj = self.get(b)
            self.flush(out, ind)
            out.append(f"{ind}{obj.wrap(100)}{self.index_suffix(self.kstr(aux) or '?')} = {val.text}")
        elif name == "GETTABLEN":
            obj = self.get(b)
            self.put(out, ind, a, Expr(f"{obj.wrap(100)}[{c + 1}]"))
        elif name == "SETTABLEN" and a != b and b in self.pending and \
                self.pending[b].fields is not None:
            table = self.pending[b]
            val = self.get(a)
            self.pending[b] = table
            table.fields.append(f"[{c + 1}] = {val.text}")
            self.finish_table(out, ind, b, table, pc)
        elif name == "SETTABLEN":
            self.flush(out, ind, keep=(a, b))
            val = self.get(a)
            obj = self.get(b)
            self.flush(out, ind)
            out.append(f"{ind}{obj.wrap(100)}[{c + 1}] = {val.text}")
        elif name in ("NEWCLOSURE", "DUPCLOSURE"):
            child = self.p.children[d] if name == "NEWCLOSURE" else self.p.k[d][1]
            caps = []
            q = nxt
            while q in self.by_pc and op_name(self.by_pc[q][1]) == "CAPTURE":
                _, _, cw, _, _ = self.by_pc[q]
                ctype, creg = (cw >> 8) & 0xFF, (cw >> 16) & 0xFF
                if ctype == 2:
                    caps.append(self.upvals[creg] if creg < len(self.upvals) else f"upval{creg}")
                else:
                    if creg == a:
                        # `local function f` that calls itself: name it now,
                        # so the body refers to it by that name
                        hint = self.mod["protos"][child].name
                        cur = self.regname.get(a)
                        if hint and IDENT.match(hint) and hint not in KEYWORDS and \
                                hint not in self.used_names and a not in self.declared and \
                                (cur is None or re.match(r"^v\d+(_\d+)?$", cur)):
                            self.regname[a] = hint
                            self.used_names.add(hint)
                    elif creg in self.pending:
                        self.assign(out, ind, creg, self.pending.pop(creg))
                    self.key_pc = q
                    caps.append(self.name_for(creg))
                    self.key_pc = None
                q += 1
            sub = Decompiler(self.mod, child, caps, self.depth + 1)
            fparams, body = sub.decompile()
            fname = self.mod["protos"][child].name
            # the body sits one level under the line that writes the closure
            shift = len(ind) + 4 - 4 * (self.depth + 2)
            if shift:
                body = [(" " * shift + l) if shift > 0 else l[min(-shift, indent_of(l)):]
                        for l in "\n".join(body).split("\n")]
            text = f"function({', '.join(fparams)})\n" + "\n".join(body) + f"\n{ind}end"
            closure = Expr(text, pure=True)
            closure.hint = fname
            self.put(out, ind, a, closure, inline=True)
            return q
        elif name == "CAPTURE":
            pass
        elif name == "NAMECALL":
            obj = self.get(b)
            method = self.kstr(aux) or "?"
            self.pending[a] = Expr(f"{obj.wrap(100)}:{method}", 100)
            self.pending[a + 1] = Expr("<self>")
            self.pending[a].method = self.pending[a + 1].method = True
        elif name in ("CALL", "CALLFB"):
            fn = self.get(a)
            nargs = b - 1
            args = []
            self_call = a + 1 in self.pending and self.pending[a + 1].text == "<self>"
            if self_call:
                self.pending.pop(a + 1)
            first = a + 2 if self_call else a + 1
            if self_call and nargs > 0:
                nargs -= 1  # the object itself is the first argument
            if nargs < 0:
                # up to the top of the stack: the last argument is multi-valued
                regs = sorted(r for r in self.pending if r >= first)
                last = regs[-1] if regs else first - 1
                nargs = last - first + 1
            for r in range(first, first + nargs):
                args.append(self.get(r))
            args = list_texts(args, b != 0)
            call = Expr(f"{fn.wrap(100)}({', '.join(args)})", 100, multi=True)
            call.one = c == 2  # cut to one value: `(f())` where a list would expand it
            self.flush(out, ind, keep=[r for r, e in self.pending.items() if e.pure])
            if c == 1:
                out.append(f"{ind}{call.text}")
            elif c == 2:
                self.put(out, ind, a, call)
            elif c == 0:
                call.multret = True  # all its values go on: never one assignment
                self.pending[a] = call
            elif c == 4 and not getattr(self, "flat", False) and nxt in self.by_pc and                     op_name(self.by_pc[nxt][1]).startswith("FORGPREP") and                     (self.by_pc[nxt][2] >> 8) & 0xFF == a:
                self.iter_call = (a, call.text)  # the loop header says it
            else:
                names = []
                for i in range(c - 1):
                    names.append(self.name_for(a + i))
                fresh = any(a + i not in self.declared for i in range(c - 1))
                for i in range(c - 1):
                    self.declared.add(a + i)
                out.append(f"{ind}{'local ' if fresh else ''}{', '.join(names)} = {call.text}")
        elif name == "RETURN":
            vals = []
            if b == 0:
                regs = sorted(r for r in self.pending if r >= a)
                count = (regs[-1] - a + 1) if regs else 0
            else:
                count = b - 1
            for r in range(a, a + count):
                vals.append(self.get(r))
            vals = list_texts(vals, b != 0)
            self.flush(out, ind)
            if not (pc == self.order[-1] and not vals):
                out.append(f"{ind}return {', '.join(vals)}".rstrip())
        elif name in BINOPS:
            sym, prec = BINOPS[name]
            l, r_ = self.get(b), self.get(c)
            self.put(out, ind, a, Expr(f"{l.wrap(prec)} {sym} {r_.wrap(prec + 1)}", prec))
        elif name in ("ADDK", "SUBK", "MULK", "DIVK", "MODK", "POWK", "IDIVK"):
            sym, prec = BINOPS[name[:-1]]
            l = self.get(b)
            self.put(out, ind, a, Expr(f"{l.wrap(prec)} {sym} {self.kexpr(c)}", prec))
        elif name in ("SUBRK", "DIVRK"):
            sym, prec = BINOPS[name[:3]]
            r_ = self.get(c)
            self.put(out, ind, a, Expr(f"{self.kexpr(b)} {sym} {r_.wrap(prec + 1)}", prec))
        elif name in ("AND", "OR"):
            l, r_ = self.get(b), self.get(c)
            sym, prec = ("and", 2) if name == "AND" else ("or", 1)
            self.put(out, ind, a, Expr(f"{l.wrap(prec)} {sym} {r_.wrap(prec + 1)}", prec))
        elif name in ("ANDK", "ORK"):
            l = self.get(b)
            sym, prec = ("and", 2) if name == "ANDK" else ("or", 1)
            self.put(out, ind, a, Expr(f"{l.wrap(prec)} {sym} {self.kexpr(c)}", prec))
        elif name == "CONCAT":
            parts = [self.get(r).wrap(6) for r in range(b, c + 1)]
            self.put(out, ind, a, Expr(" .. ".join(parts), 5))
        elif name == "NOT":
            self.put(out, ind, a, Expr("not " + self.get(b).wrap(8), 8))
        elif name == "MINUS":
            v = self.get(b).wrap(8)
            self.put(out, ind, a, Expr("-" + (f"({v})" if v.startswith("-") else v), 8))
        elif name == "LENGTH":
            self.put(out, ind, a, Expr("#" + self.get(b).wrap(8), 8))
        elif name in ("NEWTABLE", "DUPTABLE"):
            table = Expr("{}")
            table.fields = self.template_fields(d) if name == "DUPTABLE" else []
            fills = self.table_fills(pc, a)
            if name == "NEWTABLE" and fills[0] and not fills[1]:
                # `local t = {}` then `t.k = v`: a constructor with keys
                # would have compiled to DUPTABLE, so these are statements
                self.flush_impure(out, ind, a)
                self.assign(out, ind, a, Expr("{}"))
            elif self.read_once(pc, a) or self.table_filled_then_read(pc, a):
                self.pending.pop(a, None)
                self.pending[a] = table
            elif fills[2] is not None and (name == "DUPTABLE" or fills[1]):
                # a table used more than once: built as a constructor, then
                # given its variable once the last field is in
                table.build_end = fills[2]
                self.pending.pop(a, None)
                self.pending[a] = table
            else:
                # a table used more than once: its template's constant fields
                # still belong in the constructor
                self.flush_impure(out, ind, a)
                self.assign(out, ind, a, Expr(table.render()))
        elif name == "SETLIST" and a in self.pending and self.pending[a].fields is not None                 and not (b <= a < b + max(c - 1, 0)):
            table = self.pending.pop(a)
            count = c - 1
            if count < 0:
                regs = sorted(r for r in self.pending if r >= b)
                count = (regs[-1] - b + 1) if regs else 0
            vals = list_texts([self.get(r) for r in range(b, b + count)], c != 0)
            table.fields.extend(vals)
            self.pending[a] = table
            self.finish_table(out, ind, a, table, pc)
        elif name == "SETLIST":
            count = c - 1
            if count < 0:
                regs = sorted(r for r in self.pending if r >= b)
                count = (regs[-1] - b + 1) if regs else 0
            vals = list_texts([self.get(r) for r in range(b, b + count)], c != 0)
            self.flush(out, ind)
            tbl = self.reg(a)
            for i, v in enumerate(vals):
                out.append(f"{ind}{tbl}[{aux + i}] = {v}")
        elif name == "GETVARARGS":
            va = Expr("...", 100, multi=True)
            va.one = b == 2  # `local x = ...` keeps the first value only
            self.put(out, ind, a, va)
        else:
            self.flush(out, ind)
            out.append(f"{ind}-- {name} A={a} B={b} C={c} D={d}")
        return nxt

    def reg_text(self, r):
        e = self.pending.pop(r, None)
        return e.text if e else self.reg(r)

    def last_insn_before(self, target):
        prev = None
        for pc in self.order:
            if pc >= target:
                break
            prev = pc
        return prev


LOOP_OPEN = re.compile(r"^ *(for .* do|while .* do|repeat)$")
SIMPLE_CONST = re.compile(r'^(nil|true|false|-?[0-9][0-9.e+-]*|"[^"]*"|\{\})$')


def resolve_gotos(lines):
    """Gotos that remain are mostly the early returns of local functions
    the compiler inlined: a jump forward to the end of the inlined body.
      * no loop in between: `repeat <body> until true`, the gotos `break`
      * out of a search loop whose fall-through just sets defaults:
        the defaults move before the loop and the goto becomes `break`
    Returns the new lines, or None while any goto is left."""
    lines = list(lines)
    for _ in range(64):
        label = None
        for i, l in enumerate(lines):
            m = re.match(r"^( *)::L(\d+)::$", l)
            if m:
                label = (i, len(m.group(1)), m.group(2))
                break
        if label is None:
            break
        li, I, n = label
        gotos = [j for j, l in enumerate(lines) if re.search(rf"\bgoto L{n}\b", l)]
        if not gotos:
            return None
        # a short ending that leaves the function: copy it to every jump
        tail = []
        k = li + 1
        while k < len(lines) and (not lines[k].strip() or indent_of(lines[k]) >= I):
            if lines[k].strip():
                tail.append(lines[k][I:])
            k += 1
        at_end = k >= len(lines) or (indent_of(lines[k]) < I and I == indent_of(lines[0]) and
                                     lines[k].strip().startswith("end"))
        simple = all(indent_of(l) == 0 and not re.match(r"^(if|for|while|repeat|do|local function)\b", l)
                     and "function" not in l and "::" not in l and "goto" not in l for l in tail)
        if tail and simple and len(tail) <= 6 and (tail[-1].startswith("return") or
                                                    (at_end and k >= len(lines))):
            body = tail if tail[-1].startswith("return") else tail + ["return"]
            new = []
            for j, l in enumerate(lines):
                if j == li:
                    continue
                m = re.match(rf"^( *)if (.*) then goto L{n} end$", l)
                if m:
                    new.append(f"{m.group(1)}if {m.group(2)} then")
                    new.extend(m.group(1) + "    " + x for x in body)
                    new.append(f"{m.group(1)}end")
                    continue
                m = re.match(rf"^( *)goto L{n}$", l)
                if m:
                    new.extend(m.group(1) + x for x in body)
                    continue
                new.append(l)
            if not any(re.search(rf"\bgoto L{n}\b", l) for l in new):
                lines = new
                continue
        if any(g > li for g in gotos):
            return None
        pad = " " * I
        # the statement (at the label's indent) holding the first goto
        g0 = gotos[0]
        start = None
        for k in range(g0, -1, -1):
            s = lines[k]
            if not s.strip():
                continue
            ki = indent_of(s)
            if ki < I:
                start = k + 1
                break
            if ki == I and not s.strip().startswith(("else", "elseif", "end", "until", "::")):
                start = k
                break
        if start is None:
            start = 0
        if indent_of(lines[g0]) < I:
            return None
        if any(lines[k].strip() and (indent_of(lines[k]) < I or (
                indent_of(lines[k]) == I and lines[k].strip().startswith(("else", "elseif"))))
               for k in range(start, li)):
            return None  # the label is not in the block the jumps start from
        loops = {g: [k for k in enumerate_openers(lines, g, I) if LOOP_OPEN.match(lines[k])]
                 for g in gotos}
        if all(not v for v in loops.values()):
            # repeat ... until true around [start, li); a break/continue in
            # there that belongs to an outer loop would be caught by it
            for k in range(start, li):
                if re.search(r"(^|\bthen |\bdo )(break|continue)\b", lines[k].strip()) and not any(
                        LOOP_OPEN.match(lines[o]) for o in enumerate_openers(lines, k, I + 4)):
                    return None
            body = lines[start:li]
            after = lines[li + 1:]
            later = set()
            for l in after:
                later |= words(l)
            hoist = []
            new_body = []
            for l in body:
                s = l.strip()
                m = re.match(r"^local ([A-Za-z_][A-Za-z0-9_]*(?:, [A-Za-z_][A-Za-z0-9_]*)*)( = .*)?$", s) \
                    if indent_of(l) == I else None
                if m and set(m.group(1).split(", ")) & later:
                    hoist.extend(m.group(1).split(", "))
                    l = pad + m.group(1) + (m.group(2) or " = nil")
                    if not m.group(2):
                        continue
                l = re.sub(rf"\bgoto L{n}\b", "break", l)
                new_body.append(("    " + l) if l.strip() else l)
            repl = ([pad + "local " + ", ".join(dict.fromkeys(hoist))] if hoist else []) + \
                [pad + "repeat"] + new_body + [pad + "until true"]
            lines[start:li + 1] = repl
            continue
        # a search loop: every goto sits directly in the same loop, which the
        # label follows after a few constant assignments
        loop_lines = {v[0] for v in loops.values() if v}
        if len(loop_lines) != 1 or any(len(v) != 1 for v in loops.values()):
            return None
        lo = loop_lines.pop()
        if indent_of(lines[lo]) != I:
            return None
        lend = None
        for k in range(lo + 1, li):
            if lines[k].strip() and indent_of(lines[k]) <= I:
                lend = k
                break
        if lend is None or lines[lend].strip() != "end":
            return None
        tail = lines[lend + 1:li]
        targets = []
        for l in tail:
            m = re.match(rf"^{pad}([A-Za-z_][A-Za-z0-9_]*) = (.+)$", l)
            if not m or not SIMPLE_CONST.match(m.group(2)):
                return None
            targets.append(m.group(1))
        loop_words = set()
        for l in lines[lo:lend + 1]:
            s = l.strip()
            m = re.match(r"^([A-Za-z_][A-Za-z0-9_]*) = (.*)$", s)
            loop_words |= words(m.group(2)) if m else words(l)
        if set(targets) & loop_words:
            return None  # the loop reads a default it would now see early
        body = [re.sub(rf"\bgoto L{n}\b", "break", l) for l in lines[lo:lend + 1]]
        lines[lo:li + 1] = tail + body
    if any(re.search(r"\bgoto L\d+\b", l) or re.match(r"^ *::L\d+::$", l) for l in lines):
        return None
    return lines


def enumerate_openers(lines, g, I):
    """Block openers between line g and the label's block (indent I)."""
    found = []
    cur = indent_of(lines[g])
    for k in range(g - 1, -1, -1):
        s = lines[k]
        if not s.strip():
            continue
        ki = indent_of(s)
        if ki < cur:
            if ki < I:
                break
            if not s.strip().startswith(("else", "elseif")):
                found.append(k)
            cur = ki
            if ki == I:
                break
    return found


def bracket_depth(line):
    d = 0
    quote = None
    k = 0
    while k < len(line):
        ch = line[k]
        if quote:
            if ch == "\\":
                k += 2
                continue
            if ch == quote:
                quote = None
        elif ch in "\"'`":
            quote = ch
        elif ch in "([{":
            d += 1
        elif ch in ")]}":
            d -= 1
        k += 1
    return d


def statement_end(lines, i):
    """Last line of the statement starting on line i (a closure or table
    written over several lines continues below it)."""
    ind = indent_of(lines[i])
    depth = bracket_depth(lines[i])
    j = i
    while j + 1 < len(lines):
        if depth > 0:
            j += 1
            depth += bracket_depth(lines[j])
            continue
        if re.search(r"function\([^()]*\)$", lines[j].rstrip()):
            # `return function() ... end` with no brackets around it
            k = j + 1
            while k < len(lines) and not (indent_of(lines[k]) <= ind and lines[k].strip().startswith("end")):
                k += 1
            if k >= len(lines):
                break
            j = k
            depth += bracket_depth(lines[j][indent_of(lines[j]) + 3:])
            continue
        break
    return j


def wrap_mid_block_exits(lines):
    """Luau requires return/break/continue to end their block. One followed by
    more statements of the same block (reached through a jump) becomes
    `do return end` and friends. Statements starting with `(` get a `;`."""
    out = []
    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.lstrip(" ")
        ind = line[:len(line) - len(stripped)]
        if stripped.startswith("(") and out:
            # only after an expression could `(` read as a call of it; a block
            # opener before it needs nothing (and `;` there is not allowed)
            prev = out[-1].rstrip()
            opener = (re.search(r"(^|\s)(then|do|else|repeat)$", prev)
                      or re.search(r"function\([^)]*\)$", prev))
            if not opener and not prev.endswith(";"):
                out[-1] = prev + ";"
        word = stripped.split(" ", 1)[0]
        if word in ("return", "break", "continue"):
            last = statement_end(lines, i)
            nxt = lines[last + 1] if last + 1 < len(lines) else ""
            n_stripped = nxt.lstrip(" ")
            n_ind = nxt[:len(nxt) - len(n_stripped)]
            ends_block = (len(n_ind) < len(ind) or
                          n_stripped.split(" ", 1)[0] in ("end", "else", "elseif", "until", "end)")
                          or n_stripped.startswith(("end", "::", "until")))
            if not ends_block and nxt:
                if last == i:
                    out.append(f"{ind}do {stripped} end")
                else:
                    out.append(f"{ind}do")
                    out.extend("    " + l for l in lines[i:last + 1])
                    out.append(f"{ind}end")
                i = last + 1
                continue
        out.append(line)
        i += 1
    return out


def merge_elseif(body, ind):
    """An else block that is one if statement becomes elseif branches."""
    inner = ind + "    "
    if (len(body) >= 2 and body[0].startswith(inner + "if ") and body[0].endswith(" then")
            and body[-1] == inner + "end"):
        # every other line at the inner level must belong to that same if
        top = [l for l in body[1:-1] if l.startswith(inner) and not l.startswith(inner + " ")]
        if all(l in (inner + "else",) or (l.startswith(inner + "elseif ") and l.endswith(" then"))
               for l in top):
            out = [ind + "else" + body[0][len(inner):]]
            for l in body[1:-1]:
                out.append(l[4:] if l.startswith(inner) else l)
            return out
    return [ind + "else"] + body


def balanced(s):
    depth = 0
    for ch in s:
        depth += ch == "("
        depth -= ch == ")"
        if depth < 0:
            return False
    return depth == 0


def start_ok(target, nxt, end):
    return nxt < target <= end


def indent_of(line):
    return len(line) - len(line.lstrip(" "))


LVALUE = re.compile(r'^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)*$')


def if_expressions(lines, limit=150):
    """`if c then x = a elseif d then x = b else x = c end`, one assignment
    per branch, becomes `x = if c then a elseif d then b else c`."""
    out = []
    i = 0
    while i < len(lines):
        line = lines[i]
        ind = indent_of(line)
        s = line.strip()
        got = None
        if s.startswith("if ") and s.endswith(" then"):
            pad = " " * ind
            conds = [s[3:-5]]
            vals = []
            name = None
            j = i + 1
            ok = True
            while ok:
                if j + 1 >= len(lines):
                    ok = False
                    break
                body, nxt = lines[j], lines[j + 1]
                if indent_of(body) != ind + 4 or " = " not in body:
                    ok = False
                    break
                lhs, val = body.strip().split(" = ", 1)
                if lhs.startswith("local ") or not LVALUE.match(lhs) or (name and lhs != name):
                    ok = False
                    break
                name = lhs
                vals.append(val)
                ns = nxt.strip()
                if indent_of(nxt) != ind:
                    ok = False
                elif ns.startswith("elseif ") and ns.endswith(" then"):
                    conds.append(ns[7:-5])
                    j += 2
                elif ns == "else":
                    j += 2
                    if j + 1 < len(lines) and indent_of(lines[j]) == ind + 4 and \
                            lines[j + 1] == pad + "end":
                        lhs, _, val = lines[j].strip().partition(" = ")
                        if lhs == name:
                            vals.append(val)
                            got = (name, conds, vals, j + 2)
                    ok = False
                else:
                    ok = False
            if got:
                name, conds, vals, after = got
                vals = [f"({v})" if v.startswith("if ") or v.startswith("function") else v for v in vals]
                expr = f"if {conds[0]} then {vals[0]}"
                for c, v in zip(conds[1:], vals[1:-1]):
                    expr += f" elseif {c} then {v}"
                expr += f" else {vals[-1]}"
                text = f"{pad}{name} = {expr}"
                if len(text) <= limit and all("\n" not in v for v in vals):
                    out.append(text)
                    i = after
                    continue
        out.append(line)
        i += 1
    return out


def function_bodies(lines):
    """(name, params, first body line, end line) of each `local NAME = function`."""
    rx = re.compile(r'^( *)local ([A-Za-z_][A-Za-z0-9_]*) = function\(([^)]*)\)$')
    found = []
    for i, line in enumerate(lines):
        m = rx.match(line)
        if not m:
            continue
        pad = m.group(1)
        for j in range(i + 1, len(lines)):
            if lines[j] == pad + "end":
                params = [x.strip() for x in m.group(3).split(",") if x.strip()]
                found.append((m.group(2), params, i + 1, j))
                break
    return found


def top_level_comma(s):
    depth = 0
    quote = None
    for ch in s:
        if quote:
            quote = None if ch == quote else quote
        elif ch in "\"'":
            quote = ch
        elif ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        elif ch == "," and depth == 0:
            return True
    return False


def return_template(body):
    """The expression a helper returns, when its body is only returns:
    `return e`, or `if c then return a end ... return b` (an if-expression)."""
    pad = indent_of(body[0]) if body else 0
    lines = [l[pad:] for l in body]
    if len(lines) == 1 and lines[0].startswith("return ") and not top_level_comma(lines[0][7:]):
        return lines[0][7:]
    conds, vals = [], []
    i = 0
    while i + 2 < len(lines) and lines[i].startswith("if ") and lines[i].endswith(" then") \
            and lines[i + 1].startswith("    return ") and lines[i + 2] == "end":
        conds.append(lines[i][3:-5])
        vals.append(lines[i + 1][11:])
        i += 3
    if conds and i == len(lines) - 1 and lines[i].startswith("return "):
        vals.append(lines[i][7:])
        expr = f"if {conds[0]} then {vals[0]}"
        for c, v in zip(conds[1:], vals[1:-1]):
            expr += f" elseif {c} then {v}"
        return expr + f" else {vals[-1]}"
    return None


ATOM = re.compile(r'^-?[A-Za-z_0-9.]+(\([^()]*\)|\[[^\[\]]*\]|\.[A-Za-z_][A-Za-z0-9_]*|:[A-Za-z_][A-Za-z0-9_]*\([^()]*\))*$')


def balanced_all(s):
    depth = 0
    quote = None
    for ch in s:
        if quote:
            if ch == quote:
                quote = None
            continue
        if ch in "\"'":
            quote = ch
        elif ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
            if depth < 0:
                return False
    return depth == 0 and quote is None


def compile_template(expr, params):
    """Split a template into literal text and parameter holes."""
    parts = re.split(r'\b(' + "|".join(re.escape(p) for p in params) + r')\b', expr)
    toks = []
    for k, part in enumerate(parts):
        if k % 2:
            toks.append(params.index(part))
        elif part:
            toks.append(part)
    return toks


def hole_ok(text, before, after):
    """Can an argument printed here stand alone? Inside parentheses or
    between commas anything goes; elsewhere only a simple operand does."""
    if not text or not balanced_all(text) or text != text.strip():
        return False
    arg_slot = before.endswith(("(", ", ")) and after.startswith((",", ")"))
    return arg_slot or ATOM.match(text) is not None


def match_at(line, i, toks, nparams):
    caps = [None] * nparams
    pos = i
    for k, tok in enumerate(toks):
        if isinstance(tok, str):
            if not line.startswith(tok, pos):
                return None
            pos += len(tok)
            continue
        nxt = toks[k + 1] if k + 1 < len(toks) else None
        before = toks[k - 1] if k > 0 and isinstance(toks[k - 1], str) else ""
        found = None
        if nxt is None:
            # the hole ends the template: take the longest simple operand
            for e in range(len(line), pos, -1):
                cand = line[pos:e]
                if hole_ok(cand, before, ""):
                    found = e
                    break
        else:
            start = pos
            while True:
                e = line.find(nxt, start)
                if e < 0:
                    break
                if hole_ok(line[pos:e], before, nxt):
                    found = e
                    break
                start = e + 1
        if found is None:
            return None
        cap = line[pos:found]
        if caps[tok] is not None and caps[tok] != cap:
            return None
        caps[tok] = cap
        pos = found
    if any(c is None for c in caps):
        return None
    return pos, caps


def uninline(lines):
    """Luau inlines small local functions at -O2. Where the body of such a
    helper appears verbatim with its parameters filled in, put the call back."""
    templates = []
    for name, params, b0, b1 in function_bodies(lines):
        expr = return_template(lines[b0:b1])
        if expr is None or not params:
            continue
        toks = compile_template(expr, params)
        literal = sum(len(x) for x in toks if isinstance(x, str))
        used = {x for x in toks if isinstance(x, int)}
        if not toks or not isinstance(toks[0], str) or literal < 10 or len(used) != len(params):
            continue
        templates.append((name, toks, len(params), b1))
    if not templates:
        return lines
    out = list(lines)
    for name, toks, n, def_end in templates:
        head = toks[0]
        for li in range(def_end + 1, len(out)):
            line = out[li]
            changed = True
            while changed:
                changed = False
                i = line.find(head)
                while i >= 0:
                    if i > 0 and (line[i - 1].isalnum() or line[i - 1] in "_."):
                        i = line.find(head, i + 1)
                        continue
                    m = match_at(line, i, toks, n)
                    if m and re.match(r"^\s*([-+*/%^]|//|\.\.)?=(?!=)", line[m[0]:]):
                        m = None  # `f(x) = v` is not Luau: the helper body is a target here
                    if m:
                        end, caps = m
                        if end < len(line) and (line[end].isalnum() or line[end] == "_") and \
                                (toks[-1] if isinstance(toks[-1], str) else "x")[-1:].isalnum():
                            i = line.find(head, i + 1)
                            continue
                        line = line[:i] + f"{name}({', '.join(caps)})" + line[end:]
                        changed = True
                        break
                    i = line.find(head, i + 1)
            out[li] = line
    return out


# ------------------------------------------------- source-level passes

TOKEN = r"[A-Za-z_][A-Za-z0-9_]*"
PASS_KEYWORDS = {"and", "break", "do", "else", "elseif", "end", "false", "for",
            "function", "if", "in", "local", "nil", "not", "or", "repeat",
            "return", "then", "true", "until", "while", "continue", "self",
            "type", "typeof", "export"}
GENERATED = re.compile(r"^(v\d+(x\d+)?(_\d+)*|arg\d+(_\d+)?)$")




def scan_depth(s):
    """Yield (index, char, depth) outside string literals."""
    depth = 0
    quote = None
    i = 0
    while i < len(s):
        ch = s[i]
        if quote:
            if ch == "\\":
                i += 2
                continue
            if ch == quote:
                quote = None
        elif ch in "\"'`":
            quote = ch
        else:
            if ch in "([{":
                depth += 1
            elif ch in ")]}":
                depth -= 1
            yield i, ch, depth
        i += 1


def split_top(s, sep=","):
    parts, start = [], 0
    for i, ch, depth in scan_depth(s):
        if ch == sep and depth == 0:
            parts.append(s[start:i].strip())
            start = i + 1
    parts.append(s[start:].strip())
    return parts


def matching_paren(s, open_at):
    for i, ch, depth in scan_depth(s[open_at:]):
        if ch == ")" and depth == 0:
            return open_at + i
    return -1


def words(line):
    """Identifier tokens of a line, outside strings, not after a `.`/`:`."""
    out = set()
    s = line
    quote = None
    i = 0
    while i < len(s):
        ch = s[i]
        if quote:
            if ch == "\\":
                i += 2
                continue
            if ch == quote:
                quote = None
            i += 1
            continue
        if ch in "\"'":  # `{...}` in backtick strings is code
            quote = ch
            i += 1
            continue
        if ch.isalpha() or ch == "_":
            j = i
            while j < len(s) and (s[j].isalnum() or s[j] == "_"):
                j += 1
            prev = s[i - 1] if i else ""
            if not prev or prev not in ".:":
                out.add(s[i:j])
            i = j
            continue
        i += 1
    return out


def replace_token(line, old, new):
    """Rename identifier `old` (a variable, not a field) outside strings."""
    out = []
    s = line
    quote = None
    i = 0
    while i < len(s):
        ch = s[i]
        if quote:
            out.append(ch)
            if ch == "\\" and i + 1 < len(s):
                out.append(s[i + 1])
                i += 2
                continue
            if ch == quote:
                quote = None
            i += 1
            continue
        if ch in "\"'":  # `{...}` in backtick strings is code
            quote = ch
            out.append(ch)
            i += 1
            continue
        if ch.isalpha() or ch == "_":
            j = i
            while j < len(s) and (s[j].isalnum() or s[j] == "_"):
                j += 1
            word = s[i:j]
            prev = s[i - 1] if i else ""
            out.append(new if word == old and (not prev or prev not in ".:") else word)
            i = j
            continue
        out.append(ch)
        i += 1
    return "".join(out)


FUNC_OPEN = re.compile(r"function(?: [A-Za-z_][A-Za-z0-9_.:]*)?\(([^()]*)\)$")


def block_end(lines, i):
    """Index of the line closing the block opened on line i (same indent)."""
    pad = indent_of(lines[i])
    for j in range(i + 1, len(lines)):
        if lines[j].strip() and indent_of(lines[j]) <= pad:
            return j
    return len(lines) - 1


def functions(lines):
    """(header index, end index, params) of every function in the text,
    including the whole file as the outermost one."""
    found = [(-1, len(lines), [])]
    for i, line in enumerate(lines):
        m = FUNC_OPEN.search(line.rstrip())
        if m:
            params = [x.strip() for x in m.group(1).split(",") if x.strip()]
            found.append((i, block_end(lines, i), params))
    return found


# ------------------------------------------------------------------ passes

def generic_for(lines):
    """`local a = t; local b = nil; local c = nil; for k, v in a, b, c do`
    is how a plain `for k, v in t do` compiles: put it back."""
    out = list(lines)
    i = 0
    rx = re.compile(r"^( *)for (.+) in (" + TOKEN + r"), (" + TOKEN + r"), (" + TOKEN + r") do$")
    while i < len(out):
        m = rx.match(out[i])
        if m and i >= 3:
            pad, vars_, a, b, c = m.groups()
            defs = out[i - 3:i]
            want = [rf"^{pad}(local )?{re.escape(a)} = (.+)$", rf"^{pad}(local )?{re.escape(b)} = nil$",
                    rf"^{pad}(local )?{re.escape(c)} = nil$"]
            ms = [re.match(w, d) for w, d in zip(want, defs)]
            if all(ms):
                expr = ms[0].group(2)
                locals_ = [n for n, mm in zip((a, b, c), ms) if mm.group(1)]
                # the state registers mean nothing outside the loop, but a
                # name declared here may be assigned again later in its block
                fend = len(out)
                for k in range(i + 1, len(out)):
                    if out[k].strip() and indent_of(out[k]) < len(pad):
                        fend = k
                        break
                later = set()
                for line in out[i + 1:fend]:
                    later |= words(line)
                keep = [n for n in locals_ if n in later]
                repl = []
                if keep:
                    repl.append(f"{pad}local {', '.join(keep)}")
                repl.append(f"{pad}for {vars_} in {expr} do")
                out[i - 3:i + 1] = repl
                i = i - 3 + len(repl)
                continue
        i += 1
    return out


COMPOUND = {"+": 6, "-": 6, "*": 7, "/": 7, "//": 7, "%": 7, "..": 5}
TOP_OPS = [" or ", " and ", " < ", " > ", " <= ", " >= ", " == ", " ~= ", " .. ",
           " + ", " - ", " * ", " / ", " // ", " % ", " ^ "]
PREC = {" or ": 1, " and ": 2, " < ": 3, " > ": 3, " <= ": 3, " >= ": 3, " == ": 3,
        " ~= ": 3, " .. ": 5, " + ": 6, " - ": 6, " * ": 7, " / ": 7, " // ": 7,
        " % ": 7, " ^ ": 9}


def lowest_top_op(s):
    """Lowest precedence of a binary operator at depth 0 (None if atom)."""
    low = None
    starts = {i for i, ch, d in scan_depth(s) if d == 0}
    for op in TOP_OPS:
        k = s.find(op)
        while k >= 0:
            if k in starts:
                low = PREC[op] if low is None else min(low, PREC[op])
                break
            k = s.find(op, k + 1)
    if s.startswith("if "):
        return 0
    return low


def compound_assign(lines):
    """`x = x + e` -> `x += e` (Luau), when e is the whole right operand."""
    rx = re.compile(r"^( *)([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*) = (.+)$")
    out = []
    for line in lines:
        m = rx.match(line)
        if m and not line.lstrip().startswith("local "):
            pad, lhs, rhs = m.groups()
            for op, prec in COMPOUND.items():
                head = f"{lhs} {op} "
                if rhs.startswith(head):
                    rest = rhs[len(head):]
                    low = lowest_top_op(rest)
                    first = lowest_top_op(rhs)
                    if first == prec and (low is None or low > prec or (op == ".." and low >= prec)):
                        if rest.startswith("(") and matching_paren(rest, 0) == len(rest) - 1:
                            rest = rest[1:-1]  # `x += (a + b)`: the operand is all of it
                        line = f"{pad}{lhs} {op}= {rest}"
                    break
        out.append(line)
    return out


INTERP = re.compile(r'\("((?:[^"\\]|\\.)*)"\):format\(')


def interpolation(lines):
    """`("a%*b"):format(x)` is how Luau compiles `` `a{x}b` ``."""
    out = []
    for line in lines:
        pos = 0
        while True:
            m = INTERP.search(line, pos)
            if not m:
                break
            fmt = m.group(1)
            close = matching_paren(line, m.end() - 1)
            if close < 0:
                break
            args = split_top(line[m.end():close])
            holes = fmt.count("%*")
            rest = fmt.replace("%*", "").replace("%%", "")
            if holes == 0 or holes != len(args) or "%" in rest or "`" in fmt or \
                    any(not a for a in args) or any("\n" in a for a in args):
                pos = m.end()
                continue
            pieces = fmt.split("%*")
            text = "`"
            for k, piece in enumerate(pieces):
                text += piece.replace("%%", "%").replace("{", "\\{").replace('\\"', '"')
                if k < len(args):
                    text += "{" + args[k] + "}"
            text += "`"
            line = line[:m.start()] + text + line[close + 1:]
            pos = m.start() + len(text)
        out.append(line)
    return out


def local_functions(lines):
    """`local f = function(...)` -> `local function f(...)`, and
    `T.k = function(self, ...)` -> `function T:k(...)` when the first
    parameter is used as an object."""
    out = list(lines)
    rx_local = re.compile(r"^( *)local (" + TOKEN + r") = function\((.*)\)$")
    rx_field = re.compile(r"^( *)(" + TOKEN + r"(?:\." + TOKEN + r")*)\.(" + TOKEN + r") = function\((.*)\)$")
    for i, line in enumerate(out):
        m = rx_local.match(line)
        if m:
            out[i] = f"{m.group(1)}local function {m.group(2)}({m.group(3)})"
            continue
        m = rx_field.match(line)
        if m:
            pad, obj, key, params = m.groups()
            plist = [x.strip() for x in params.split(",") if x.strip()]
            end = block_end(out, i)
            body = out[i + 1:end]
            first = plist[0] if plist else None
            used_as_obj = first and GENERATED.match(first) and any(
                re.search(rf"(?<![A-Za-z0-9_.]){re.escape(first)}[.:]", l) for l in body)
            taken = any("self" in words(l) for l in body)
            if used_as_obj and not taken:
                for j in range(i + 1, end):
                    out[j] = replace_token(out[j], first, "self")
                out[i] = f"{pad}function {obj}:{key}({', '.join(plist[1:])})"
            else:
                out[i] = f"{pad}function {obj}.{key}({params})"
    return out


def sink_declarations(lines):
    """Hoisted `local a, b` lines: a name whose every use sits in the block
    where it is first assigned is declared there instead."""
    out = list(lines)
    dropped = set()
    for fi, fend, _ in sorted(functions(out), key=lambda f: -f[0]):
        start = fi + 1
        if start >= len(out):
            continue
        decl = out[start]
        m = re.match(r"^( *)local (" + TOKEN + r"(?:, " + TOKEN + r")*)$", decl)
        if not m:
            continue
        pad = m.group(1)
        names = m.group(2).split(", ")
        keep = []
        for name in names:
            uses = [j for j in range(start + 1, min(fend, len(out))) if name in words(out[j])]
            if not uses:
                keep.append(name)  # not seen in range: keep it rather than guess
                continue
            first = uses[0]
            fline = out[first]
            fm = re.match(rf"^( *){re.escape(name)} = (.+)$", fline)
            if not fm or name in words(fm.group(2)):
                keep.append(name)
                continue
            ind = indent_of(fline)
            # the block holding the first assignment: from its opener up to
            # the line that drops below its indent
            stop = fend
            for j in range(first + 1, fend):
                if out[j].strip() and indent_of(out[j]) < ind:
                    stop = j
                    break
            # `else`/`elseif` at a lower indent ends the block too
            if all(first <= u < stop for u in uses) and not any(
                    re.match(rf"^ *(for|while) .*\b{re.escape(name)}\b", out[j]) for j in uses[1:]):
                # a loop around the first assignment could carry the value
                # to its next pass: only allowed when the assignment is
                # the first thing that touches the name inside that loop
                out[first] = f"{fm.group(1)}local {name} = {fm.group(2)}"
            else:
                keep.append(name)
        out[start] = f"{pad}local {', '.join(keep)}" if keep else ""
        if not keep:
            dropped.add(start)
    return [l for k, l in enumerate(out) if k not in dropped]


SINGULAR = [(re.compile(r"ies$"), "y"), (re.compile(r"(ss|us|is)$"), r"\1"),
            (re.compile(r"(xes|ches|shes|sses)$"), lambda m: m.group(1)[:-2]),
            (re.compile(r"s$"), "")]


def singular(name):
    for rx, rep in SINGULAR:
        if rx.search(name):
            new = rx.sub(rep, name)
            return new if new != name else None
    return None


SKIP_TYPES = {"Vector3", "Vector2", "CFrame", "Color3", "UDim2", "UDim", "Instance",
              "TweenInfo", "NumberSequence", "ColorSequence", "Ray", "Region3",
              "NumberRange", "Rect", "BrickColor", "vector"}


def lower_first(s):
    return s[:1].lower() + s[1:] if s and not s.isupper() else s


def derive_name(rhs):
    """A natural variable name for a value, from how it was computed."""
    rhs = rhs.strip()
    m = re.match(r"^(?:[A-Za-z_][A-Za-z0-9_]*)(?:\.[A-Za-z_][A-Za-z0-9_]*)*\.([A-Za-z_][A-Za-z0-9_]*)$", rhs)
    if m:
        return m.group(1)
    m = re.match(r'^.*:GetService\("([A-Za-z]+)"\)$', rhs)
    if m:
        return m.group(1)
    m = re.match(r'^require\(.*[.:](?:WaitForChild|FindFirstChild)\("([A-Za-z_][A-Za-z0-9_]*)"\)\)$', rhs) or \
        re.match(r'^require\(.*\.([A-Za-z_][A-Za-z0-9_]*)\)$', rhs)
    if m:
        return m.group(1)
    m = re.match(r'^.*[.:](?:WaitForChild|FindFirstChild|FindFirstChildOfClass|FindFirstChildWhichIsA|'
                 r'FindFirstAncestor|FindFirstAncestorOfClass|FindFirstAncestorWhichIsA)\("([A-Za-z_][A-Za-z0-9_]*)"(, [^()]*)?\)$', rhs)
    if m:
        return m.group(1)
    m = re.match(r'^.*:GetAttribute\("([A-Za-z_][A-Za-z0-9_]*)"\)$', rhs)
    if m:
        return lower_first(m.group(1))
    m = re.match(r'^Instance\.new\("([A-Za-z]+)"(, .*)?\)$', rhs)
    if m:
        return lower_first(m.group(1))
    m = re.match(r"^([A-Z][A-Za-z0-9_]*)\.new\(.*\)$", rhs)
    if m and m.group(1) not in SKIP_TYPES:
        return lower_first(m.group(1))
    m = re.match(r"^.*[.:]Get([A-Z][A-Za-z0-9_]*)\([^()]*\)$", rhs)
    if m and m.group(1) not in ("Service", "Attribute", "Output"):
        return lower_first(m.group(1))
    m = re.match(r"^" + TOKEN + r"\((?:[A-Za-z_][A-Za-z0-9_]*\.)+([A-Z][A-Z0-9_]+)\)$", rhs)
    if m:
        return m.group(1)  # F(Consts.NAME): a converted constant
    for rx, name in CALL_NAMES:
        m = rx.match(rhs)
        if m:
            return name(m) if callable(name) else name
    m = re.match(r'^.*:GetPropertyChangedSignal\("([A-Za-z0-9_]+)"\):(?:Connect|Once)\(', rhs)
    if m:
        return lower_first(m.group(1)) + "ChangedConnection"
    m = re.match(r"^.*[.:]([A-Z][A-Za-z0-9_]*):(?:Connect|Once|ConnectParallel)\(", rhs)
    if m:
        return lower_first(m.group(1)) + "Connection"
    return None


CALL_NAMES = [
    (re.compile(r'^.*:IsA\("([A-Za-z]+)"\)$'), lambda m: "is" + m.group(1)),
    (re.compile(r"^.*:Clone\(\)$"), "clone"),
    (re.compile(r"^table\.find\("), "index"),
    (re.compile(r"^(?:string\.split\(|.*:split\()"), "parts"),
    (re.compile(r"^table\.concat\("), "joined"),
    (re.compile(r"^table\.clone\("), "copy"),
    (re.compile(r"^#[A-Za-z_][A-Za-z0-9_.]*$"), "count"),
    (re.compile(r"^(?:os\.clock|tick|time|workspace:GetServerTimeNow|os\.time)\(\)$"), "now"),
    (re.compile(r"^.*:Raycast\("), "raycastResult"),
    (re.compile(r"^.*TweenService:Create\(|^.*:Create\(.*TweenInfo"), "tween"),
    (re.compile(r"^.*:LoadAnimation\("), "animationTrack"),
    (re.compile(r"^TweenInfo\.new\("), "tweenInfo"),
    (re.compile(r"^RaycastParams\.new\("), "raycastParams"),
    (re.compile(r"^OverlapParams\.new\("), "overlapParams"),
    (re.compile(r"^tostring\("), "str"),
    (re.compile(r"^tonumber\("), "num"),
    (re.compile(r"^(?:string\.format\(|`)"), "text"),
    (re.compile(r"^math\.random\("), "roll"),
    (re.compile(r"^.*:GetMouse\(\)$"), "mouse"),
    (re.compile(r"^.*:GetPivot\(\)$"), "pivot"),
    (re.compile(r"^.*:GetChildren\(\)$"), "children"),
    (re.compile(r"^.*:GetDescendants\(\)$"), "descendants"),
    (re.compile(r"^.*:GetPlayers\(\)$"), "players"),
    (re.compile(r"^.*:GetPlayerFromCharacter\("), "player"),
    (re.compile(r"^.*:InvokeServer\("), "response"),
    (re.compile(r"^.*:GetAsync\("), "data"),
    (re.compile(r"^coroutine\.create\("), "thread"),
    (re.compile(r"^task\.(?:spawn|delay|defer)\("), "thread"),
    (re.compile(r"^Instance\.new\(\"([A-Za-z]+)\""), lambda m: lower_first(m.group(1))),
    (re.compile(r"^.*:WaitForChild\(\"([A-Za-z_][A-Za-z0-9_]*)\""), lambda m: m.group(1)),
]


def rename_variables(lines):
    """Generated names (v5_1, arg2_1) get natural ones where the value says
    what it is: one definition, and the name is free in the scope."""
    out = list(lines)
    funcs = sorted(functions(out), key=lambda f: f[0])
    for fi, fend, params in funcs:
        lo, hi = fi + 1, min(fend, len(out))
        if lo >= hi:
            continue
        # the generated names that belong to this function: declared in it
        own = set()
        for j in range(lo, hi):
            m = re.match(r"^ *local (" + TOKEN + r"(?:, " + TOKEN + r")*)( = .*)?$", out[j])
            if m:
                own |= set(m.group(1).split(", "))
            m = re.match(r"^ *for (.+) in .+ do$", out[j]) or re.match(r"^ *for (" + TOKEN + r") = .+ do$", out[j])
            if m:
                own |= {x.strip() for x in m.group(1).split(",")}
        # generated names carry their function's depth: `v3` at file level,
        # `v3_1`/`arg1_1` one function deep, and so on
        depth = 0 if fi < 0 else indent_of(out[fi]) // 4 + 1
        own = {n for n in own if GENERATED.match(n) and
               (re.fullmatch(r"v\d+(x\d+)?", n) if depth == 0 else
                re.fullmatch(rf"(v\d+(x\d+)?|arg\d+)_{depth}", n))}
        if not own:
            continue
        # names in use anywhere in this function (and so unavailable)
        used = set()
        for j in range(max(fi, 0), hi):
            used |= words(out[j])
        for name in sorted(own):
            defs = []
            for j in range(lo, hi):
                s = out[j].strip()
                if re.match(rf"^(local )?{re.escape(name)} = ", s):
                    defs.append(j)
                elif re.match(rf"^(local )?([A-Za-z0-9_]+, )*{re.escape(name)}(, [A-Za-z0-9_]+)* = ", s) or \
                        re.match(rf"^{re.escape(name)} [-+*/%.]+= ", s):
                    defs.append(None)
            new = None
            fors = [j for j in range(lo, hi)
                    if re.match(rf"^ *for (.*\b{re.escape(name)}\b.*) in (.+) do$", out[j])]
            if fors and not defs:
                m = re.match(r"^ *for (.+) in (.+) do$", out[fors[0]])
                vars_ = [x.strip() for x in m.group(1).split(",")]
                src = m.group(2)
                end = block_end(out, fors[0])
                body_used = set()
                for j in range(fors[0] + 1, end):
                    body_used |= words(out[j])
                idx = vars_.index(name) if name in vars_ else -1
                tail = re.search(r"([A-Za-z_][A-Za-z0-9_]*)(?:\(\))?\)?$", src.replace("ipairs(", "").replace("pairs(", ""))
                base = tail.group(1) if tail else None
                if base and base.startswith("Get") and len(base) > 3:
                    base = lower_first(base[3:])
                if idx == 0 and len(vars_) > 1:
                    new = "_" if name not in body_used else ("i" if "ipairs(" in src else "key")
                elif idx == len(vars_) - 1 and base:
                    new = singular(base)
                    if new:
                        new = lower_first(new)
            elif len(defs) == 1 and defs[0] is not None:
                rhs = out[defs[0]].strip().split(" = ", 1)[1]
                new = derive_name(rhs)
            if not new or new in PASS_KEYWORDS or not re.match("^" + TOKEN + "$", new):
                continue
            # the variable's scope: its loop, or the block its `local` is in;
            # the new name only has to be free there
            s_lo, s_hi = lo, hi
            if fors and not defs:
                s_lo, s_hi = fors[0], block_end(out, fors[0]) + 1
            elif defs and defs[0] is not None and out[defs[0]].lstrip().startswith("local "):
                d = defs[0]
                s_lo, s_hi = d, hi
                for k in range(d + 1, hi):
                    if out[k].strip() and indent_of(out[k]) < indent_of(out[d]):
                        s_hi = k
                        break
            if any(name in words(out[j]) for j in list(range(lo, s_lo)) + list(range(s_hi, hi))):
                s_lo, s_hi = lo, hi  # used outside that block after all
            if new != "_":
                taken = set()
                for j in range(max(s_lo, 0), s_hi):
                    taken |= words(out[j])
                if new in taken or (s_lo, s_hi) == (lo, hi) and new in used:
                    continue
            for j in range(s_lo, s_hi):
                out[j] = replace_token(out[j], name, new)
            used.add(new)
    return out


SIGNAL_PARAMS = {
    "PlayerAdded": ["player"], "PlayerRemoving": ["player"],
    "CharacterAdded": ["character"], "CharacterRemoving": ["character"],
    "CharacterAppearanceLoaded": ["character"],
    "ChildAdded": ["child"], "ChildRemoved": ["child"],
    "DescendantAdded": ["descendant"], "DescendantRemoving": ["descendant"],
    "AncestryChanged": ["child", "parent"],
    "Touched": ["hit"], "TouchEnded": ["hit"],
    "InputBegan": ["input", "gameProcessed"], "InputChanged": ["input", "gameProcessed"],
    "InputEnded": ["input", "gameProcessed"],
    "TouchStarted": ["touch", "gameProcessed"], "TouchMoved": ["touch", "gameProcessed"],
    "TouchEnded_": ["touch", "gameProcessed"],
    "Heartbeat": ["deltaTime"], "RenderStepped": ["deltaTime"], "PreRender": ["deltaTime"],
    "PreSimulation": ["deltaTime"], "PostSimulation": ["deltaTime"], "PreAnimation": ["deltaTime"],
    "Stepped": ["time", "deltaTime"],
    "HealthChanged": ["health"], "StateChanged": ["oldState", "newState"],
    "Running": ["speed"], "MoveToFinished": ["reached"], "Chatted": ["message"],
    "AttributeChanged": ["attribute"], "FocusLost": ["enterPressed", "input"],
    "Activated": ["input", "clickCount"], "Triggered": ["player"],
    "PromptTriggered": ["prompt", "player"], "MouseButton1Click": [],
    "LastInputTypeChanged": ["inputType"], "CameraTypeChanged": ["cameraType"],
}


def callback_params(lines):
    """Handlers of well-known Roblox events get the parameter names the
    documentation gives them."""
    out = list(lines)
    rx = re.compile(r"[.:]([A-Za-z0-9_]+):(?:Connect|Once|ConnectParallel)\(function\(([^()]*)\)$")
    for i, line in enumerate(out):
        m = rx.search(line.rstrip())
        if not m or m.group(1) not in SIGNAL_PARAMS:
            continue
        names = SIGNAL_PARAMS[m.group(1)]
        params = [x.strip() for x in m.group(2).split(",") if x.strip()]
        end = block_end(out, i)
        taken = set()
        for j in range(i, end + 1):
            taken |= words(out[j])
        for old, new in zip(params, names):
            if not GENERATED.match(old) or new in taken:
                continue
            for j in range(i, end + 1):
                out[j] = replace_token(out[j], old, new)
            taken.add(new)
    return out


def pcall_results(lines):
    """`local a, b = pcall(...)` -> `local success, result = pcall(...)`."""
    out = list(lines)
    rx = re.compile(r"^( *)local (" + TOKEN + r")(?:, (" + TOKEN + r"))? = x?pcall\(")
    for i, line in enumerate(out):
        m = rx.match(line)
        if not m:
            continue
        pad = m.group(1)
        end = len(out)
        for k in range(i + 1, len(out)):
            if out[k].strip() and indent_of(out[k]) < len(pad):
                end = k
                break
        taken = set()
        for j in range(i, end):
            taken |= words(out[j])
        for old, new in ((m.group(2), "success"), (m.group(3), "result")):
            if old and GENERATED.match(old) and new not in taken:
                for j in range(i, end):
                    out[j] = replace_token(out[j], old, new)
                taken.add(new)
    return out


def ident_from_path(path):
    """`ReplicatedStorage/Shared/Sim/Car` -> Car; `.../table-util/init` ->
    TableUtil (an init module is named after its folder)."""
    parts = [x for x in re.split(r"[/\\]", path or "") if x]
    while parts and parts[-1].lower() in ("init", "init.client", "init.server", "main"):
        parts.pop()
    if not parts:
        return None
    base = re.sub(r"\.(client|server|spec|lua|luau)$", "", parts[-1], flags=re.I)
    words_ = [w for w in re.split(r"[^A-Za-z0-9]+", base) if w]
    if not words_:
        return None
    name = words_[0] if len(words_) == 1 else "".join(w[:1].upper() + w[1:] for w in words_)
    if name[0].isdigit():
        name = "_" + name
    return name if re.match("^" + TOKEN + "$", name) and name not in PASS_KEYWORDS else None


def module_table(lines, path):
    """The table a ModuleScript returns is named after the script."""
    name = ident_from_path(path)
    if not name:
        return lines
    last = next((l for l in reversed(lines) if l.strip()), "")
    m = re.match(r"^return (v\d+(?:x\d+)?)$", last)
    if not m:
        return lines
    var = m.group(1)
    decl = [l for l in lines if re.match(rf"^local {var} = (\{{.*\}}|setmetatable\(.*)$", l)]
    if not decl:
        return lines
    taken = set()
    for l in lines:
        taken |= words(l)
    if name in taken:
        return lines
    return [replace_token(l, var, name) for l in lines]


# types recognised from what a value is used for: member -> weight
USAGE_TYPES = [
    ("player", {"Character": 2, "UserId": 3, "DisplayName": 2, "Kick": 3, "LoadCharacter": 3,
                "CharacterAdded": 3, "PlayerGui": 3, "Backpack": 2, "GetMouse": 3, "Team": 1,
                "AccountAge": 3, "MembershipType": 3}),
    ("character", {"Humanoid": 2, "HumanoidRootPart": 3, "PrimaryPart": 1, "GetPivot": 1}),
    ("humanoid", {"WalkSpeed": 3, "JumpPower": 3, "JumpHeight": 3, "MoveDirection": 3, "HipHeight": 3,
                  "Health": 2, "MaxHealth": 3, "ChangeState": 3, "GetState": 3, "Died": 3,
                  "RootPart": 2, "MoveTo": 2, "UseJumpPower": 3, "AutoRotate": 3}),
    ("input", {"KeyCode": 3, "UserInputType": 3, "UserInputState": 3, "Delta": 1}),
    ("track", {"AdjustSpeed": 3, "TimePosition": 2, "IsPlaying": 3, "Looped": 2, "Length": 1,
               "AdjustWeight": 3, "KeyframeReached": 3, "GetMarkerReachedSignal": 3}),
    ("sound", {"SoundId": 3, "PlaybackSpeed": 3, "Volume": 2, "TimeLength": 2}),
    ("camera", {"FieldOfView": 3, "ViewportSize": 3, "CameraType": 3, "CameraSubject": 3,
                "ViewportPointToRay": 3, "WorldToViewportPoint": 3, "ScreenPointToRay": 3}),
    ("part", {"Anchored": 3, "CanCollide": 3, "Material": 2, "Touched": 3, "AssemblyLinearVelocity": 3,
              "BrickColor": 2, "Transparency": 1}),
    ("gui", {"AbsoluteSize": 3, "AbsolutePosition": 3, "ZIndex": 2, "BackgroundTransparency": 2,
             "TextColor3": 2, "Text": 1, "Visible": 1}),
    ("connection", {"Disconnect": 3, "Connected": 2}),
    ("tween", {"Play": 1, "Cancel": 2, "Completed": 2}),
]


def usage_name(name, scope_lines):
    """What a value is, from the members it is used with."""
    score = {}
    rx = re.compile(rf"(?<![A-Za-z0-9_.]){re.escape(name)}[.:]([A-Za-z_][A-Za-z0-9_]*)")
    seen = set()
    for l in scope_lines:
        for m in rx.finditer(l):
            seen.add(m.group(1))
    # `x:FindFirstChild("Humanoid")` says x is a character
    for l in scope_lines:
        if re.search(rf"(?<![A-Za-z0-9_.]){re.escape(name)}:(?:FindFirstChild|WaitForChild|FindFirstChildOfClass)\(\"(Humanoid|HumanoidRootPart)\"", l):
            seen.add("Humanoid")
    for kind, members in USAGE_TYPES:
        s = sum(w for m, w in members.items() if m in seen)
        if s:
            score[kind] = s
    if not score:
        return None
    best = max(score, key=score.get)
    return best if score[best] >= 3 else None


GETTER = re.compile(r"^(?:get|Get|find|Find|make|Make|create|Create|build|Build|compute|Compute|calculate|Calculate)([A-Z][A-Za-z0-9_]*)$")


def function_name_of(header):
    m = re.search(r"function (?:[A-Za-z_][A-Za-z0-9_.]*[.:])?([A-Za-z_][A-Za-z0-9_]*)\(", header)
    if m:
        return m.group(1)
    m = re.match(r"^ *local ([A-Za-z_][A-Za-z0-9_]*) = function\(", header)
    return m.group(1) if m else None


def collect_calls(lines, calls=None):
    """(kind, function name) -> argument lists, kind "" f(...), "." t.f(...),
    ":" t:f(...)."""
    calls = {} if calls is None else calls
    call_rx = re.compile(r"(?:(?<![A-Za-z0-9_.:])|([.:]))([A-Za-z_][A-Za-z0-9_]*)\(")
    for l in lines:
        if FUNC_OPEN.search(l.rstrip()) and re.match(r"^ *(local )?function ", l):
            continue  # a definition, not a call
        for m in call_rx.finditer(l):
            if m.group(2) in PASS_KEYWORDS:
                continue
            close = matching_paren(l, m.end() - 1)
            if close > 0:
                calls.setdefault((m.group(1) or "", m.group(2)), []).append(
                    split_top(l[m.end():close]))
    return calls


EXTRA_CALLS = {}  # call sites seen in the other scripts of the same place


def more_names(lines):
    """Second naming round, for generated names still left:
      * `local x = setmetatable({}, T)` -> self (outside methods)
      * stored into a field (`self.speed = x`, `{speed = x}`) -> speed
      * returned at the end of `GetFoo` -> foo
      * used like a known Roblox type (`x.Character`, `x.UserId`) -> player
    Parameters take part too, and are named from their call sites first."""
    out = list(lines)
    funcs = sorted(functions(out), key=lambda f: f[0])
    # the module's own table: `self` passed around in its methods is one
    module = None
    last = next((l for l in reversed(out) if l.strip()), "")
    m = re.match(r"^return ([A-Za-z_][A-Za-z0-9_]*)$", last)
    if m and not GENERATED.match(m.group(1)) and any(
            l.startswith(f"local {m.group(1)} = ") for l in out):
        module = m.group(1)
    # call sites: (kind, function name) -> list of argument lists, where kind
    # is "" for f(...), "." for t.f(...) and ":" for t:f(...)
    calls = collect_calls(out)
    for k, v in EXTRA_CALLS.items():
        if k[0]:  # only t.f / t:f calls reach across scripts
            calls.setdefault(k, []).extend(v)
    for fi, fend, params in funcs:
        if fi < 0:
            continue
        lo, hi = fi + 1, min(fend, len(out))
        depth = indent_of(out[fi]) // 4 + 1
        header = out[fi]
        fname = function_name_of(header)
        scope = out[fi:hi + 1]
        taken = set()
        for l in scope:
            taken |= words(l)
        cands = {}
        # parameters, from what callers pass
        kind = ":" if re.search(r"function [A-Za-z_][A-Za-z0-9_.]*:", header) else             "." if re.search(r"function [A-Za-z_][A-Za-z0-9_]*\.", header) or             re.match(r"^ *[A-Za-z_][A-Za-z0-9_.]*\.[A-Za-z_][A-Za-z0-9_]* = function\(", header) else ""
        sites = calls.get((kind, fname), []) if fname else []
        if sites:
            for i, pname in enumerate(params):
                if not GENERATED.match(pname):
                    continue
                votes = {}
                for args in sites:
                    if i < len(args):
                        a = args[i].strip()
                        n = a if re.match("^" + TOKEN + "$", a) and not GENERATED.match(a) else derive_name(a)
                        if n == "self":
                            n = lower_first(module) if module and kind != ":" else None
                        if n and not GENERATED.match(n):
                            votes[n] = votes.get(n, 0) + 1
                if votes:
                    best = max(votes, key=votes.get)
                    if votes[best] * 2 >= len(sites):
                        cands[pname] = lower_first(best) if best[:1].isupper() and not best.isupper() else best
        own = set(params)
        for j in range(lo, hi):
            m = re.match(r"^ *local (" + TOKEN + r"(?:, " + TOKEN + r")*)( = .*)?$", out[j])
            if m:
                own |= set(m.group(1).split(", "))
        own = {n for n in own if re.fullmatch(rf"(v\d+(x\d+)?|arg\d+)_{depth}", n)}
        for name in sorted(own):
            if name in cands:
                continue
            defs = [j for j in range(lo, hi) if re.match(rf"^ *(local )?{re.escape(name)} = ", out[j])]
            new = None
            if len(defs) == 1:
                rhs = out[defs[0]].strip().split(" = ", 1)[1]
                if rhs.startswith("setmetatable(") and "self" not in taken:
                    new = "self"
            if not new:
                for j in range(lo, hi):
                    m = re.match(rf"^ *[A-Za-z_][A-Za-z0-9_.]*\.([A-Za-z_][A-Za-z0-9_]*) = {re.escape(name)}$", out[j]) or \
                        re.search(rf"[{{,] ([A-Za-z_][A-Za-z0-9_]*) = {re.escape(name)}[,}}]", out[j])
                    if m and len(defs) <= 1:
                        f = m.group(1)
                        new = f if not f[:1].isupper() or f.isupper() else lower_first(f)
                        break
            if not new and fname and GETTER.match(fname) and len(defs) <= 2:
                last = next((l.strip() for l in reversed(out[lo:hi]) if l.strip()), "")
                if last == f"return {name}":
                    new = lower_first(GETTER.match(fname).group(1))
            if not new:
                new = usage_name(name, out[lo:hi])
            if new:
                cands[name] = new
        for old, new in cands.items():
            if not re.match("^" + TOKEN + "$", new) or new in PASS_KEYWORDS or new in taken:
                continue
            for j in range(fi, hi + 1 if hi < len(out) else hi):
                out[j] = replace_token(out[j], old, new)
            taken.add(new)
    return out


def blank_lines(lines):
    """A blank line between top-level statements and function definitions."""
    out = []
    for i, line in enumerate(lines):
        s = line.strip()
        prev = out[-1].strip() if out else ""
        opens = indent_of(line) == 0 and FUNC_OPEN.search(line.rstrip()) and (
            s.startswith(("local function ", "function ")) or
            re.match(r"^[A-Za-z_][A-Za-z0-9_.]* = function\(", s))
        if opens and prev and not prev.startswith("--"):
            out.append("")
        out.append(line)
        if line == "end" and i + 1 < len(lines) and lines[i + 1].strip():
            out.append("")
    return out


BLOCK_WORDS = re.compile(r"(?<![A-Za-z0-9_.:])(function|end|until|repeat|do)(?![A-Za-z0-9_])")


def code_words(line):
    """Block keywords of a line, outside strings (`{...}` in backtick strings
    is code, but never holds blocks)."""
    plain = []
    quote = None
    k = 0
    while k < len(line):
        ch = line[k]
        if quote:
            if ch == "\\":
                k += 2
                continue
            if ch == quote:
                quote = None
            plain.append(" ")
        elif ch in "\"'`":
            quote = ch
            plain.append(" ")
        elif line.startswith("--", k):
            break
        else:
            plain.append(ch)
        k += 1
    return BLOCK_WORDS.findall("".join(plain)), "".join(plain).strip()


def reindent(lines):
    """Indentation recomputed from the code: four spaces per open block.
    Opening: `function`, an if/elseif/while/for statement, `do`, `repeat`;
    closing: `end`, `until`. A line that starts by closing a block (or with
    else/elseif) sits one level out."""
    out = []
    depth = 0
    skip_do = 0  # a loop header's own `do`, possibly on a later line
    for line in lines:
        s = line.strip()
        if not s:
            out.append("")
            continue
        words_, code = code_words(s)
        opens = words_.count("function") + words_.count("repeat")
        closes = words_.count("end") + words_.count("until")
        first = s.split(" ", 1)[0].split("(", 1)[0].split(",", 1)[0].split(")", 1)[0]
        if first == "if":
            opens += 1  # a statement if always has an `end`, wherever `then` is
        if first in ("while", "for"):
            opens += 1
            skip_do += 1
        do_count = words_.count("do")
        while do_count and skip_do:
            do_count -= 1
            skip_do -= 1
        opens += do_count
        lead_close = first in ("end", "until", "else", "elseif") or s.startswith(("end)", "end,", "end}", "}"))
        level = depth - 1 if lead_close else depth
        out.append("    " * max(level, 0) + s)
        depth += opens - closes
        depth = max(depth, 0)
    return out


def tidy(lines):
    lines = reindent(lines)
    for f in (generic_for, compound_assign, interpolation, local_functions,
              sink_declarations, rename_variables, callback_params, pcall_results,
              more_names, more_names, blank_lines):
        lines = f(lines)
    return lines


def decompile(mod, path=None):
    main = Decompiler(mod, mod["main"], [], -1)
    params, body = main.decompile()
    lines = if_expressions("\n".join(body).split("\n"))
    lines = uninline(lines)
    lines = module_table(lines, path)
    lines = tidy(lines)
    text = "\n".join(lines)
    # labels that a goto names but no structure absorbed
    return text + "\n"


def disassemble(mod):
    return "\n\n".join(Fn(mod, i).disasm() for i in range(len(mod["protos"])))


def main(argv):
    upstream = "--upstream" in argv  # standard Luau bytecode (unscrambled)
    argv = [a for a in argv if a != "--upstream"]
    if len(argv) >= 2 and argv[0] == "--dir":
        src = argv[1]
        dst = argv[argv.index("--out") + 1] if "--out" in argv else os.path.join(src, "lua")
        names = {}
        idx = os.path.join(src, "index.tsv")
        if os.path.exists(idx):
            for line in open(idx, encoding="utf-8", errors="replace"):
                f = line.rstrip("\n").split("\t")
                if len(f) >= 6:
                    names[f[0]] = f[5]
        ok = bad = 0
        written = set()
        # first pass: call sites across the place, for parameter names
        EXTRA_CALLS.clear()
        for fn in sorted(os.listdir(src)):
            if fn.endswith(".wire"):
                try:
                    m0 = parse(open(os.path.join(src, fn), "rb").read(), scrambled=not upstream)
                    text0 = decompile(m0, names.get(fn[:-5]))
                    collect_calls(text0.split("\n"), EXTRA_CALLS)
                except Exception:
                    pass
        for fn in sorted(os.listdir(src)):
            if not fn.endswith(".wire"):
                continue
            sid = fn[:-5]
            rel = names.get(sid, sid)
            rel = re.sub(r"[^A-Za-z0-9_./ -]", "_", rel).strip("/") or sid
            path = os.path.join(dst, rel + ".lua")
            if path in written:  # two scripts at the same game path
                path = os.path.join(dst, f"{rel} ({sid}).lua")
            written.add(path)
            os.makedirs(os.path.dirname(path), exist_ok=True)
            data = open(os.path.join(src, fn), "rb").read()
            try:
                mod = parse(data, scrambled=not upstream)
                text = decompile(mod, names.get(sid))
                ok += 1
            except Exception as e:  # keep going; report at the end
                text = f"-- decompile failed: {e}\n"
                bad += 1
            with open(path, "w", encoding="utf-8") as f:
                f.write(f"-- {rel} (script {sid})\n" + text)
        print(f"{ok} decompiled, {bad} failed -> {dst}")
        return 0 if bad == 0 else 1
    disasm = "--disasm" in argv
    files = [a for a in argv if not a.startswith("--")]
    if not files:
        print(__doc__)
        return 2
    mod = parse(open(files[0], "rb").read(), scrambled=not upstream)
    print(disassemble(mod) if disasm else decompile(mod), end="")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
