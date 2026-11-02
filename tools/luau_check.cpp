// luau_check.cpp — compile Luau source files and report syntax errors.
//
// Checks that the decompiler writes valid Luau: each file is run through the
// real Luau compiler. Prints "path: error" per failure and a summary; exit
// status 1 when any file fails.
//
//   luau_check FILE...
//   luau_check --emit DIR FILE...   also write each file's bytecode, compiled
//                                   the way Roblox compiles game scripts
//                                   (-O2), to DIR/<n>.bc, and print "n path"
//                                   (the harness compares them with the
//                                   original bytecode)
//   --stdin                         the paths come from standard input too,
//                                   one per line
//
// Files go through on every core; what they report comes out in file order.
#include "Luau/Common.h"
#include "luacode.h"

#include <algorithm>
#include <atomic>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>
#include <thread>
#include <vector>

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
    const char* emit = nullptr;
    bool from_stdin = false;
    std::vector<std::string> paths;
    for (int i = 1; i < argc; ++i) {
        if (std::strcmp(argv[i], "--emit") == 0 && i + 1 < argc)
            emit = argv[++i];
        else if (std::strcmp(argv[i], "--stdin") == 0)
            from_stdin = true;
        else
            paths.push_back(argv[i]);
    }
    for (std::string line; from_stdin && std::getline(std::cin, line);) {
        if (!line.empty() && line.back() == '\r')
            line.pop_back();
        if (paths.empty() && line.rfind("\xEF\xBB\xBF", 0) == 0)
            line.erase(0, 3);  // a byte order mark, as PowerShell pipes
        if (!line.empty())
            paths.push_back(line);
    }
    const int count = int(paths.size());
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
    std::atomic<int> next{0}, ok{0}, bad{0};
    std::vector<std::string> said(count);
    const auto work = [&] {
        for (int i = next++; i < count; i = next++) {
            const std::string& path = paths[i];
            std::ifstream f(path, std::ios::binary);
            if (!f) {
                std::fprintf(stderr, "%s: cannot open\n", path.c_str());
                ++bad;
                continue;
            }
            const std::string src((std::istreambuf_iterator<char>(f)),
                                  std::istreambuf_iterator<char>());
            size_t size = 0;
            char* bc = luau_compile(src.data(), src.size(), &opts, &size);
            std::string& line = said[i];
            // A failed compile returns version byte 0 followed by the message.
            if (size > 0 && bc[0] == 0) {
                line = path + ": " + std::string(bc + 1, size - 1) + "\n";
                ++bad;
            } else {
                ++ok;
                if (emit) {
                    const std::string n = std::to_string(i);
                    std::ofstream o(std::string(emit) + "/" + n + ".bc", std::ios::binary);
                    o.write(bc, std::streamsize(size));
                    line = n + " " + path + "\n";
                }
            }
            std::free(bc);
        }
    };
    size_t workers = std::min<size_t>(std::max(std::thread::hardware_concurrency(), 1u), said.size());
    std::vector<std::thread> others;
    while (workers-- > 1)
        others.emplace_back(work);
    work();
    for (std::thread& t : others)
        t.join();
    for (const std::string& line : said)
        std::fputs(line.c_str(), stdout);
    std::printf("%d compile, %d fail\n", ok.load(), bad.load());
    return bad.load() ? 1 : 0;
}
