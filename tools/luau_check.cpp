// luau_check.cpp — compile Luau source files and report syntax errors.
//
// Checks that tools/luau_decompile.py writes valid Luau: each file is run
// through the real Luau compiler. Prints "path: error" per failure and a
// summary; exit status 1 when any file fails.
//
//   luau_check FILE...
//   luau_check --emit DIR FILE...   also write each file's bytecode, compiled
//                                   the way Roblox compiles game scripts
//                                   (-O2), to DIR/<n>.bc, and print "n path"
//                                   (tools/luau_roundtrip.py compares them
//                                   with the original bytecode)
#include "Luau/Common.h"
#include "luacode.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iterator>
#include <string>

// Compiler changes Roblox's bytecode shows, which are off by default here.
static const char* const kFlags[] = {
    "LuauCompileMoveElision", "LuauCompileReuseLocalRegs", "LuauCompileConcatTargetTop",
    "LuauCompileLoopUnrollZero", "LuauCompileCleanBlockDeadClose",
};

// Globals a game may replace, so `script.Parent` must not become an import.
static const char* const kMutableGlobals[] = {
    "Game", "Workspace", "game", "plugin", "script", "shared", "workspace", nullptr,
};

int main(int argc, char** argv) {
    int first = 1;
    const char* emit = nullptr;
    if (argc > 2 && std::strcmp(argv[1], "--emit") == 0) {
        emit = argv[2];
        first = 3;
    }
    lua_CompileOptions opts = {};
    opts.optimizationLevel = emit ? 2 : 1;
    opts.debugLevel = 1;
    opts.vectorLib = "Vector3";
    opts.vectorCtor = "new";
    opts.vectorType = "Vector3";
    if (emit) {
        opts.mutableGlobals = kMutableGlobals;
        for (Luau::FValue<bool>* f = Luau::FValue<bool>::list; f; f = f->next)
            for (const char* name : kFlags)
                if (std::strcmp(f->name, name) == 0)
                    f->value = true;
    }
    int ok = 0, bad = 0;
    for (int i = first; i < argc; ++i) {
        std::ifstream f(argv[i], std::ios::binary);
        if (!f) {
            std::fprintf(stderr, "%s: cannot open\n", argv[i]);
            ++bad;
            continue;
        }
        const std::string src((std::istreambuf_iterator<char>(f)),
                              std::istreambuf_iterator<char>());
        size_t size = 0;
        char* bc = luau_compile(src.data(), src.size(), &opts, &size);
        // A failed compile returns version byte 0 followed by the message.
        if (size > 0 && bc[0] == 0) {
            std::printf("%s: %.*s\n", argv[i], int(size - 1), bc + 1);
            ++bad;
        } else {
            ++ok;
            if (emit) {
                const std::string out = std::string(emit) + "/" + std::to_string(i - first) + ".bc";
                std::ofstream o(out, std::ios::binary);
                o.write(bc, std::streamsize(size));
                std::printf("%d %s\n", i - first, argv[i]);
            }
        }
        std::free(bc);
    }
    std::printf("%d compile, %d fail\n", ok, bad);
    return bad ? 1 : 0;
}
