# luau-decompiler

Turns Roblox game-script bytecode (wire format v14) back into readable Luau,
as close as possible to what was written: `local` variables where the source
had them, `if` expressions and `and`/`or` chains, helpers the compiler inlined
at -O2 turned back into calls.

The decompiler is a Rust crate with no dependencies. `tools/` holds what tests it
against the real Luau compiler and VM, which come in as a submodule pinned to one
Luau commit.

## Build

    cargo build --release

The test harness also needs `luau_check` and `luau_run`, built from Luau:

    git submodule update --init
    cmake -S . -B build
    cmake --build build --config Release --target luau_check luau_run

They are looked for in `build/Release`; set `LUAU_BIN` if they are elsewhere.

## Use

    target/release/luau_decompile --dir DUMPS --out OUTPUT    # every .wire in a folder
    target/release/luau_decompile FILE.wire                   # one script to stdout
    target/release/luau_decompile --disasm FILE.wire          # bytecode with source lines

`--upstream` reads stock Luau bytecode instead of wire format. Environment:
`LUAU_NOFLAT` prints raw gotos, `LUAU_WHY` lists each flag with what was refused,
`LUAU_THREADS`, `LUAU_TIMING`.

## Check

    python tools/check.py          # build, cases, one random set
    python tools/check.py --full   # all 19 random sets

With `LUAU_RUN` set to a folder of script dumps (`<place>/<id>.wire`), it also
decompiles them into `out/` and reports flags, undeclared names and how many
functions recompile to the original. Dumps are never committed.

| Tool | What it answers |
| --- | --- |
| `tools/luau_cases.py` | Do the hand-written cases in `tools/luau_cases/` behave and recompile? `--show NAME` lists what differs. |
| `tools/luau_difftest.py` | Do random programs print the same after decompiling? `--keep DIR` saves failures. |
| `tools/luau_reduce.py FILE` | Shrinks a failing program. |
| `tools/luau_edits.py` | Which functions differ and by what edit; `--save NAME` then `--since NAME` shows exactly what a change did. |
| `tools/luau_diverge.py WIRE LUA` | Where functions first part from the original. |
| `tools/luau_undeclared.py DIR...` | Generated names used without a declaration: always a bug. |

`tools/luau_decompile.py` is the original Python port, frozen. The output has
moved on from it on purpose, but the harness still uses it to parse bytecode.
