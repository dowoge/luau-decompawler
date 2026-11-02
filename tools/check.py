#!/usr/bin/env python3
"""check.py - everything that has to hold before a change goes in.

  check.py [--full]

Builds the decompiler, runs the hand-written cases and the random differential
tests (one seed set, or all 19 with --full). With LUAU_RUN pointing at a folder
of script dumps (<place>/<id>.wire) it also decompiles those into out/ and
reports the jump flags, dispatchers, undeclared names and how many functions
recompile to the original. Exit status 1 if anything is wrong.

Failing random programs are kept in failed/.
"""
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
EXE = os.path.join(REPO, "target", "release", "luau_decompile.exe")
PY = [sys.executable, "-I"]
QUICK = [(300, 12, 700000)]
FULL = [(300, 3, 5000), (300, 18, 20000), (300, 12, 700000), (200, 25, 1), (300, 15, 900000), (300, 8, 1300000),
        (300, 10, 2000000), (300, 20, 3000000), (300, 14, 4000000), (300, 16, 5000000), (300, 22, 6000000),
        (300, 30, 7000000), (300, 6, 8000000), (300, 11, 9000000), (300, 13, 10000000), (300, 24, 11000000),
        (300, 9, 12000000), (300, 17, 13000000), (300, 7, 14000000)]
CLEAN = " 0 differ, 0 decompiled programs do not compile, 0 decompiler crashes"


def run(cmd, **kw):
    return subprocess.run(cmd, cwd=REPO, capture_output=True, text=True, errors="replace", **kw)


def tool(name, *args):
    return run(PY + [os.path.join(HERE, name), *map(str, args)])


def build():
    r = run(["cargo", "build", "--release"])
    if r.returncode:
        print(r.stderr[-3000:])
    return r.returncode == 0


def cases():
    r = tool("luau_cases.py")
    for line in r.stdout.splitlines():
        if "BAD" in line or line.startswith("total"):
            print(line)
    return r.returncode == 0


def random_sets(seeds):
    bad = 0
    for count, size, seed in seeds:
        r = tool("luau_difftest.py", "--count", count, "--size", size, "--seed", seed,
                 "--keep", os.path.join(REPO, "failed"))
        summary = [l for l in r.stdout.splitlines() if "programs (" in l]
        if not summary or CLEAN not in summary[0]:
            bad += 1
            print(f"seed {seed}:", summary[0] if summary else r.stderr[-500:])
    print(f"random sets clean: {len(seeds) - bad} of {len(seeds)}")
    return bad == 0


def real_scripts(dumps):
    good = True
    out = os.path.join(REPO, "out")
    places = sorted(p for p in os.listdir(dumps) if not p.endswith("_lua") and os.path.isdir(os.path.join(dumps, p))
                    and any(f.endswith(".wire") for f in os.listdir(os.path.join(dumps, p))))
    for place in places:
        wire, lua = os.path.join(dumps, place), os.path.join(out, place)
        r = run([EXE, "--dir", wire, "--out", lua])
        print((r.stdout + r.stderr).strip().splitlines()[-1])
        print(" ", tool("luau_diverge.py", wire, lua, "--top", 0).stdout.splitlines()[0])
        if re.search(r"[1-9]\d* (failed|dispatchers)", r.stdout + r.stderr):
            good = False
    r = tool("luau_undeclared.py", *(os.path.join(out, p) for p in places))
    print(r.stdout.strip())
    return good and r.stdout.startswith("0 undeclared")


def main(argv):
    ok = build()
    ok = ok and cases()
    ok = random_sets(FULL if "--full" in argv else QUICK) and ok
    dumps = os.environ.get("LUAU_RUN")
    if dumps:
        ok = real_scripts(dumps) and ok
    print("all good" if ok else "FAILED")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
