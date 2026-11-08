#pragma once

#include <functional>
#include <memory>
#include <unordered_map>
#include <vector>

#include "compressed_file_system.h"
#include "file_system.h"

namespace rag3db {
namespace main {
class Database;
}

namespace storage {
class BufferManager;
};
namespace common {

class RAG3DB_API VirtualFileSystem final : public FileSystem {
    friend class storage::BufferManager;

public:
    VirtualFileSystem();
    explicit VirtualFileSystem(std::string homeDir);

    ~VirtualFileSystem() override;

    void registerFileSystem(std::unique_ptr<FileSystem> fileSystem);

    // Pour les tests seulement : la fabrique du système de fichiers par défaut (le local), lue à la
    // construction de chaque VirtualFileSystem, donc avant que la Database ouvre ses fichiers et
    // rejoue son journal. Un témoin de coupure y pose un système de fichiers qui garde en mémoire ce
    // qui n'est pas synchronisé. Sans effet hors des tests (aucune fabrique posée) ; nullptr la
    // retire. Sur le modèle de WALReplayer::setRecoveryHookForTesting.
    using default_file_system_factory_t =
        std::function<std::unique_ptr<FileSystem>(const std::string& homeDir)>;
    static void setDefaultFileSystemFactoryForTesting(default_file_system_factory_t factory);

    std::unique_ptr<FileInfo> openFile(const std::string& path, FileOpenFlags flags,
        main::ClientContext* context = nullptr) override;

    std::vector<std::string> glob(main::ClientContext* context,
        const std::string& path) const override;

    void overwriteFile(const std::string& from, const std::string& to) override;

    void createDir(const std::string& dir) const override;

    void removeFileIfExists(const std::string& path,
        const main::ClientContext* context = nullptr) override;

    bool fileOrPathExists(const std::string& path, main::ClientContext* context = nullptr) override;

    std::string expandPath(main::ClientContext* context, const std::string& path) const override;

    void syncFile(const FileInfo& fileInfo) const override;

    void cleanUP(main::ClientContext* context) override;

    bool handleFileViaFunction(const std::string& path) const override;

    function::TableFunction getHandleFunction(const std::string& path) const override;

    static VirtualFileSystem* GetUnsafe(const main::ClientContext& context);

protected:
    void readFromFile(FileInfo& fileInfo, void* buffer, uint64_t numBytes,
        uint64_t position) const override;

    int64_t readFile(FileInfo& fileInfo, void* buf, size_t nbyte) const override;

    void writeFile(FileInfo& fileInfo, const uint8_t* buffer, uint64_t numBytes,
        uint64_t offset) const override;

    int64_t seek(FileInfo& fileInfo, uint64_t offset, int whence) const override;

    void truncate(FileInfo& fileInfo, uint64_t size) const override;

    uint64_t getFileSize(const FileInfo& fileInfo) const override;

private:
    FileSystem* findFileSystem(const std::string& path) const;

    static FileCompressionType autoDetectCompressionType(const std::string& path);

private:
    std::vector<std::unique_ptr<FileSystem>> subSystems;
    std::unique_ptr<FileSystem> defaultFS;
    std::unordered_map<FileCompressionType, std::unique_ptr<CompressedFileSystem>>
        compressedFileSystem;
};

} // namespace common
} // namespace rag3db
