#!/usr/bin/env python3
"""luau_difftest.py - differential test of the Luau decompiler.

Generates random Luau programs (deterministic, always terminating, printing
the values they compute), compiles each at -O2 like Roblox does, decompiles
the bytecode, and runs the original and the decompiled program side by side
with tools/luau_run: their output must be identical. Also checks that the
decompiled program compiles, and reports how many functions recompile to the
original instruction sequence.

  luau_difftest.py [--count N] [--seed S] [--keep DIR] [--size N]
                   [--python | --bin PATH]

Decompiles with the Rust binary (target/release) when it is built, else
with tools/luau_decompile.py in-process. --python forces that; --bin picks
another binary (a .py path runs under this Python, as a subprocess).
Failing cases (source, bytecode, decompiled, both outputs) go to DIR.
"""
import concurrent.futures as cf
import os
import random
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import luau_decompile as L  # noqa: E402
import luau_roundtrip as R  # noqa: E402

ROOT = os.path.dirname(HERE)
# luau_check and luau_run: see CMakeLists.txt
BIN = os.environ.get("LUAU_BIN", os.path.join(ROOT, "build", "Release"))
DECOMPILER = os.path.join(ROOT, "target", "release", "luau_decompile.exe")


# ------------------------------------------------------------------ generator

class Gen:
    """Typed random program generator. Every expression has a type (num,
    bool, str) and never errors at run time; loops are bounded."""

    def __init__(self, seed, size):
        self.r = random.Random(seed)
        self.size = size
        self.n = 0
        self.lines = []
        self.helpers = []  # (name, param types, return type)

    def fresh(self, base="v"):
        self.n += 1
        return f"{base}{self.n}"

    # -- expressions
    def num(self, sc, d=0):
        r = self.r
        nums = [v for v, t in sc if t == "num"]
        c = r.random()
        if d > 3 or c < 0.25:
            if nums and r.random() < 0.7:
                return r.choice(nums)
            return str(r.choice([0, 1, 2, 3, 5, 7, 10, -1, -4, 100, 0.5, 2.25]))
        k = r.randrange(14)
        if k == 0:
            return f"{self.num(sc, d + 1)} + {self.num(sc, d + 1)}"
        if k == 1:
            return f"{self.num(sc, d + 1)} - {self.num(sc, d + 1)}"
        if k == 2:
            return f"({self.num(sc, d + 1)}) * {r.choice([2, 3, -1, 0.5])}"
        if k == 3:
            return f"({self.num(sc, d + 1)}) // {r.choice([2, 3, 4])}"
        if k == 4:
            return f"({self.num(sc, d + 1)}) % {r.choice([3, 5, 7])}"
        if k == 5:
            return f"math.{r.choice(['max', 'min'])}({self.num(sc, d + 1)}, {self.num(sc, d + 1)})"
        if k == 6:
            return f"math.abs({self.num(sc, d + 1)})"
        if k == 7:
            return f"(if {self.boolean(sc, d + 1)} then {self.num(sc, d + 1)} else {self.num(sc, d + 1)})"
        if k == 8:
            return f"({self.boolean(sc, d + 1)} and {self.num(sc, d + 1)} or {self.num(sc, d + 1)})"
        if k == 9:
            arrs = [v for v, t in sc if t == "arr"]
            if arrs:
                return f"#{r.choice(arrs)}"
        if k == 10:
            objs = [v for v, t in sc if t == "obj"]
            if objs:
                o = r.choice(objs)
                return r.choice([f"{o}.x", f"{o}.y", f"{o}:get()"])
        if k == 11:
            hs = [h for h in self.helpers if h[2] == "num"]
            if hs:
                return self.call(r.choice(hs), sc, d)
        if k == 12:
            return f"-{self.atom_num(sc)}"
        if k == 13:
            return f"string.len({self.string(sc, d + 1)})"
        return self.atom_num(sc)

    def atom_num(self, sc):
        nums = [v for v, t in sc if t == "num"]
        return self.r.choice(nums) if nums else str(self.r.randrange(10))

    def boolean(self, sc, d=0):
        r = self.r
        bools = [v for v, t in sc if t == "bool"]
        c = r.random()
        if d > 3 or c < 0.2:
            if bools and r.random() < 0.7:
                return r.choice(bools)
            return r.choice(["true", "false"])
        k = r.randrange(8)
        if k == 0:
            return f"{self.num(sc, d + 1)} {r.choice(['<', '<=', '>', '>=', '==', '~='])} {self.num(sc, d + 1)}"
        if k == 1:
            return f"not ({self.boolean(sc, d + 1)})"
        if k == 2:
            return f"({self.boolean(sc, d + 1)} and {self.boolean(sc, d + 1)})"
        if k == 3:
            return f"({self.boolean(sc, d + 1)} or {self.boolean(sc, d + 1)})"
        if k == 4:
            return f"{self.string(sc, d + 1)} == {self.string(sc, d + 1)}"
        if k == 5:
            hs = [h for h in self.helpers if h[2] == "bool"]
            if hs:
                return self.call(r.choice(hs), sc, d)
        if k == 6:
            maybes = [v for v, t in sc if t == "maybe"]
            if maybes:
                return f"{r.choice(maybes)} {r.choice(['==', '~='])} nil"
        return f"{self.num(sc, d + 1)} > {self.num(sc, d + 1)}"

    def string(self, sc, d=0):
        r = self.r
        strs = [v for v, t in sc if t == "str"]
        c = r.random()
        if d > 3 or c < 0.3:
            if strs and r.random() < 0.6:
                return r.choice(strs)
            return '"' + r.choice(["a", "bc", "x y", "", "hello", "%", "{q}"]) + '"'
        k = r.randrange(5)
        if k == 0:
            return f"{self.string(sc, d + 1)} .. {self.string(sc, d + 1)}"
        if k == 1:
            return f"`<{{{self.num(sc, d + 1)}}}:{{{self.string(sc, d + 1)}}}>`"
        if k == 2:
            return f"tostring({self.num(sc, d + 1)})"
        if k == 3:
            return f"string.rep({self.string(sc, d + 2)}, {r.choice([0, 1, 2])})"
        return f"(if {self.boolean(sc, d + 1)} then {self.string(sc, d + 1)} else {self.string(sc, d + 1)})"

    def expr(self, t, sc):
        return {"num": self.num, "bool": self.boolean, "str": self.string}[t](sc)

    def call(self, h, sc, d=0):
        name, ptypes, _ = h
        args = ", ".join(self.expr(t, sc) if d < 3 else self.lit(t) for t in ptypes)
        return f"{name}({args})"

    def lit(self, t):
        return {"num": "1", "bool": "true", "str": '"z"'}[t]

    # -- statements
    def emit(self, ind, text):
        self.lines.append("    " * ind + text)

    def block(self, ind, sc, budget, in_loop=False, in_func=None):
        sc = list(sc)
        for _ in range(budget):
            self.stmt(ind, sc, in_loop, in_func)

    def stmt(self, ind, sc, in_loop, in_func):
        r = self.r
        k = r.randrange(26)
        t = r.choice(["num", "num", "bool", "str"])
        if k <= 2:
            v = self.fresh()
            self.emit(ind, f"local {v} = {self.expr(t, sc)}")
            sc.append((v, t))
        elif k == 3:
            own = [(v, ty) for v, ty in sc if ty in ("num", "bool", "str") and v[0] not in "ic"]
            if own:
                v, ty = r.choice(own)
                if ty == "num" and r.random() < 0.5:
                    self.emit(ind, f"{v} {r.choice(['+=', '-=', '*='])} {self.atom_num(sc)}")
                elif ty == "str" and r.random() < 0.3:
                    self.emit(ind, f"{v} ..= {self.string(sc)}")
                else:
                    self.emit(ind, f"{v} = {self.expr(ty, sc)}")
        elif k <= 5:
            self.emit(ind, f"if {self.boolean(sc)} then")
            self.block(ind + 1, sc, r.randrange(1, 4), in_loop, in_func)
            for _ in range(r.randrange(3)):
                self.emit(ind, f"elseif {self.boolean(sc)} then")
                self.block(ind + 1, sc, r.randrange(1, 3), in_loop, in_func)
            if r.random() < 0.5:
                self.emit(ind, "else")
                self.block(ind + 1, sc, r.randrange(1, 3), in_loop, in_func)
            self.emit(ind, "end")
        elif k == 6 and ind < 4:
            i = self.fresh("i")
            self.emit(ind, f"for {i} = {r.choice([1, 0, 3])}, {r.choice([3, 4, 1])}{r.choice(['', ', -1', ', 2'])} do")
            self.block(ind + 1, sc + [(i, "num")], r.randrange(1, 4), True, in_func)
            self.emit(ind, "end")
        elif k == 7 and ind < 4:
            arrs = [v for v, ty in sc if ty == "arr"]
            if arrs:
                i, x = self.fresh("i"), self.fresh("x")
                form = r.choice(["{a}", "ipairs({a})"]).format(a=r.choice(arrs))
                self.emit(ind, f"for {i}, {x} in {form} do")
                self.block(ind + 1, sc + [(i, "num"), (x, "num")], r.randrange(1, 4), True, in_func)
                self.emit(ind, "end")
        elif k == 8 and ind < 4:
            c = self.fresh("c")
            self.emit(ind, f"local {c} = 0")
            if r.random() < 0.5:
                self.emit(ind, f"while {c} < {r.choice([2, 3, 4])} do")
                self.emit(ind + 1, f"{c} += 1")
                self.block(ind + 1, sc + [(c, "num")], r.randrange(1, 3), True, in_func)
                self.emit(ind, "end")
            else:
                self.emit(ind, "repeat")
                self.emit(ind + 1, f"{c} += 1")
                self.block(ind + 1, sc + [(c, "num")], r.randrange(1, 3), True, in_func)
                self.emit(ind, f"until {c} >= {r.choice([2, 3])}")
        elif k == 9 and in_loop:
            self.emit(ind, f"if {self.boolean(sc)} then {r.choice(['break', 'continue'])} end")
        elif k == 10 and in_func:
            self.emit(ind, f"if {self.boolean(sc)} then return {self.expr(in_func, sc)} end")
        elif k == 11 and not in_loop:  # growing an array inside a loop may never end
            arrs = [v for v, ty in sc if ty == "arr"]
            if arrs:
                a = r.choice(arrs)
                self.emit(ind, r.choice([f"table.insert({a}, {self.num(sc)})",
                                         f"{a}[{r.randrange(1, 4)}] = {self.num(sc)}"]))
        elif k == 12:
            objs = [v for v, ty in sc if ty == "obj"]
            if objs:
                o = r.choice(objs)
                self.emit(ind, r.choice([f"{o}.x = {self.num(sc)}", f"{o}:bump({self.num(sc)})"]))
        elif k == 13:
            v = self.fresh()
            self.emit(ind, f"local {v} = if {self.boolean(sc)} then {self.num(sc)} else nil")
            sc.append((v, "maybe"))
            self.emit(ind, f"print({v} or {self.num(sc)})")
        elif k == 14:
            ok = self.fresh("ok")
            self.emit(ind, f"local {ok} = pcall(function()")
            self.emit(ind + 1, f"if {self.boolean(sc)} then error(\"boom\") end")
            self.emit(ind, "end)")
            self.emit(ind, f"print({ok})")
        elif k == 15:  # several targets at once: values in temporaries
            own = [v for v, ty in sc if ty == "num" and v[0] not in "ic"]
            if len(own) >= 2:
                a, b = r.sample(own, 2)
                c = r.choice(own)
                self.emit(ind, r.choice([f"{a}, {b} = {b}, {a}", f"{a}, {b} = {b}, {a} + {self.atom_num(sc)}",
                                         f"{a}, {b} = {b} * 2, {c} - 1"]))
                self.emit(ind, f"print({a}, {b})")
        elif k == 16:  # a table a closure captures, built with calls inside
            v, h = self.fresh("t"), self.fresh("h")
            self.emit(ind, f"local {v} = {{{self.num(sc)}, tostring({self.num(sc)}), k = {self.num(sc)}}}")
            self.emit(ind, f"local function {h}() return #{v}, {v}.k end")
            self.emit(ind, f"print({h}())")
        elif k == 17:  # all of a call's results, or of ...
            v = self.fresh("t")
            self.emit(ind, f"local {v} = {{{self.num(sc)}, select(1, {self.num(sc)}, {self.num(sc)})}}")
            self.emit(ind, f"print(#{v})")
            f, x, y = self.fresh("f"), self.fresh("x"), self.fresh("x")
            self.emit(ind, f"local function {f}(...) local {x}, {y} = ... return {y}, {x}, select('#', ...) end")
            self.emit(ind, f"print({f}({self.num(sc)}, {self.num(sc)}, {self.num(sc)}))")
        elif k == 18:  # a variable a closure changes, read around the call
            v, h = self.fresh("n"), self.fresh("h")
            self.emit(ind, f"local {v} = {self.num(sc)}")
            self.emit(ind, f"local function {h}() {v} += 1 return {v} end")
            a = self.fresh()
            self.emit(ind, f"local {a} = {v}")
            self.emit(ind, f"print({a}, {h}(), {v}, {a} + {v})")
            sc.append((a, "num"))
        elif k == 19 and ind < 4:  # order: both sides are calls
            s = self.string(sc)
            log = self.fresh("o")
            self.emit(ind, f"local {log} = {{}}")
            w = f"(function(v) table.insert({log}, v) return v end)"
            x, y = self.num(sc), self.num(sc)
            op = r.choice([">", ">=", "<", "<=", "==", "+", "-"])
            self.emit(ind, f"print({w}({x}) {op} {w}({y}), #{log}, {log}[1])")
        elif k == 20:  # a value from before a branch, read in its arms
            v = self.fresh()
            self.emit(ind, f"local {v} = math.abs({self.num(sc)})")
            self.emit(ind, f"if {self.boolean(sc)} then")
            self.emit(ind + 1, f"{v} = -{v}")
            self.emit(ind, "end")
            self.emit(ind, f"print({v})")
            sc.append((v, "num"))
        elif k == 21 and ind < 4:  # chains, with variables tested and then used
            a, b = self.fresh(), self.fresh()
            self.emit(ind, f"local {a} = if {self.boolean(sc)} then {self.num(sc)} else nil")
            self.emit(ind, f"local {b} = if {self.boolean(sc)} then {self.num(sc)} else nil")
            self.emit(ind, f"if {a} and {b} then")
            self.emit(ind + 1, f"print({a} + {b})")
            self.emit(ind, "else")
            self.emit(ind + 1, f"print({a} or {b} or {self.num(sc)}, ({a} or {b}) and {self.num(sc)})")
            self.emit(ind, "end")
        elif k == 22 and in_func is None and not in_loop:  # a loop, then an else
            i = self.fresh("i")
            self.emit(ind, f"if {self.boolean(sc)} then")
            self.emit(ind + 1, f"for {i} = 1, 2 do print({i}) end")
            self.emit(ind, f"elseif {self.boolean(sc)} then")
            self.emit(ind + 1, f"print({self.num(sc)})")
            self.emit(ind, "end")
        elif k == 23:  # a loop header with a computed iterator
            arrs = [v for v, ty in sc if ty == "arr"]
            if arrs and ind < 4:
                i, x = self.fresh("i"), self.fresh("x")
                a = r.choice(arrs)
                self.emit(ind, f"for {i}, {x} in ipairs(if {self.boolean(sc)} then {a} else {{{self.num(sc)}}}) do")
                self.block(ind + 1, sc + [(i, "num"), (x, "num")], 1, True, in_func)
                self.emit(ind, "end")
        else:
            vals = [v for v, ty in sc if ty in ("num", "bool", "str")]
            picks = [self.r.choice(vals) for _ in range(min(3, len(vals)))] if vals else []
            picks.append(self.expr(t, sc))
            self.emit(ind, f"print({', '.join(picks)})")

    def helper(self):
        """A small local function (Luau -O2 inlines these) or a bigger one."""
        r = self.r
        name = self.fresh("f")
        ptypes = [r.choice(["num", "num", "bool", "str"]) for _ in range(r.randrange(1, 4))]
        params = [(self.fresh("p"), t) for t in ptypes]
        rt = r.choice(["num", "num", "bool", "str"])
        self.emit(0, f"local function {name}({', '.join(p for p, _ in params)})")
        if r.random() < 0.5:
            self.emit(1, f"return {self.expr(rt, params)}")
        else:
            for _ in range(r.randrange(1, 3)):
                self.emit(1, f"if {self.boolean(params)} then")
                self.emit(2, f"return {self.expr(rt, params)}")
                self.emit(1, "end")
            if r.random() < 0.5:
                self.block(1, params, r.randrange(1, 3), False, rt)
            self.emit(1, f"return {self.expr(rt, params)}")
        self.emit(0, "end")
        self.helpers.append((name, ptypes, rt))

    def program(self):
        r = self.r
        for _ in range(r.randrange(1, 4)):
            self.helper()
        sc = []
        a = self.fresh("arr")
        self.emit(0, f"local {a} = {{{', '.join(str(r.randrange(-5, 9)) for _ in range(r.randrange(1, 5)))}}}")
        sc.append((a, "arr"))
        o = self.fresh("obj")
        self.emit(0, f"local {o} = {{x = {r.randrange(9)}, y = {r.randrange(9)}}}")
        self.emit(0, f"function {o}:get() return self.x * 2 + self.y end")
        self.emit(0, f"function {o}:bump(n) self.x += n end")
        sc.append((o, "obj"))
        mk = self.fresh("mk")
        self.emit(0, f"local function {mk}(n)")
        self.emit(1, "local count = 0")
        self.emit(1, "return function(x) count += 1 return x + n * count end")
        self.emit(0, "end")
        adder = self.fresh("add")
        self.emit(0, f"local {adder} = {mk}({r.randrange(1, 5)})")
        self.block(0, sc, self.size)
        self.emit(0, f"print({adder}(1), {adder}(2), {o}.x, {o}.y, #{a})")
        self.emit(0, f"for i, x in {a} do print(i, x) end")
        return "\n".join(self.lines) + "\n"


# ------------------------------------------------------------------ driver

def run(cmd, files=(), chunk=40):
    """Run cmd over files in batches (Windows limits command line length)."""
    files = list(files)
    if not files:
        return subprocess.run(cmd, capture_output=True, text=True).stdout
    return "".join(subprocess.run(cmd + files[i:i + chunk], capture_output=True, text=True).stdout
                   for i in range(0, len(files), chunk))


def emit(check, files, outdir, chunk=40):
    """Compile files at -O2; {source path: bytecode path}."""
    mapping = {}
    for i in range(0, len(files), chunk):
        sub = os.path.join(outdir, str(i))
        os.makedirs(sub)
        out = subprocess.run([check, "--emit", sub] + files[i:i + chunk],
                             capture_output=True, text=True).stdout
        for line in out.splitlines():
            m = re.match(r"^(\d+) (.+)$", line)
            if m and m.group(2) in files:
                mapping[m.group(2)] = os.path.join(sub, m.group(1) + ".bc")
    return mapping


def outputs(text):
    """luau_run output -> {path: output text}."""
    res, cur, buf = {}, None, []
    for line in text.splitlines():
        m = re.match(r"^=== (.+)$", line)
        if m and cur is None:
            cur, buf = m.group(1), []
        elif m and (m.group(1) == "ok" or m.group(1).startswith("error:")):
            res[cur] = "\n".join(buf + [line])
            cur = None
        else:
            buf.append(line)
    return res


def decompile_python(bc):
    """(text, None) or (None, why), in-process with tools/luau_decompile.py."""
    try:
        with open(bc, "rb") as f:
            return L.decompile(L.parse(f.read(), scrambled=False)), None
    except Exception as e:  # report and keep going
        return None, repr(e)


def decompile_binary(cmd, bc):
    """(text, None) or (None, why), through a decompiler subprocess."""
    try:
        p = subprocess.run(cmd + ["--upstream", bc], capture_output=True, timeout=300)
    except subprocess.TimeoutExpired:
        return None, "timed out"
    if p.returncode != 0:
        tail = p.stderr.decode("utf-8", "replace").strip().splitlines()[-1:]
        return None, f"exit status {p.returncode}: {tail[0] if tail else ''}"
    return p.stdout.replace(b"\r\n", b"\n").decode("utf-8", "replace"), None


def decompile_all(bcs, binary):
    """{bytecode path: (text, why)}; subprocesses run in parallel."""
    if binary is None:
        return {bc: decompile_python(bc) for bc in bcs}
    cmd = [sys.executable, "-I", binary] if binary.endswith(".py") else [binary]
    with cf.ThreadPoolExecutor(os.cpu_count() or 4) as ex:
        return dict(zip(bcs, ex.map(lambda bc: decompile_binary(cmd, bc), bcs)))


def main(argv):
    binary = argv[argv.index("--bin") + 1] if "--bin" in argv else DECOMPILER
    if "--python" in argv or ("--bin" not in argv and not os.path.isfile(binary)):
        binary = None
    print(f"decompiler: {binary or 'tools/luau_decompile.py (in-process)'}")
    count = int(argv[argv.index("--count") + 1]) if "--count" in argv else 200
    seed = int(argv[argv.index("--seed") + 1]) if "--seed" in argv else 1
    size = int(argv[argv.index("--size") + 1]) if "--size" in argv else 25
    keep = argv[argv.index("--keep") + 1] if "--keep" in argv else None
    check = os.path.join(BIN, "luau_check.exe")
    runner = os.path.join(BIN, "luau_run.exe")
    tmp = tempfile.mkdtemp(prefix="luau_dt_")
    try:
        srcs = []
        for k in range(count):
            p = os.path.join(tmp, f"g{seed + k}.luau")
            with open(p, "w", encoding="utf-8") as f:
                f.write(Gen(seed + k, size).program())
            srcs.append(p)
        bad_src = [l for l in run([check], srcs).splitlines() if ": " in l and not l.endswith("fail")]
        if bad_src:
            print("generator wrote invalid programs:", bad_src[:3])
        mapping = emit(check, srcs, os.path.join(tmp, "bc"))
        decs = {}
        crashed = []
        done = decompile_all([mapping[s] for s in srcs if s in mapping], binary)
        for s in srcs:
            if s not in mapping:
                continue
            text, why = done[mapping[s]]
            if why is not None:
                crashed.append((s, why))
                continue
            d = s[:-5] + ".dec.luau"
            with open(d, "w", encoding="utf-8") as f:
                f.write(text)
            decs[s] = d
        compile_fail = [l.split(": ", 1)[0] for l in run([check], list(decs.values())).splitlines()
                        if ": " in l and not l.endswith("fail")]
        out = outputs(run([runner], srcs + list(decs.values())))
        same = differ = skipped = 0
        failures = []
        for s, d in decs.items():
            if d in compile_fail:
                failures.append((s, d, "decompiled program does not compile"))
                continue
            a, b = out.get(s), out.get(d)
            if a is not None and a.endswith("budget exceeded"):
                skipped += 1  # the original itself does not finish
                continue
            if a == b:
                same += 1
            else:
                differ += 1
                failures.append((s, d, "output differs"))
        # how many functions recompile to the original instruction sequence
        funcs = ident = 0
        dmap = emit(check, list(decs.values()), os.path.join(tmp, "dbc"))
        for s, d in decs.items():
            if d not in dmap:
                continue
            a = L.parse(open(mapping[s], "rb").read(), scrambled=False)["protos"]
            b = L.parse(open(dmap[d], "rb").read(), scrambled=False)["protos"]
            paired = R.pair_protos(a, b)
            for i, pa in enumerate(a):
                funcs += 1
                ident += i in paired and R.shape(pa) == R.shape(b[paired[i]])
        print(f"{count} programs ({skipped} skipped: never finish): {same} behave identically, {differ} differ, "
              f"{len(compile_fail)} decompiled programs do not compile, {len(crashed)} decompiler crashes")
        print(f"{funcs} functions: {ident} recompile instruction-for-instruction ({ident / max(funcs, 1):.1%})")
        texts = [open(d, encoding="utf-8").read() for d in decs.values()]
        flags = sum(bool(re.search(r"local jump_\d+ = false", x)) for x in texts)
        blocks = sum("local block = 0" in x for x in texts)
        print(f"fallbacks: {flags} programs with jump flags, {blocks} with a dispatcher")
        for s, e in crashed[:5]:
            print("  crash:", os.path.basename(s), e)
        if keep and (failures or crashed):
            os.makedirs(keep, exist_ok=True)
            for s, d, why in failures:
                base = os.path.join(keep, os.path.basename(s)[:-5])
                shutil.copy(s, base + ".src.luau")
                shutil.copy(d, base + ".dec.luau")
                with open(base + ".why.txt", "w", encoding="utf-8") as f:
                    f.write(why + "\n--- original output\n" + str(out.get(s)) +
                            "\n--- decompiled output\n" + str(out.get(d)) + "\n")
            for s, e in crashed:
                base = os.path.join(keep, os.path.basename(s)[:-5])
                shutil.copy(s, base + ".src.luau")
                with open(base + ".why.txt", "w", encoding="utf-8") as f:
                    f.write("decompiler crash: " + e + "\n")
            print(f"  failing cases written to {keep}")
        for s, d, why in failures[:8]:
            print(f"  {os.path.basename(s)}: {why}")
        return 0 if not failures and not crashed else 1
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
