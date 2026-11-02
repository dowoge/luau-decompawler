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
//   luau_check --stream             sources come as records on stdin (see
//                                   records.h) and are compiled as --emit
//                                   does; each result goes back as a record,
//                                   its bytecode or the compiler's message
//   -O0, -O1 or -O2                 with --emit or --stream: compile at that
//                                   level instead
//
// Files go through on every core, or LUAU_THREADS of them.
#include "Luau/Common.h"
#include "luacode.h"
#include "records.h"

#include <algorithm>
#include <atomic>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iterator>
#include <mutex>
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

static size_t worker_count(size_t files) {
    size_t n = std::max(std::thread::hardware_concurrency(), 1u);
    if (const char* v = std::getenv("LUAU_THREADS"))
        n = std::max(std::atoi(v), 1);
    return std::min(n, files);
}

int main(int argc, char** argv) {
    const char* emit = nullptr;
    bool stream = false;
    int level = 2;
    std::vector<Record> files;  // from the command line: names only, read below
    for (int i = 1; i < argc; ++i) {
        if (std::strcmp(argv[i], "--emit") == 0 && i + 1 < argc)
            emit = argv[++i];
        else if (std::strcmp(argv[i], "--stream") == 0)
            stream = true;
        else if (std::strcmp(argv[i], "-O0") == 0 || std::strcmp(argv[i], "-O1") == 0 ||
                 std::strcmp(argv[i], "-O2") == 0)
            level = argv[i][2] - '0';
        else
            files.push_back({argv[i], {}});
    }
    const size_t named = files.size();
    if (stream) {
        binary_stdio();
        for (Record& r : read_records())
            files.push_back(std::move(r));
    }
    lua_CompileOptions opts = {};
    opts.optimizationLevel = emit || stream ? level : 1;
    opts.debugLevel = 1;
    opts.vectorLib = "Vector3";
    opts.vectorCtor = "new";
    opts.vectorType = "Vector3";
    if (emit || stream) {
        opts.mutableGlobals = kMutableGlobals;
        for (Luau::FValue<bool>* f = Luau::FValue<bool>::list; f; f = f->next)
            for (const char* name : kFlags)
                if (std::strcmp(f->name, name) == 0)
                    f->value = true;
    }
    std::atomic<size_t> next{0};
    std::atomic<int> ok{0}, bad{0};
    std::mutex out;
    std::vector<std::string> said(files.size());  // printed in order, at the end
    const auto work = [&] {
        for (size_t i = next++; i < files.size(); i = next++) {
            Record& file = files[i];
            if (i < named) {
                std::ifstream f(file.name, std::ios::binary);
                if (!f) {
                    std::fprintf(stderr, "%s: cannot open\n", file.name.c_str());
                    ++bad;
                    continue;
                }
                file.data.assign(std::istreambuf_iterator<char>(f), std::istreambuf_iterator<char>());
            }
            size_t size = 0;
            char* bc = luau_compile(file.data.data(), file.data.size(), &opts, &size);
            // A failed compile returns version byte 0 followed by the message.
            const bool good = !(size > 0 && bc[0] == 0);
            ++(good ? ok : bad);
            if (stream && i >= named) {
                std::lock_guard<std::mutex> hold(out);
                write_result(i - named, good, good ? bc : bc + 1, good ? size : size - 1);
            } else if (!good) {
                said[i] = file.name + ": " + std::string(bc + 1, size - 1) + "\n";
            } else if (emit) {
                const std::string n = std::to_string(i);
                std::ofstream o(std::string(emit) + "/" + n + ".bc", std::ios::binary);
                o.write(bc, std::streamsize(size));
                said[i] = n + " " + file.name + "\n";
            }
            std::free(bc);
        }
    };
    size_t workers = worker_count(files.size());
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
