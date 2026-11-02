// luau_run.cpp — run Luau source files and print what they print.
//
// For the decompiler's differential tests: a generated program and its
// decompiled version must print the same thing. Each file runs in a fresh
// VM (standard libraries, sandboxed), compiled at -O2 like Roblox game
// scripts, with an instruction budget so a broken program cannot hang.
//
//   luau_run FILE...
// prints, per file:  "=== FILE" then its output, then "=== ok" or
// "=== error: MESSAGE" (chunk names and line numbers stripped, since they
// differ between a program and its decompiled form).
#include "lua.h"
#include "lualib.h"
#include "luacode.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iterator>
#include <string>

static long g_budget = 0;

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
    std::string line;
    for (int i = 1; i <= n; ++i) {
        size_t len = 0;
        const char* s = luaL_tolstring(L, i, &len);
        if (i > 1)
            line += '\t';
        line.append(s, len);
        lua_pop(L, 1);
    }
    std::printf("%s\n", line.c_str());
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

int main(int argc, char** argv) {
    lua_CompileOptions opts = {};
    opts.optimizationLevel = 2;
    opts.debugLevel = 1;
    for (int i = 1; i < argc; ++i) {
        std::ifstream f(argv[i], std::ios::binary);
        std::printf("=== %s\n", argv[i]);
        if (!f) {
            std::printf("=== error: cannot open\n");
            continue;
        }
        const std::string src((std::istreambuf_iterator<char>(f)),
                              std::istreambuf_iterator<char>());
        size_t size = 0;
        char* bc = luau_compile(src.data(), src.size(), &opts, &size);
        lua_State* L = luaL_newstate();
        luaL_openlibs(L);
        lua_pushcfunction(L, lprint, "print");
        lua_setglobal(L, "print");
        luaL_sandbox(L);
        luaL_sandboxthread(L);
        lua_callbacks(L)->interrupt = interrupt;
        g_budget = 20000000;
        int rc = luau_load(L, "=prog", bc, size, 0);
        std::free(bc);
        if (rc == 0)
            rc = lua_pcall(L, 0, 0, 0);
        if (rc != 0) {
            const char* m = lua_tostring(L, -1);
            std::printf("=== error: %s\n", strip_location(m ? m : "?").c_str());
        } else {
            std::printf("=== ok\n");
        }
        std::fflush(stdout);
        lua_close(L);
    }
    return 0;
}
