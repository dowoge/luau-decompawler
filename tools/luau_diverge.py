#!/usr/bin/env python3
"""luau_diverge.py - where do decompiled functions stop matching the original?

Recompiles decompiled scripts and, for every function that does not come back
instruction for instruction, records the first place the two sequences part
ways. The histogram says which code shapes cost the most functions; --show
prints examples of one bucket, disassembled side by side.

  luau_diverge.py WIRE_DIR LUA_DIR [--top N] [--show "ORIG/BACK"] [--count N]
"""
import collections
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import luau_decompile as L  # noqa: E402
import luau_roundtrip as R  # noqa: E402

BIN = os.environ.get("LUAU_BIN", os.path.join(os.path.dirname(HERE), "build", "Release"))
CHECK = os.path.join(BIN, "luau_check.exe")


def first_diff(a, b):
    n = min(len(a), len(b))
    for i in range(n):
        if a[i] != b[i]:
            return i
    return n


def compile_all(paths, out):
    done = {}
    for k in range(0, len(paths), 100):
        sub = os.path.join(out, str(k))
        os.makedirs(sub)
        res = subprocess.run([CHECK, "--emit", sub] + paths[k:k + 100], capture_output=True, text=True)
        for line in res.stdout.splitlines():
            m = re.match(r"^(\d+) (.+)$", line)
            if m:
                done[m.group(2)] = os.path.join(sub, m.group(1) + ".bc")
    return done


def scripts(lua_dir):
    files = {}
    for dp, _, fns in os.walk(lua_dir):
        for fn in fns:
            if fn.endswith(".lua"):
                path = os.path.join(dp, fn)
                with open(path, encoding="utf-8") as f:
                    m = re.search(r"\(script (\d+)\)", f.readline())
                if m:
                    files[path] = m.group(1)
    return files


def main(argv):
    wire_dir, lua_dir = argv[0], argv[1]
    top = int(argv[argv.index("--top") + 1]) if "--top" in argv else 40
    show = argv[argv.index("--show") + 1] if "--show" in argv else None
    count = int(argv[argv.index("--count") + 1]) if "--count" in argv else 3
    files = scripts(lua_dir)
    tmp = tempfile.mkdtemp(prefix="luau_dv_")
    try:
        done = compile_all(sorted(files), tmp)
        hist = collections.Counter()
        lost = collections.Counter()
        examples = collections.defaultdict(list)
        total = same = 0
        for path, sid in sorted(files.items()):
            if path not in done:
                continue
            orig = L.parse(open(os.path.join(wire_dir, sid + ".wire"), "rb").read())["protos"]
            back = L.parse(open(done[path], "rb").read(), scrambled=False)["protos"]
            paired = R.pair_protos(orig, back)
            if len(orig) != len(back):
                hist["(function count differs)"] += 1
            for i, pa in enumerate(orig):
                total += 1
                if i not in paired:
                    hist["(no matching function)"] += 1
                    continue
                sa, sb = R.shape(pa), R.shape(back[paired[i]])
                if sa == sb:
                    same += 1
                    continue
                k = first_diff(sa, sb)
                x = sa[k] if k < len(sa) else "(end)"
                y = sb[k] if k < len(sb) else "(end)"
                key = f"{x}/{y}"
                hist[key] += 1
                lost[key] += len(sa)
                examples[key].append((path, i, pa.name, k, sa, sb))
        print(f"{total} functions, {same} identical ({same / max(total, 1):.1%})")
        for key, n in hist.most_common(top):
            print(f"{n:6d}  {key:36s} {lost[key]:8d} insns")
        if show:
            for path, i, name, k, sa, sb in sorted(examples[show], key=lambda e: len(e[4]))[:count]:
                print(f"\n--- {path} proto {i} ({name}) at {k}")
                print("  orig:", " ".join(sa[max(0, k - 6):k]), "|", " ".join(sa[k:k + 10]))
                print("  back:", " ".join(sb[max(0, k - 6):k]), "|", " ".join(sb[k:k + 10]))
                with open(path, encoding="utf-8") as f:
                    src = f.read().splitlines()
                at = next((n for n, l in enumerate(src)
                           if name and "function" in l and re.search(rf"\b{re.escape(name)}\b", l)), None)
                if at is None:
                    continue
                ind = len(src[at]) - len(src[at].lstrip(" "))
                for l in src[at:at + 30]:
                    print("   ", l[:130])
                    if l.rstrip() == " " * ind + "end":
                        break
        return 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
