#include "storage/readers_lock.h"

#include <algorithm>
#include <thread>

#include "common/file_system/local_file_system.h"

#ifdef _WIN32
#include <windows.h>
#else
#include <fcntl.h>
#include <sys/file.h>
#include <unistd.h>
#endif

namespace rag3db {
namespace storage {

namespace {

#ifdef _WIN32
using native_t = HANDLE;
native_t openLockFile(const std::string& path) {
    auto handle = CreateFileA(path.c_str(), GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, nullptr, OPEN_ALWAYS,
        FILE_ATTRIBUTE_NORMAL, nullptr);
    if (handle == INVALID_HANDLE_VALUE) {
        // Un dossier en lecture seule : ouvrir le fichier existant sans écrire.
        handle = CreateFileA(path.c_str(), GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, nullptr, OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL, nullptr);
    }
    return handle == INVALID_HANDLE_VALUE ? nullptr : handle;
}
bool tryLock(native_t handle, ReadersLock::Mode mode) {
    DWORD flags = LOCKFILE_FAIL_IMMEDIATELY;
    if (mode == ReadersLock::Mode::EXCLUSIVE) {
        flags |= LOCKFILE_EXCLUSIVE_LOCK;
    }
    OVERLAPPED overlapped = {0};
    return LockFileEx(handle, flags, 0 /*reserved*/, 1 /*numBytesLow*/, 0 /*numBytesHigh*/,
        &overlapped);
}
void unlockAndClose(native_t handle, bool held) {
    if (held) {
        OVERLAPPED overlapped = {0};
        UnlockFileEx(handle, 0 /*reserved*/, 1 /*numBytesLow*/, 0 /*numBytesHigh*/, &overlapped);
    }
    CloseHandle(handle);
}
#else
using native_t = int;
native_t openLockFile(const std::string& path) {
    auto fd = open(path.c_str(), O_RDWR | O_CREAT | O_CLOEXEC, 0644);
    if (fd == -1) {
        // Un dossier en lecture seule : flock vaut aussi sur un fichier ouvert en lecture.
        fd = open(path.c_str(), O_RDONLY | O_CLOEXEC);
    }
    return fd;
}
bool tryLock(native_t fd, ReadersLock::Mode mode) {
    const auto operation = mode == ReadersLock::Mode::SHARED ? LOCK_SH : LOCK_EX;
    return flock(fd, operation | LOCK_NB) == 0;
}
void unlockAndClose(native_t fd, bool /*held*/) {
    // Fermer le dernier descripteur de la description rend le verrou.
    close(fd);
}
#endif

} // namespace

ReadersLock ReadersLock::acquire(const std::string& databasePath, Mode mode,
    std::chrono::milliseconds bound) {
    ReadersLock lock;
    if (databasePath.empty() || !common::LocalFileSystem::isLocalPath(databasePath)) {
        return lock;
    }
    const auto native = openLockFile(getFilePath(databasePath));
#ifdef _WIN32
    if (native == nullptr) {
        return lock;
    }
    lock.handle = native;
#else
    if (native == -1) {
        return lock;
    }
    lock.fd = native;
#endif
    const auto start = std::chrono::steady_clock::now();
    auto pause = std::chrono::microseconds(500);
    while (true) {
        if (tryLock(native, mode)) {
            lock.held = true;
            break;
        }
        const auto elapsed = std::chrono::steady_clock::now() - start;
        if (elapsed >= bound) {
            lock.timedOut_ = true;
            break;
        }
        std::this_thread::sleep_for(pause);
        pause = std::min(pause * 2, std::chrono::microseconds(5000));
    }
    lock.waitedMs_ = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::steady_clock::now() - start)
                         .count();
    if (!lock.held) {
        lock.release();
    }
    return lock;
}

ReadersLock::ReadersLock(ReadersLock&& other) noexcept {
    *this = std::move(other);
}

ReadersLock& ReadersLock::operator=(ReadersLock&& other) noexcept {
    if (this != &other) {
        release();
#ifdef _WIN32
        handle = other.handle;
        other.handle = nullptr;
#else
        fd = other.fd;
        other.fd = -1;
#endif
        held = other.held;
        timedOut_ = other.timedOut_;
        waitedMs_ = other.waitedMs_;
        other.held = false;
    }
    return *this;
}

void ReadersLock::release() {
#ifdef _WIN32
    if (handle != nullptr) {
        unlockAndClose(static_cast<HANDLE>(handle), held);
        handle = nullptr;
    }
#else
    if (fd != -1) {
        unlockAndClose(fd, held);
        fd = -1;
    }
#endif
    held = false;
}

} // namespace storage
} // namespace rag3db
