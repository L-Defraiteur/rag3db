#pragma once

// Un système de fichiers d'essai qui simule une coupure de courant (ticket
// 2026-10-04-durabilite-sur-faute-d-entree-sortie-ou-coupure).
//
// Il enveloppe le système de fichiers local. Chaque écriture (writeFile) et chaque troncature vont
// dans une couche en mémoire, propre au chemin ; les lectures et la taille voient cette couche ;
// seul syncFile la recopie sur le disque, dans l'ordre, puis synchronise le vrai fichier. Une
// coupure se joue dans un processus fils qui meurt par _exit à un point nommé : la couche meurt
// avec lui, et ce que le parent rouvre est exactement ce qui était synchronisé. Le système
// d'exploitation n'est pas simulé : c'est le fils qui garde le non-synchronisé, comme les bancs de
// SQLite et de PostgreSQL tuent le processus.
//
// Les entrées de répertoire (16 novembre 2026, hotfixs) : par défaut, la création et la suppression
// restent immédiates, comme si chaque entrée de répertoire était synchronisée. Avec
// pendingDirectoryEntries() à vrai, elles attendent la synchronisation de leur répertoire
// (syncDirectory), comme sur un vrai disque après une coupure :
// - un fichier créé ne vit que sous un nom caché (`<chemin>.cut-entry-pending`) jusqu'à
//   syncDirectory de son répertoire, qui lui donne son nom ; à la coupure, il n'a pas de nom ;
// - un fichier supprimé disparaît pour le fils, mais reste sur le disque jusqu'à syncDirectory ;
//   à la coupure, il est encore là.
// La troncature à l'ouverture (CREATE_AND_TRUNCATE) reste immédiate.
//
// Une faute d'entrée-sortie s'injecte par failSyncOf : la synchronisation d'un chemin choisi lève,
// comme sur un disque plein ; ce qui n'était pas synchronisé reste dans la couche, perdu à la
// coupure.
//
// Posé par VirtualFileSystem::setDefaultFileSystemFactoryForTesting dans le fils, avant
// d'ouvrir la base.

#include <algorithm>
#include <atomic>
#include <cstring>
#include <filesystem>
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <set>
#include <string>
#include <vector>

#include "common/exception/io.h"
#include "common/file_system/file_info.h"
#include "common/file_system/file_system.h"
#include "common/file_system/local_file_system.h"

namespace rag3db {
namespace testing {

class CutFileSystem final : public common::FileSystem {
    struct Op {
        bool isTruncate;
        uint64_t offset; // la taille, pour une troncature
        std::vector<uint8_t> bytes;
    };
    struct Layer {
        std::mutex mtx;
        std::vector<Op> ops;
    };

public:
    struct CutFileInfo final : common::FileInfo {
        CutFileInfo(std::string path, std::unique_ptr<common::FileInfo> inner,
            common::FileSystem* fileSystem)
            : FileInfo{std::move(path), fileSystem}, inner{std::move(inner)} {}
        std::unique_ptr<common::FileInfo> inner;
        uint64_t cursor = 0;
    };

    explicit CutFileSystem(const std::string& homeDir)
        : FileSystem{homeDir}, local{std::make_unique<common::LocalFileSystem>(homeDir)} {}

    std::unique_ptr<common::FileInfo> openFile(const std::string& path, common::FileOpenFlags flags,
        main::ClientContext* context = nullptr) override {
        if (pendingDirectoryEntries()) {
            std::string hidden;
            {
                std::lock_guard lck{entriesMtx};
                const bool removed = pendingRemoved.contains(path);
                if (auto it = pendingCreated.find(path); it != pendingCreated.end()) {
                    hidden = it->second;
                } else if ((removed || !local->fileOrPathExists(path, context)) &&
                           (flags.flags & (common::FileFlags::CREATE_IF_NOT_EXISTS |
                                              common::FileFlags::CREATE_AND_TRUNCATE_IF_EXISTS))) {
                    hidden = path + ".cut-entry-pending";
                    pendingCreated[path] = hidden;
                } else if (removed) {
                    throw common::IOException("Cannot open file " + path + ": removed");
                }
            }
            if (!hidden.empty()) {
                auto inner = local->openFile(hidden, flags, context);
                return std::make_unique<CutFileInfo>(path, std::move(inner), this);
            }
        }
        auto inner = local->openFile(path, flags, context);
        const auto key = inner->path;
        return std::make_unique<CutFileInfo>(key, std::move(inner), this);
    }
    std::vector<std::string> glob(main::ClientContext* context,
        const std::string& path) const override {
        return local->glob(context, path);
    }
    void overwriteFile(const std::string& from, const std::string& to) override {
        local->overwriteFile(from, to);
    }
    void copyFile(const std::string& from, const std::string& to) override {
        local->copyFile(from, to);
    }
    void createDir(const std::string& dir) const override { local->createDir(dir); }
    void removeFileIfExists(const std::string& path,
        const main::ClientContext* context = nullptr) override {
        {
            std::lock_guard lck{layersMtx};
            layers.erase(path);
        }
        if (pendingDirectoryEntries()) {
            std::lock_guard lck{entriesMtx};
            if (auto it = pendingCreated.find(path); it != pendingCreated.end()) {
                local->removeFileIfExists(it->second, context);
                pendingCreated.erase(it);
            }
            if (local->fileOrPathExists(path, const_cast<main::ClientContext*>(context))) {
                pendingRemoved.insert(path);
            }
            return;
        }
        local->removeFileIfExists(path, context);
    }
    bool fileOrPathExists(const std::string& path,
        main::ClientContext* context = nullptr) override {
        if (pendingDirectoryEntries()) {
            std::lock_guard lck{entriesMtx};
            if (pendingCreated.contains(path)) {
                return true;
            }
            if (pendingRemoved.contains(path)) {
                return false;
            }
        }
        return local->fileOrPathExists(path, context);
    }

    // Les entrées de répertoire attendent leur synchronisation (voir l'en-tête).
    static bool& pendingDirectoryEntries() {
        static bool pending = false;
        return pending;
    }

    // Synchronise les entrées du répertoire `dir` : les suppressions en attente se font, les
    // créations en attente prennent leur nom.
    void syncDirectory(const std::string& dir) const override {
        std::lock_guard lck{entriesMtx};
        const auto inDir = [&](const std::string& path) {
            return std::filesystem::path(path).parent_path() == std::filesystem::path(dir);
        };
        for (auto it = pendingRemoved.begin(); it != pendingRemoved.end();) {
            if (inDir(*it) && !pendingCreated.contains(*it)) {
                std::filesystem::remove(*it);
                it = pendingRemoved.erase(it);
            } else {
                ++it;
            }
        }
        for (auto it = pendingCreated.begin(); it != pendingCreated.end();) {
            if (inDir(it->first)) {
                std::filesystem::rename(it->second, it->first);
                pendingRemoved.erase(it->first);
                it = pendingCreated.erase(it);
            } else {
                ++it;
            }
        }
    }
    std::string expandPath(main::ClientContext* context, const std::string& path) const override {
        return local->expandPath(context, path);
    }
    bool canHandleFile(const std::string_view) const override { return true; }

    // La faute injectée : la synchronisation d'un chemin pour lequel la fonction rend vrai lève.
    static std::function<bool(const std::string& path)>& failSyncOf() {
        static std::function<bool(const std::string& path)> predicate;
        return predicate;
    }

    void syncFile(const common::FileInfo& fileInfo) const override {
        auto& info = const_cast<common::FileInfo&>(fileInfo).cast<CutFileInfo>();
        if (const auto& fail = failSyncOf(); fail && fail(info.path)) {
            throw common::IOException(
                "injected: no space left on device while syncing " + info.path);
        }
        auto& layer = layerOf(info.path);
        std::lock_guard lck{layer.mtx};
        for (const auto& op : layer.ops) {
            if (op.isTruncate) {
                info.inner->truncate(op.offset);
            } else if (!op.bytes.empty()) {
                info.inner->writeFile(op.bytes.data(), op.bytes.size(), op.offset);
            }
        }
        layer.ops.clear();
        info.inner->syncFile();
    }

protected:
    void readFromFile(common::FileInfo& fileInfo, void* buffer, uint64_t numBytes,
        uint64_t position) const override {
        auto& info = fileInfo.cast<CutFileInfo>();
        auto& layer = layerOf(info.path);
        std::lock_guard lck{layer.mtx};
        auto* out = static_cast<uint8_t*>(buffer);
        std::memset(out, 0, numBytes);
        // Le vrai fichier, coupé à la première troncature de la couche qui le raccourcit.
        uint64_t visible = info.inner->getFileSize();
        for (const auto& op : layer.ops) {
            if (op.isTruncate) {
                visible = std::min(visible, op.offset);
            }
        }
        if (position < visible) {
            const auto n = std::min<uint64_t>(numBytes, visible - position);
            info.inner->readFromFile(out, n, position);
        }
        // Puis les opérations de la couche, dans l'ordre.
        uint64_t size = info.inner->getFileSize();
        for (const auto& op : layer.ops) {
            if (op.isTruncate) {
                size = op.offset;
                if (position + numBytes > size) {
                    const auto from = std::max(position, size);
                    std::memset(out + (from - position), 0, position + numBytes - from);
                }
                continue;
            }
            const auto end = op.offset + op.bytes.size();
            size = std::max(size, end);
            const auto lo = std::max(position, op.offset);
            const auto hi = std::min(position + numBytes, end);
            if (lo < hi) {
                std::memcpy(out + (lo - position), op.bytes.data() + (lo - op.offset), hi - lo);
            }
        }
    }
    int64_t readFile(common::FileInfo& fileInfo, void* buf, size_t nbyte) const override {
        auto& info = fileInfo.cast<CutFileInfo>();
        const auto size = getFileSize(info);
        if (info.cursor >= size) {
            return 0;
        }
        const auto n = std::min<uint64_t>(nbyte, size - info.cursor);
        readFromFile(info, buf, n, info.cursor);
        info.cursor += n;
        return static_cast<int64_t>(n);
    }
    void writeFile(common::FileInfo& fileInfo, const uint8_t* buffer, uint64_t numBytes,
        uint64_t offset) const override {
        auto& info = fileInfo.cast<CutFileInfo>();
        auto& layer = layerOf(info.path);
        std::lock_guard lck{layer.mtx};
        layer.ops.push_back(Op{false, offset, std::vector<uint8_t>(buffer, buffer + numBytes)});
    }
    int64_t seek(common::FileInfo& fileInfo, uint64_t offset, int whence) const override {
        auto& info = fileInfo.cast<CutFileInfo>();
        if (whence == SEEK_SET) {
            info.cursor = offset;
        } else if (whence == SEEK_CUR) {
            info.cursor += offset;
        } else {
            info.cursor = getFileSize(info) + offset;
        }
        return static_cast<int64_t>(info.cursor);
    }
    void truncate(common::FileInfo& fileInfo, uint64_t size) const override {
        auto& info = fileInfo.cast<CutFileInfo>();
        auto& layer = layerOf(info.path);
        std::lock_guard lck{layer.mtx};
        layer.ops.push_back(Op{true, size, {}});
    }
    uint64_t getFileSize(const common::FileInfo& fileInfo) const override {
        const auto& info = fileInfo.constCast<CutFileInfo>();
        auto& layer = layerOf(info.path);
        std::lock_guard lck{layer.mtx};
        uint64_t size = info.inner->getFileSize();
        for (const auto& op : layer.ops) {
            size =
                op.isTruncate ? op.offset : std::max<uint64_t>(size, op.offset + op.bytes.size());
        }
        return size;
    }

private:
    Layer& layerOf(const std::string& path) const {
        std::lock_guard lck{layersMtx};
        auto& slot = layers[path];
        if (!slot) {
            slot = std::make_unique<Layer>();
        }
        return *slot;
    }

    std::unique_ptr<common::LocalFileSystem> local;
    mutable std::mutex entriesMtx;
    mutable std::map<std::string, std::string> pendingCreated; // nom → nom caché
    mutable std::set<std::string> pendingRemoved;
    mutable std::mutex layersMtx;
    mutable std::map<std::string, std::unique_ptr<Layer>> layers;
};

} // namespace testing
} // namespace rag3db
