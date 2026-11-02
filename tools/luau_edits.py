#!/usr/bin/env python3
"""luau_edits.py - which functions differ, how, and what changed since a snapshot.

  luau_edits.py [--places a,b] [--top N] [--show KEY] [--count N]
                [--save NAME] [--since NAME] [--worse KEY]

Reads the decompiled scripts in $LUAU_RS/<place> (default out/<place>) against
the dumps in $LUAU_RUN/<place> (default run/<place>).

For every function that does not come back instruction for instruction, the
opcode diff reduced to one key: `+[LOADNIL]` means adding that is ALL that
separates the function from the original. --save stores the set of differing
functions; --since prints those that differ now and did not then, and how many
got fixed.
"""
import collections
import concurrent.futures as cf
import difflib
import os
import pickle
import shutil
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import luau_decompile as L  # noqa: E402
import luau_diverge as V  # noqa: E402
import luau_roundtrip as R  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ROOT = os.environ.get("LUAU_RUN", os.path.join(REPO, "run"))
HERE = os.path.join(REPO, "out")  # snapshots live with the output
RS = os.environ.get("LUAU_RS", os.path.join(REPO, "out"))
PLACES = ["scr_off", "scr_live", "scr_110443", "scr_75885"]


def edit_key(a, b):
    i = 0
    while i < len(a) and i < len(b) and a[i] == b[i]:
        i += 1
    j = 0
    while j < len(a) - i and j < len(b) - i and a[-1 - j] == b[-1 - j]:
        j += 1
    x, y = a[i:len(a) - j], b[i:len(b) - j]
    if len(x) * len(y) > 40000:
        return "(long)"
    words = []
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(a=x, b=y, autojunk=False).get_opcodes():
        if tag == "delete":
            words.append(f"-[{' '.join(x[i1:i2])}]")
        elif tag == "insert":
            words.append(f"+[{' '.join(y[j1:j2])}]")
        elif tag == "replace":
            words.append(f"[{' '.join(x[i1:i2])}]>[{' '.join(y[j1:j2])}]")
    return " ".join(sorted(set(words)))


def one_place(place):
    wire, lua = os.path.join(ROOT, place), os.path.join(RS, place)
    files = V.scripts(lua)
    tmp = tempfile.mkdtemp(prefix="luau_ed_")
    rows = []
    try:
        done = V.compile_all(sorted(files), tmp)
        for path, sid in sorted(files.items()):
            if path not in done:
                continue
            orig = L.parse(open(os.path.join(wire, sid + ".wire"), "rb").read())["protos"]
            back = L.parse(open(done[path], "rb").read(), scrambled=False)["protos"]
            paired = R.pair_protos(orig, back)
            for i, pa in enumerate(orig):
                if i not in paired:
                    rows.append((place, path, sid, i, pa.name, R.shape(pa), []))
                    continue
                sa, sb = R.shape(pa), R.shape(back[paired[i]])
                if sa != sb:
                    rows.append((place, path, sid, i, pa.name, sa, sb))
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    return rows


def main(argv):
    places = argv[argv.index("--places") + 1].split(",") if "--places" in argv else PLACES
    top = int(argv[argv.index("--top") + 1]) if "--top" in argv else 30
    show = argv[argv.index("--show") + 1] if "--show" in argv else None
    count = int(argv[argv.index("--count") + 1]) if "--count" in argv else 2
    with cf.ThreadPoolExecutor(4) as pool:
        rows = [r for rs in pool.map(one_place, places) for r in rs]
    print(len(rows), "functions differ")
    if "--since" in argv:
        name = argv[argv.index("--since") + 1]
        then = pickle.load(open(os.path.join(HERE, name + ".snap"), "rb"))
        now = {(r[0], r[2], r[3]) for r in rows}
        scope = {k for k in then if k[0] in places}
        worse = [r for r in rows if (r[0], r[2], r[3]) not in scope]
        print(len(scope - now), "fixed since", name, "and", len(worse), "newly different")
        wh = collections.Counter(edit_key(r[5], r[6])[:96] for r in worse)
        for k, n in wh.most_common(12):
            print(f"  worse {n:4d}  {k}")
        pick = argv[argv.index("--worse") + 1] if "--worse" in argv else None
        chosen = [r for r in worse if pick is None or edit_key(r[5], r[6])[:96] == pick[:96]]
        for place, path, sid, i, name_, sa, sb in sorted(chosen, key=lambda r: (r[4] is None, len(r[5])))[:count]:
            print(f"\n--- {os.path.relpath(path, ROOT)} script {sid} proto {i} name {name_}")
            print("  orig:", " ".join(sa[:50]))
            print("  back:", " ".join(sb[:50]))
            src = open(path, encoding="utf-8").read().splitlines()
            if name_:
                at = next((n for n, l in enumerate(src) if "function" in l and name_ in l), None)
                if at is not None:
                    ind = len(src[at]) - len(src[at].lstrip(" "))
                    for l in src[at:at + 16]:
                        print("    ", l[:100])
                        if l.rstrip() == " " * ind + "end":
                            break
    if "--save" in argv:
        name = argv[argv.index("--save") + 1]
        pickle.dump({(r[0], r[2], r[3]) for r in rows}, open(os.path.join(HERE, name + ".snap"), "wb"))
    hist = collections.Counter()
    egs = collections.defaultdict(list)
    for r in rows:
        key = edit_key(r[5], r[6])[:96]
        hist[key] += 1
        egs[key].append(r)
    if "--since" not in argv or top:
        print(len(hist), "kinds of edit; the commonest:")
        for k, n in hist.most_common(top):
            print(f"{n:5d}  {k}")
    if show:
        for place, path, sid, i, name_, sa, sb in sorted(egs[show[:96]], key=lambda e: (e[4] is None, len(e[5])))[:count]:
            print(f"\n--- {os.path.relpath(path, ROOT)} script {sid} proto {i} name {name_}")
            print("  orig:", " ".join(sa[:60]))
            print("  back:", " ".join(sb[:60]))
            src = open(path, encoding="utf-8").read().splitlines()
            if name_:
                at = next((n for n, l in enumerate(src) if "function" in l and name_ in l), None)
                if at is not None:
                    ind = len(src[at]) - len(src[at].lstrip(" "))
                    for l in src[at:at + 26]:
                        print("    ", l[:100])
                        if l.rstrip() == " " * ind + "end":
                            break
    return 0


sys.exit(main(sys.argv[1:]))
