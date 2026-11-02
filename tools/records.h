// records.h — many files through one pipe, between the harness and the tools.
//
// A record is a line "<length> <name>" and then exactly that many bytes, which
// may be anything. Standard input is read whole, in binary. A result goes back
// as "<index> ok <length>" or "<index> error <length>" and its bytes, written
// as soon as it is ready: a crash later keeps what was done.
#pragma once

#include <cstdio>
#include <cstdlib>
#include <string>
#include <vector>
#ifdef _WIN32
#include <fcntl.h>
#include <io.h>
#endif

struct Record {
    std::string name;
    std::string data;
};

// Lengths count bytes, so no line ends may be changed on the way.
inline void binary_stdio() {
#ifdef _WIN32
    _setmode(_fileno(stdin), _O_BINARY);
    _setmode(_fileno(stdout), _O_BINARY);
#endif
}

// Every record on standard input; whatever does not parse ends the list.
inline std::vector<Record> read_records() {
    std::string all;
    static char buf[1 << 16];
    for (size_t n; (n = std::fread(buf, 1, sizeof buf, stdin)) > 0;)
        all.append(buf, n);
    std::vector<Record> out;
    size_t at = 0;
    while (at < all.size()) {
        const size_t eol = all.find('\n', at);
        const size_t space = all.find(' ', at);
        if (eol == std::string::npos || space == std::string::npos || space > eol)
            break;
        char* end = nullptr;
        const unsigned long long size = std::strtoull(all.c_str() + at, &end, 10);
        if (end != all.c_str() + space || size > all.size() - eol - 1)
            break;
        out.push_back({all.substr(space + 1, eol - space - 1), all.substr(eol + 1, size)});
        at = eol + 1 + size;
    }
    return out;
}

// The caller holds whatever keeps two results from mixing.
inline void write_result(size_t index, bool ok, const char* data, size_t size) {
    std::fprintf(stdout, "%zu %s %zu\n", index, ok ? "ok" : "error", size);
    std::fwrite(data, 1, size, stdout);
    std::fflush(stdout);
}
