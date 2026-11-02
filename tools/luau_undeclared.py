"""Find generated variable names (vN_d, argN_d, vN) used in a decompiled file
without any declaration (local / for / parameter): they would be globals."""
import os
import re
import sys

GEN = re.compile(r"(?<![A-Za-z0-9_.:\"])(v\d+(?:x\d+)?(?:_\d+)*|arg\d+_\d+)(?![A-Za-z0-9_])")
total = 0
files = 0
examples = []
for root in sys.argv[1:]:
    for dp, _, fns in os.walk(root):
        for fn in fns:
            if not fn.endswith(".lua"):
                continue
            path = os.path.join(dp, fn)
            text = open(path, encoding="utf-8").read()
            text_nostr = re.sub(r'"(?:[^"\\\n]|\\.)*"', '""', text)
            declared = set()
            for m in re.finditer(r"\blocal (?:function )?([A-Za-z0-9_, ]+?)(?: =|\(|$)", text_nostr, re.M):
                declared |= {x.strip() for x in m.group(1).split(",")}
            for m in re.finditer(r"\bfor ([A-Za-z0-9_, ]+?) (?:in|=) ", text_nostr):
                declared |= {x.strip() for x in m.group(1).split(",")}
            for m in re.finditer(r"function[ A-Za-z0-9_.:]*\(([^)]*)\)", text_nostr):
                declared |= {x.strip() for x in m.group(1).split(",")}
            bad = {m.group(1) for m in GEN.finditer(text_nostr)} - declared
            if bad:
                files += 1
                total += len(bad)
                if len(examples) < 8:
                    examples.append((path, sorted(bad)[:5]))
print(f"{total} undeclared generated names in {files} files")
for p, b in examples:
    print(" ", p, b)
