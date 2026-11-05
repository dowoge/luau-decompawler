// luau_run.cpp — run Luau source files and print what they print.
//
// For the decompiler's differential tests: a generated program and its
// decompiled version must print the same thing. Each file runs in a fresh
// VM (standard libraries, sandboxed), compiled at -O2 like Roblox game
// scripts unless -O0 or -O1 says otherwise, with an instruction budget so a
// broken program cannot hang.
//
//   luau_run [-O0|-O1|-O2] [--budget N] [--seconds S] [--stream] [--bytecode] FILE...
// prints, per file:  "=== FILE" then its output, then "=== ok" or
// "=== error: MESSAGE" (chunk names and line numbers stripped, since they
// differ between a program and its decompiled form). With --stream, programs
// come as records on stdin too (see records.h), each called by its name.
// With --bytecode, each file is bytecode, loaded without compiling.
//
// Files run on every core, and each one's lines come out together when it
// ends, in no particular order. The budget counts calls and loop passes, so a
// program stuck inside one library call (a pattern that backtracks without
// end) gets past it: with --seconds, one still running after S seconds is left
// behind. It prints nothing, and the others go on without it.
#include "Luau/Common.h"
#include "lua.h"
#include "lualib.h"
#include "luacode.h"
#include "records.h"

#include <algorithm>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iterator>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

// The file a thread is running: its budget, and where its print goes.
static thread_local long g_budget = 0;
static thread_local std::string* g_out = nullptr;

static void interrupt(lua_State* L, int gc) {
    if (gc >= 0)
        return;
    if (--g_budget < 0) {
        g_budget = 1 << 20;  // room for the error itself
        lua_rawcheckstack(L, 1);
        luaL_error(L, "budget exceeded");
    }
}

static int lprint(lua_State* L) {
    int n = lua_gettop(L);
    std::string& out = *g_out;
    for (int i = 1; i <= n; ++i) {
        size_t len = 0;
        const char* s = luaL_tolstring(L, i, &len);
        if (i > 1)
            out += '\t';
        out.append(s, len);
        lua_pop(L, 1);
    }
    out += '\n';
    return 0;
}

static std::string strip_location(std::string msg) {
    // "chunk:12: message" -> "message"
    size_t a = msg.find(':');
    if (a != std::string::npos) {
        size_t b = msg.find(':', a + 1);
        if (b != std::string::npos && b > a + 1 &&
            msg.find_first_not_of("0123456789", a + 1) == b)
            msg = msg.substr(b + 1);
    }
    while (!msg.empty() && msg[0] == ' ')
        msg.erase(0, 1);
    return msg;
}

// Everything one program prints, its result line last. A program from the
// command line is read here; a record brings its own.
static std::string run_file(Record& file, bool named, bool bytecode, long budget, lua_CompileOptions opts) {
    std::string out = "=== " + file.name + "\n";
    if (named) {
        std::ifstream f(file.name, std::ios::binary);
        if (!f)
            return out + "=== error: cannot open\n";
        file.data.assign(std::istreambuf_iterator<char>(f), std::istreambuf_iterator<char>());
    }
    const std::string& src = file.data;
    size_t size = 0;
    char* bc = nullptr;
    if (bytecode) {
        // loaded as it is: bytecode no compiler wrote, for the decompiler to read
        size = src.size();
        bc = static_cast<char*>(std::malloc(size ? size : 1));
        std::memcpy(bc, src.data(), size);
    } else {
        bc = luau_compile(src.data(), src.size(), &opts, &size);
    }
    lua_State* L = luaL_newstate();
    luaL_openlibs(L);
    lua_pushcfunction(L, lprint, "print");
    lua_setglobal(L, "print");
    // Roblox's Vector3.new, which decompiled code spells vector constants
    // with: here a plain vector.create
    lua_getglobal(L, "vector");
    lua_getfield(L, -1, "create");
    lua_createtable(L, 0, 1);
    lua_insert(L, -2);
    lua_setfield(L, -2, "new");
    lua_setglobal(L, "Vector3");
    lua_pop(L, 1);
    luaL_sandbox(L);
    luaL_sandboxthread(L);
    lua_callbacks(L)->interrupt = interrupt;
    g_budget = budget;
    g_out = &out;
    int rc = luau_load(L, "=prog", bc, size, 0);
    std::free(bc);
    if (rc == 0)
        rc = lua_pcall(L, 0, 0, 0);
    if (rc != 0) {
        const char* m = lua_tostring(L, -1);
        out += "=== error: " + strip_location(m ? m : "?") + "\n";
    } else {
        out += "=== ok\n";
    }
    lua_close(L);
    g_out = nullptr;
    return out;
}

// One thread's state, as the watchdog sees it.
struct Worker {
    std::atomic<int> file{-1};
    std::atomic<long long> since{0};  // when it took that file, ms
    bool abandoned = false;           // under the lock
};

// Workers: one per core, or LUAU_THREADS of them.
static size_t worker_count(size_t files) {
    size_t n = std::max(std::thread::hardware_concurrency(), 1u);
    if (const char* v = std::getenv("LUAU_THREADS"))
        n = std::max(std::atoi(v), 1);
    return std::min(n, files);
}

int main(int argc, char** argv) {
    // integer literals (`5i`), as luau_check takes them
    for (Luau::FValue<bool>* f = Luau::FValue<bool>::list; f; f = f->next)
        if (std::strcmp(f->name, "LuauIntegerType2") == 0)
            f->value = true;
    lua_CompileOptions opts = {};
    opts.optimizationLevel = 2;
    opts.debugLevel = 1;
    long budget = 20000000;
    double seconds = 0;
    bool stream = false;
    bool bytecode = false;
    std::vector<Record> files;  // from the command line: names only
    for (int i = 1; i < argc; ++i) {
        if (std::strcmp(argv[i], "--budget") == 0 && i + 1 < argc)
            budget = std::atol(argv[++i]);
        else if (std::strcmp(argv[i], "--seconds") == 0 && i + 1 < argc)
            seconds = std::atof(argv[++i]);
        else if (std::strcmp(argv[i], "--stream") == 0)
            stream = true;
        else if (std::strcmp(argv[i], "--bytecode") == 0)
            bytecode = true;
        else if (std::strcmp(argv[i], "-O0") == 0 || std::strcmp(argv[i], "-O1") == 0 ||
                 std::strcmp(argv[i], "-O2") == 0)
            opts.optimizationLevel = argv[i][2] - '0';
        else
            files.push_back({argv[i], {}});
    }
    const size_t named = files.size();
    if (stream) {
        binary_stdio();
        for (Record& r : read_records())
            files.push_back(std::move(r));
    }

    const int count = int(files.size());
    const auto start = std::chrono::steady_clock::now();
    const auto now_ms = [&] {
        return std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - start)
            .count();
    };
    std::atomic<int> next{0};
    std::mutex lock;
    std::condition_variable ended;
    int done = 0, stuck = 0;  // under the lock
    const auto work = [&](Worker* w) {
        for (int i = next++; i < count; i = next++) {
            w->since = now_ms();
            w->file = i;
            const std::string said = run_file(files[i], size_t(i) < named, bytecode, budget, opts);
            std::lock_guard<std::mutex> hold(lock);
            if (w->abandoned)
                return;  // too late: it was counted as never ending
            w->file = -1;
            std::fputs(said.c_str(), stdout);
            std::fflush(stdout);
            ++done;
            ended.notify_one();
        }
    };

    std::vector<std::unique_ptr<Worker>> workers;
    std::vector<std::thread> threads;
    const auto add_worker = [&] {
        workers.push_back(std::make_unique<Worker>());
        threads.emplace_back(work, workers.back().get());
    };
    for (size_t k = 0, n = worker_count(files.size()); k < n; ++k)
        add_worker();
    if (seconds <= 0) {
        for (std::thread& t : threads)
            t.join();
        return 0;
    }
    const long long limit = (long long)(seconds * 1000);
    std::unique_lock<std::mutex> hold(lock);
    while (done + stuck < count) {
        ended.wait_for(hold, std::chrono::milliseconds(50), [&] { return done + stuck >= count; });
        for (size_t k = 0, n = workers.size(); k < n; ++k) {
            Worker* w = workers[k].get();
            if (!w->abandoned && w->file >= 0 && now_ms() - w->since > limit) {
                w->abandoned = true;  // a thread gone for good: another takes its place
                ++stuck;
                add_worker();
            }
        }
    }
    hold.unlock();
    std::fflush(stdout);
    if (stuck > 0)
        std::_Exit(0);  // the stuck threads cannot be joined
    for (std::thread& t : threads)
        t.join();
    return 0;
}
