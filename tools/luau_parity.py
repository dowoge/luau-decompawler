#!/usr/bin/env python3
"""luau_parity.py - is the Rust decompiler's output what the reference wrote?

  luau_parity.py GOLDEN_DIR NEW_DIR
      Compare two output trees (CRLF counts as LF: Python on Windows wrote
      CRLF, the Rust tool writes LF). Exit 0 only if they match exactly.

  luau_parity.py --corpus CORPUS_DIR BIN
      Run BIN --upstream on every program of a frozen corpus, in parallel,
      and compare with the recorded decompile and disassembly.
      A BIN ending in .py runs under this Python (to test the tooling).

  luau_parity.py --build-corpus CORPUS_DIR
      Build a corpus: generated programs (tools/luau_difftest.py Gen),
      compiled at -O2, with the reference decompiler's output recorded.

Corpus files, per program: NAME.luau source, NAME.bc bytecode, NAME.lua the
reference's decompile (LF), and for a sample NAME.disasm; NAME.error instead
of NAME.lua if the reference raised. manifest.json holds the settings.
"""
import concurrent.futures as cf
import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
REFERENCE = os.path.join(ROOT, "run", "golden", "luau_decompile.reference.py")
CHECK = os.path.join(os.environ.get("LUAU_BIN", os.path.join(ROOT, "build", "Release")), "luau_check.exe")
SHOW = 5        # differences printed in full
CONTEXT = 2     # lines of context around a difference
TIMEOUT = 300   # seconds per program, so a hang is a failure, not a stall
# (size, first seed, programs): the seeds earlier difftest runs used
SETS = [(3, 5000, 150), (12, 9000, 150), (18, 20000, 150), (25, 1, 150)]
DISASM_EVERY = 10


def norm(data):
    return data.replace(b"\r\n", b"\n")


def first_difference(old, new):
    """(line number, reference context, new context) of the first difference."""
    a = old.decode("utf-8", "replace").split("\n")
    b = new.decode("utf-8", "replace").split("\n")
    i = 0
    while i < len(a) and i < len(b) and a[i] == b[i]:
        i += 1
    lo = max(0, i - CONTEXT)
    return i + 1, a[lo:i + CONTEXT + 1], b[lo:i + CONTEXT + 1], lo + 1


def compare_files(pairs):
    """Worker: [(rel, old path, new path)] -> [(rel, difference)]."""
    out = []
    for rel, p, q in pairs:
        with open(p, "rb") as f:
            old = norm(f.read())
        with open(q, "rb") as f:
            new = norm(f.read())
        if old != new:
            out.append((rel, first_difference(old, new)))
    return out


def show(rel, diff):
    line, a, b, first = diff
    print(f"  {rel}: first difference at line {line}")
    for tag, lines in (("reference", a), ("new      ", b)):
        for k, text in enumerate(lines):
            mark = ">" if first + k == line else " "
            print(f"    {tag} {mark} {first + k}: {text[:160]}")


def tree(root):
    found = {}
    for dp, _, fns in os.walk(root):
        for fn in fns:
            p = os.path.join(dp, fn)
            found[os.path.relpath(p, root).replace("\\", "/")] = p
    return found


def compare_trees(golden, new):
    t0 = time.time()
    if not os.path.isdir(golden) or not os.path.isdir(new):
        print(f"not a directory: {golden if not os.path.isdir(golden) else new}")
        return 2
    a, b = tree(golden), tree(new)
    common = sorted(set(a) & set(b))
    missing = sorted(set(a) - set(b))
    extra = sorted(set(b) - set(a))
    pairs = [(rel, a[rel], b[rel]) for rel in common]
    chunks = [pairs[i:i + 64] for i in range(0, len(pairs), 64)]
    differ = []
    if len(pairs) < 200:
        differ = compare_files(pairs)
    else:
        with cf.ProcessPoolExecutor(min(8, os.cpu_count() or 1)) as ex:
            for got in ex.map(compare_files, chunks):
                differ.extend(got)
    differ.sort()
    print(f"{len(a)} reference files: {len(common) - len(differ)} identical, {len(differ)} differ, "
          f"{len(missing)} missing, {len(extra)} extra ({time.time() - t0:.1f}s)")
    for rel, diff in differ[:SHOW]:
        show(rel, diff)
    if len(differ) > SHOW:
        print(f"  ... and {len(differ) - SHOW} more")
    for name, items in (("missing", missing), ("extra", extra)):
        for rel in items[:SHOW]:
            print(f"  {name}: {rel}")
        if len(items) > SHOW:
            print(f"  ... and {len(items) - SHOW} more {name}")
    return 0 if not differ and not missing and not extra else 1


def command(binary):
    if binary.endswith(".py"):
        return [sys.executable, "-I", binary]
    return [binary]


def run_one(cmd, args, expected):
    """Run one comparison; None when it matches, else a reason or difference."""
    try:
        p = subprocess.run(cmd + args, capture_output=True, timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        return f"timed out after {TIMEOUT}s"
    if p.returncode != 0:
        tail = p.stderr.decode("utf-8", "replace").strip().splitlines()[-1:]
        return f"exit status {p.returncode}: {tail[0] if tail else ''}"
    new = norm(p.stdout)
    if new == expected:
        return None
    return first_difference(expected, new)


def check_corpus(root, binary):
    t0 = time.time()
    root = os.path.abspath(root)
    if not binary.endswith(".py") and not os.path.isfile(binary):
        print(f"no such binary: {binary}")
        return 2
    cmd = command(binary)
    names = sorted(f[:-3] for f in os.listdir(root) if f.endswith(".bc"))
    if not names:
        print(f"no programs in {root}")
        return 2
    jobs = []
    skipped = 0
    for name in names:
        bc = os.path.join(root, name + ".bc")
        lua = os.path.join(root, name + ".lua")
        if os.path.exists(lua):
            with open(lua, "rb") as f:
                jobs.append((name + ".lua", ["--upstream", bc], f.read()))
        else:
            skipped += 1  # the reference raised: nothing to match
        dis = os.path.join(root, name + ".disasm")
        if os.path.exists(dis):
            with open(dis, "rb") as f:
                jobs.append((name + ".disasm", ["--upstream", "--disasm", bc], f.read()))
    with cf.ThreadPoolExecutor(os.cpu_count() or 4) as ex:
        results = list(ex.map(lambda j: (j[0], run_one(cmd, j[1], j[2])), jobs))
    bad = [(n, r) for n, r in results if r is not None]
    lua_jobs = sum(1 for j in jobs if j[0].endswith(".lua"))
    print(f"{len(names)} programs ({skipped} the reference failed on): "
          f"{lua_jobs} decompiles and {len(jobs) - lua_jobs} disassemblies checked, "
          f"{len(jobs) - len(bad)} identical, {len(bad)} differ ({time.time() - t0:.1f}s)")
    for name, why in bad[:SHOW]:
        if isinstance(why, str):
            print(f"  {name}: {why}")
        else:
            show(name, why)
    if len(bad) > SHOW:
        print(f"  ... and {len(bad) - SHOW} more")
    return 0 if not bad else 1


def load_reference():
    spec = importlib.util.spec_from_file_location("luau_reference", REFERENCE)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def record(job):
    """Worker: write the reference's output for one program."""
    base, want_disasm = job
    ref = record.ref
    with open(base + ".bc", "rb") as f:
        data = f.read()
    try:
        text = ref.decompile(ref.parse(data, scrambled=False))
    except Exception as e:  # the Rust tool is not held to this one
        with open(base + ".error", "w", encoding="utf-8", newline="\n") as f:
            f.write(repr(e) + "\n")
        return 1
    with open(base + ".lua", "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    if want_disasm:
        with open(base + ".disasm", "w", encoding="utf-8", newline="\n") as f:
            f.write(ref.disassemble(ref.parse(data, scrambled=False)))
    return 0


def record_init():
    record.ref = load_reference()


def build_corpus(root):
    t0 = time.time()
    root = os.path.abspath(root)
    sys.path.insert(0, HERE)
    import luau_difftest as D
    if os.path.exists(root):
        shutil.rmtree(root)
    os.makedirs(root)
    srcs = []
    for size, seed, count in SETS:
        for k in range(count):
            p = os.path.join(root, f"s{size}_g{seed + k}.luau")
            with open(p, "w", encoding="utf-8", newline="\n") as f:
                f.write(D.Gen(seed + k, size).program())
            srcs.append(p)
    tmp = tempfile.mkdtemp(prefix="luau_corpus_")
    try:
        mapping = D.emit(CHECK, srcs, os.path.join(tmp, "bc"))
        for s, bc in mapping.items():
            shutil.move(bc, s[:-5] + ".bc")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    uncompiled = [s for s in srcs if s not in mapping]
    jobs = [(s[:-5], k % DISASM_EVERY == 0) for k, s in enumerate(s for s in srcs if s in mapping)]
    with cf.ProcessPoolExecutor(min(12, os.cpu_count() or 1), initializer=record_init) as ex:
        failed = sum(ex.map(record, jobs, chunksize=4))
    with open(REFERENCE, "rb") as f:
        digest = hashlib.sha256(f.read()).hexdigest()
    manifest = {"reference_sha256": digest, "sets": SETS, "disasm_every": DISASM_EVERY,
                "programs": len(jobs), "reference_failures": failed,
                "did_not_compile": [os.path.basename(s) for s in uncompiled]}
    with open(os.path.join(root, "manifest.json"), "w", encoding="utf-8", newline="\n") as f:
        json.dump(manifest, f, indent=1)
        f.write("\n")
    print(f"{len(jobs)} programs in {root}: {failed} reference failures, "
          f"{len(uncompiled)} did not compile ({time.time() - t0:.1f}s)")
    return 0 if not uncompiled else 1


def main(argv):
    if len(argv) == 2 and argv[0] == "--build-corpus":
        return build_corpus(argv[1])
    if len(argv) == 3 and argv[0] == "--corpus":
        return check_corpus(argv[1], argv[2])
    if len(argv) == 2 and not argv[0].startswith("--"):
        return compare_trees(argv[0], argv[1])
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
