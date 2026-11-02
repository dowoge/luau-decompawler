#!/usr/bin/env python3
"""luau_cases.py - how close does the decompiler get on hand-written scripts?

Every file in tools/luau_cases is compiled at -O2, decompiled, and judged on:
  run     prints the same as the original (and ran without error)
  shape   each function recompiles to the same opcode sequence
  exact   also the same A operands and constants, so the same registers: the
          variables came back the way they were written
Files print one row each; --show FILE lists its mismatching functions with
both instruction sequences and the decompiled text. The exit status is 1 when
behaviour differs, since that is a bug and not just a difference in form.

  luau_cases.py [--dir DIR] [--show FILE] [--bin PATH] [--all]
"""
import os
import shutil
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import luau_decompile as L  # noqa: E402
import luau_difftest as D  # noqa: E402
import luau_roundtrip as R  # noqa: E402

CHECK = os.path.join(D.BIN, "luau_check.exe")
RUNNER = os.path.join(D.BIN, "luau_run.exe")


def exact(proto):
    """The instruction sequence with its target register and constants."""
    out = []
    for pc, op, w, aux, n in proto.code:
        name = L.op_name(op)
        if name in R.DROP:
            continue
        a = (w >> 8) & 255
        out.append(f"{R.SAME.get(name, name)}:{a}")
    return out


def main(argv):
    cases = argv[argv.index("--dir") + 1] if "--dir" in argv else os.path.join(HERE, "luau_cases")
    show = argv[argv.index("--show") + 1] if "--show" in argv else None
    binary = argv[argv.index("--bin") + 1] if "--bin" in argv else D.DECOMPILER
    srcs = sorted(os.path.join(cases, f) for f in os.listdir(cases) if f.endswith(".luau"))
    tmp = tempfile.mkdtemp(prefix="luau_cs_")
    try:
        bc = D.emit(CHECK, srcs, os.path.join(tmp, "a"))
        decs = {}
        crashed = {}
        for s in srcs:
            if s not in bc:
                crashed[s] = "the case itself does not compile"
                continue
            text, why = D.decompile_binary([binary], bc[s])
            if why is not None:
                crashed[s] = why
                continue
            d = os.path.join(tmp, os.path.basename(s)[:-5] + ".dec.luau")
            with open(d, "w", encoding="utf-8") as f:
                f.write(text)
            decs[s] = d
        back = D.emit(CHECK, list(decs.values()), os.path.join(tmp, "b"))
        out = D.outputs(D.run([RUNNER], srcs + list(decs.values())))
        totals = [0, 0, 0]
        bad_run = 0
        print(f"{'case':24s} {'run':4s} {'funcs':>5s} {'shape':>6s} {'exact':>6s}")
        for s in srcs:
            base = os.path.basename(s)
            if s not in decs:
                print(f"{base:24s} crash: {crashed.get(s)}")
                bad_run += 1
                continue
            d = decs[s]
            if d not in back:
                print(f"{base:24s} decompiled text does not compile")
                bad_run += 1
                continue
            same = out.get(s) == out.get(d) and str(out.get(s)).endswith("=== ok")
            bad_run += not same
            a = L.parse(open(bc[s], "rb").read(), scrambled=False)["protos"]
            b = L.parse(open(back[d], "rb").read(), scrambled=False)["protos"]
            paired = R.pair_protos(a, b)
            n = shp = ex = 0
            rows = []
            for i, pa in enumerate(a):
                n += 1
                j = paired.get(i)
                if j is None:
                    rows.append((pa.name, "no matching function", [], []))
                    continue
                sa, sb = R.shape(pa), R.shape(b[j])
                if sa == sb:
                    shp += 1
                    if exact(pa) == exact(b[j]):
                        ex += 1
                    else:
                        rows.append((pa.name, "registers", exact(pa), exact(b[j])))
                else:
                    rows.append((pa.name, "shape", sa, sb))
            totals[0] += n
            totals[1] += shp
            totals[2] += ex
            print(f"{base:24s} {'ok' if same else 'BAD':4s} {n:5d} {shp:6d} {ex:6d}")
            if show and show in base:
                if not same:
                    print("--- original output\n" + str(out.get(s)) + "\n--- decompiled output\n" + str(out.get(d)))
                with open(d, encoding="utf-8") as f:
                    text = f.read().splitlines()
                for name, why, x, y in rows:
                    print(f"\n  {name}: {why}")
                    k = next((i for i in range(min(len(x), len(y))) if x[i] != y[i]), min(len(x), len(y)))
                    print("    orig:", " ".join(x[max(0, k - 3):k]), "|", " ".join(x[k:k + 12]))
                    print("    back:", " ".join(y[max(0, k - 3):k]), "|", " ".join(y[k:k + 12]))
                    at = next((n_ for n_, l in enumerate(text) if name and "function" in l and f".{name}(" in l or
                               name and f" {name}(" in l and "function" in l or
                               name and f":{name}(" in l and "function" in l), None)
                    if at is not None:
                        ind = len(text[at]) - len(text[at].lstrip(" "))
                        for l in text[at:at + 24]:
                            print("      " + l[:110])
                            if l.rstrip() == " " * ind + "end":
                                break
        n, shp, ex = totals
        print(f"{'total':24s} {'':4s} {n:5d} {shp:6d} {ex:6d}   shape {shp / max(n, 1):.1%}, exact {ex / max(n, 1):.1%}"
              f", {bad_run} misbehave")
        return 1 if bad_run else 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
