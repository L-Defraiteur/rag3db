#pragma once

#include <cstdio>

#if defined(__GLIBC__)
#include <execinfo.h>
#include <unistd.h>
#endif

namespace rag3db {
namespace common {

// La pile de l'appelant sur stderr, en adresses et symboles bruts (addr2line ou eu-addr2line sur
// la lib les relit). Pour les refus qui disent qu'un numéro de page faux est arrivé : le prochain
// cas se lit sur le journal de l'hôte, sans la base. Sans glibc, rien.
inline void printStackToStderr(const char* why) {
    fprintf(stderr, "stack of %s:\n", why);
#if defined(__GLIBC__)
    void* frames[48];
    const auto numFrames = backtrace(frames, 48);
    backtrace_symbols_fd(frames, numFrames, STDERR_FILENO);
#endif
    fflush(stderr);
}

} // namespace common
} // namespace rag3db
