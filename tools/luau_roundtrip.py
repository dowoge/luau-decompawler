#!/usr/bin/env python3
"""luau_roundtrip.py - how faithful is the decompiled Luau?

Recompiles every decompiled script with the real Luau compiler at -O2 (the
level Roblox compiles game scripts at, which is why small local functions
appear inlined) and compares each function's instruction sequence with the
original bytecode. A function whose sequence comes back identical almost
certainly means what the original meant; the rest are listed worst first.

Instructions that do not change behaviour are normalized away first:
fast-call hints (FASTCALL*), Roblox-only variants of standard opcodes
(CALLFB, NAMECALLUDATA, ...), NOP/COVERAGE, JUMPX/LOADKX long forms.

  luau_roundtrip.py WIRE_DIR LUA_DIR [--check PATH_TO_luau_check.exe] [--worst N]
"""
import difflib
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import luau_decompile as L  # noqa: E402

DROP = {"NOP", "COVERAGE", "FASTCALL", "FASTCALL1", "FASTCALL2", "FASTCALL2K",
        "FASTCALL3", "NATIVECALL", "BREAK"}
SAME = {"CALLFB": "CALL", "NAMECALLUDATA": "NAMECALL", "GETUDATAKS": "GETTABLEKS",
        "SETUDATAKS": "SETTABLEKS", "JUMPX": "JUMP", "LOADKX": "LOADK",
        "JUMPBACK": "JUMP"}


def is_pcall(proto, aux):
    """Is this GETIMPORT of the global pcall or xpcall?"""
    if aux >> 30 != 1:
        return False
    kind, value = proto.k[(aux >> 20) & 1023]
    return kind == "string" and value in (b"pcall", b"xpcall", "pcall", "xpcall")


def shape(proto):
    # Roblox's compiler treats pcall as a fast builtin and loads it after its
    # arguments; the Luau we recompile with loads it before them. No source can
    # choose between those, so both are compared with the load at the call.
    out = []
    waiting = set()
    for pc, op, w, aux, n in proto.code:
        name = L.op_name(op)
        a = (w >> 8) & 255
        if name == "GETIMPORT" and is_pcall(proto, aux):
            waiting.add(a)
            continue
        if name in ("CALL", "CALLFB") and a in waiting:
            waiting.discard(a)
            out.append("GETIMPORT")
        if name != "FASTPCALL" and name not in DROP:
            out.append(SAME.get(name, name))
    return out + ["GETIMPORT"] * len(waiting)


def pair_protos(a, b):
    """{index in a: index in b}. Numbered in compile order, so one function
    gone missing would misalign all that follow: pair the same function by
    name and arity, then what is left by identical code, then by similarity."""
    paired, used = {}, set()

    def take(i, j):
        paired[i] = j
        used.add(j)

    sig_a = [(p.name, p.numparams) for p in a]
    sig_b = [(p.name, p.numparams) for p in b]
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, sig_a, sig_b, autojunk=False).get_opcodes():
        if tag == "equal":
            for i, j in zip(range(i1, i2), range(j1, j2)):
                take(i, j)
    by_sig = {}
    for j, s in enumerate(sig_b):
        if j not in used:
            by_sig.setdefault(s, []).append(j)
    for i, s in enumerate(sig_a):
        if i not in paired and by_sig.get(s):
            take(i, by_sig[s].pop(0))
    code_b = {}
    for j, p in enumerate(b):
        if j not in used:
            code_b.setdefault(tuple(shape(p)), []).append(j)
    for i, p in enumerate(a):
        if i not in paired and code_b.get(tuple(shape(p))):
            take(i, code_b[tuple(shape(p))].pop(0))
    # what is left: the most alike of the same arity
    shapes = {j: shape(p) for j, p in enumerate(b) if j not in used}
    for i, p in enumerate(a):
        if i in paired:
            continue
        sa = shape(p)
        best, score = None, 0.5
        for j, sb in shapes.items():
            if b[j].numparams != p.numparams or not 0.5 <= (len(sa) + 1) / (len(sb) + 1) <= 2:
                continue
            m = difflib.SequenceMatcher(None, sa, sb, autojunk=False)
            if m.real_quick_ratio() > score and m.quick_ratio() > score and m.ratio() > score:
                best, score = j, m.ratio()
        if best is not None:
            take(i, best)
            del shapes[best]
    return paired


def main(argv):
    wire_dir, lua_dir = argv[0], argv[1]
    check = argv[argv.index("--check") + 1] if "--check" in argv else \
        os.path.join(os.environ.get("LUAU_BIN", os.path.join(HERE, "..", "build", "Release")), "luau_check.exe")
    worst = int(argv[argv.index("--worst") + 1]) if "--worst" in argv else 10
    files = {}
    for dp, _, fns in os.walk(lua_dir):
        for fn in fns:
            if fn.endswith(".lua"):
                path = os.path.join(dp, fn)
                with open(path, encoding="utf-8") as f:
                    m = re.search(r"\(script (\d+)\)", f.readline())
                if m:
                    files[path] = m.group(1)
    paths = sorted(files)
    tmp = tempfile.mkdtemp(prefix="luau_rt_")
    compiled = {}
    # in batches: a place's paths overflow the Windows command line
    for k in range(0, len(paths), 100):
        sub = os.path.join(tmp, str(k))
        os.makedirs(sub)
        res = subprocess.run([check, "--emit", sub] + paths[k:k + 100],
                             capture_output=True, text=True)
        for line in res.stdout.splitlines():
            m = re.match(r"^(\d+) (.+)$", line)
            if m and m.group(2) in files:
                compiled[m.group(2)] = os.path.join(sub, m.group(1) + ".bc")
    funcs = same = 0
    insns = matched = 0
    scripts_exact = 0
    rows = []
    for path in paths:
        sid = files[path]
        orig = L.parse(open(os.path.join(wire_dir, sid + ".wire"), "rb").read())
        if path not in compiled:
            rows.append((0.0, path, "did not compile"))
            continue
        back = L.parse(open(compiled[path], "rb").read(), scrambled=False)
        a = [shape(p) for p in orig["protos"]]
        b = [shape(p) for p in back["protos"]]
        paired = pair_protos(orig["protos"], back["protos"])
        exact = True
        worst_fn = (1.0, None)
        for i, sa in enumerate(a):
            funcs += 1
            insns += len(sa)
            j = paired.get(i)
            if j is None:
                exact = False
                worst_fn = min(worst_fn, (0.0, orig["protos"][i].name or f"#{i}"))
                continue
            sb = b[j]
            if sa == sb:
                same += 1
                matched += len(sa)
                continue
            exact = False
            r = difflib.SequenceMatcher(None, sa, sb, autojunk=False)
            matched += sum(blk.size for blk in r.get_matching_blocks())
            worst_fn = min(worst_fn, (r.ratio(), orig["protos"][i].name or f"#{i}"))
        scripts_exact += exact
        if not exact:
            rows.append((worst_fn[0], path, f"worst function {worst_fn[1]} ({worst_fn[0]:.0%} alike)"))
    shutil.rmtree(tmp, ignore_errors=True)
    print(f"{len(paths)} scripts: {scripts_exact} come back instruction-for-instruction identical")
    print(f"{funcs} functions: {same} identical ({same / max(funcs, 1):.1%})")
    print(f"{insns} instructions: {matched} in matching order ({matched / max(insns, 1):.1%})")
    rows.sort()
    for score, path, why in rows[:worst]:
        print(f"  {path}: {why}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
