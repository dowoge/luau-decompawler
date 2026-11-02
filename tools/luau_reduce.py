#!/usr/bin/env python3
"""luau_reduce.py - shrink a Luau program that the decompiler gets wrong.

Removes lines and whole blocks while the program stays valid and the failure
stays the same kind, printing what is left. "Wrong" means the decompiled
program does not compile, or prints something else than the original; with
--shape it means a function that does not recompile to the original sequence.

  luau_reduce.py FILE [--shape] [--out FILE] [--seconds N]

Works from a private copy of the decompiler, so rebuilding does not disturb a
run, and stops after N seconds with the smallest program so far.
"""
import concurrent.futures as cf
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import luau_decompile as L  # noqa: E402
import luau_difftest as D  # noqa: E402
import luau_roundtrip as R  # noqa: E402

CHECK = os.path.join(D.BIN, "luau_check.exe")
RUNNER = os.path.join(D.BIN, "luau_run.exe")


DECOMPILER = D.DECOMPILER


def compiled(path, out):
    return D.emit(CHECK, [path], out).get(path)


def kind_of(lines, tmp, name, shape):
    """What goes wrong with this program, None when nothing does."""
    d = os.path.join(tmp, name)
    os.makedirs(d, exist_ok=True)
    src = os.path.join(d, "s.luau")
    with open(src, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")
    bc = compiled(src, os.path.join(d, "a"))
    if bc is None:
        return None
    text, why = D.decompile_binary([DECOMPILER], bc)
    if why is not None:
        return "crash"
    dec = os.path.join(d, "d.luau")
    with open(dec, "w", encoding="utf-8") as f:
        f.write(text)
    bd = compiled(dec, os.path.join(d, "b"))
    if bd is None:
        return "no-compile"
    if shape:
        a = L.parse(open(bc, "rb").read(), scrambled=False)["protos"]
        b = L.parse(open(bd, "rb").read(), scrambled=False)["protos"]
        for i, pa in enumerate(a):
            if i >= len(b) or R.shape(pa) != R.shape(b[i]):
                return "shape"
        return None
    try:
        # the VM budget counts calls and loops: one huge string.rep is neither
        text = subprocess.run([RUNNER, src, dec], capture_output=True, text=True, timeout=10).stdout
    except subprocess.TimeoutExpired:
        return None
    out = D.outputs(text)
    x, y = out.get(src), out.get(dec)
    # a program that now fails on its own is a different problem: without
    # this the search drifts to undefined names and error message wording
    if x is None or not x.endswith("=== ok"):
        return None
    return None if x == y else "differs"


def block_end(lines, i):
    """Line after the block opened at i, or i + 1 when it opens none."""
    open_re = re.compile(r"\b(function|if|for|while|repeat|do)\b")
    if not open_re.search(lines[i]):
        return i + 1
    ind = len(lines[i]) - len(lines[i].lstrip(" "))
    for j in range(i + 1, len(lines)):
        s = lines[j].strip()
        if s and len(lines[j]) - len(lines[j].lstrip(" ")) <= ind:
            if re.match(r"(end\b|until\b)", s):
                return j + 1
            if not re.match(r"(else|elseif)\b", s):
                return j
    return i + 1


def parens(line):
    """(start, end) of every parenthesized group, outermost first."""
    stack, found, quote = [], [], None
    for i, c in enumerate(line):
        if quote:
            quote = None if c == quote and line[i - 1] != "\\" else quote
        elif c in "\"'`":
            quote = c
        elif c == "(":
            stack.append(i)
        elif c == ")" and stack:
            found.append((stack.pop(), i))
    return sorted(found, key=lambda se: se[0] - se[1])


def expression_cuts(lines):
    for i, l in enumerate(lines):
        if len(l) < 30:
            continue
        for s, e in parens(l):
            for what in ("1", "true", '"a"', l[s + 1:e]):
                # a call keeps its parentheses, its arguments get simpler
                call = s > 0 and (l[s - 1].isalnum() or l[s - 1] in "_")
                new = l[:s + 1] + what + l[e:] if call else l[:s] + what + l[e + 1:]
                if new != l:
                    yield lines[:i] + [new] + lines[i + 1:]


def candidates(lines):
    """Smaller programs, biggest cuts first."""
    n = len(lines)
    size = n // 2
    while size >= 2:
        for i in range(0, n - size + 1, max(size // 2, 1)):
            yield lines[:i] + lines[i + size:]
        size //= 2
    for i in sorted(range(n), key=lambda k: block_end(lines, k) - k, reverse=True):
        e = block_end(lines, i)
        if e - i > 1:
            yield lines[:i] + lines[e:]
    for i in range(n):
        yield lines[:i] + lines[i + 1:]
    # keep a block's body, drop its header and end
    for i in range(n):
        e = block_end(lines, i)
        if e - i > 2:
            ind = len(lines[i]) - len(lines[i].lstrip(" "))
            body = [l[4:] if len(l) - len(l.lstrip(" ")) > ind else l for l in lines[i + 1:e - 1]]
            yield lines[:i] + body + lines[e:]
    yield from expression_cuts(lines)


def main(argv):
    path = argv[0]
    shape = "--shape" in argv
    target = argv[argv.index("--out") + 1] if "--out" in argv else None
    with open(path, encoding="utf-8") as f:
        lines = f.read().splitlines()
    seconds = float(argv[argv.index("--seconds") + 1]) if "--seconds" in argv else 600
    tmp = tempfile.mkdtemp(prefix="luau_red_")
    global DECOMPILER
    try:
        DECOMPILER = shutil.copy(D.DECOMPILER, os.path.join(tmp, "decompiler.exe"))
        stop = time.time() + seconds
        want = kind_of(lines, tmp, "start", shape)
        if want is None:
            print("nothing wrong with this program")
            return 1
        workers = os.cpu_count() or 4
        pool = cf.ThreadPoolExecutor(workers)
        serial = 0
        progress = True
        while progress and time.time() < stop:
            progress = False
            cands = list(candidates(lines))
            for k in range(0, len(cands), workers):
                if time.time() >= stop:
                    break
                batch = cands[k:k + workers]
                names = [f"c{serial + i}" for i in range(len(batch))]
                serial += len(batch)
                got = list(pool.map(lambda c: kind_of(c[0], tmp, c[1], shape), zip(batch, names)))
                for name in names:
                    shutil.rmtree(os.path.join(tmp, name), ignore_errors=True)
                hit = next((c for c, g in zip(batch, got) if g == want), None)
                if hit is not None:
                    lines = hit
                    progress = True
                    break
        text = "\n".join(lines) + "\n"
        print(f"-- {want}, {len(lines)} lines")
        print(text, end="")
        if target:
            with open(target, "w", encoding="utf-8") as f:
                f.write(text)
        return 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
