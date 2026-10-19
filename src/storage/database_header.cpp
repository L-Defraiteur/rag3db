#include "storage/database_header.h"

#include <cstring>

#include "common/assert.h"
#include "common/exception/runtime.h"
#include "common/exception/storage_version.h"
#include "common/file_system/file_info.h"
#include "common/serializer/buffered_file.h"
#include "common/serializer/deserializer.h"
#include "common/serializer/serializer.h"
#include "common/system_config.h"
#include "main/client_context.h"
#include "storage/page_manager.h"
#include "storage/storage_version_info.h"

namespace rag3db::storage {
// Lit la version de format et, à partir de la 41, le bâti du dernier écrivain ; refuse en le
// disant une base plus récente que ce moteur (en lecture comme en écriture : le format n'est
// pas lisible) ou plus ancienne que la plus ancienne lisible. Entre les deux, la base s'ouvre et
// son premier point de reprise la réécrit à la version courante.
static std::pair<storage_version_t, std::string> readStorageVersion(common::Deserializer& deSer) {
    std::string key;
    deSer.validateDebuggingInfo(key, "storage_version");
    storage_version_t savedStorageVersion = 0;
    deSer.deserializeValue(savedStorageVersion);
    std::string writer;
    if (savedStorageVersion > StorageVersionInfo::VERSION_BEFORE_WRITER_BUILD_ID) {
        deSer.validateDebuggingInfo(key, "writer");
        deSer.deserializeValue(writer);
    }
    const auto storageVersion = StorageVersionInfo::getStorageVersion();
    if (savedStorageVersion > storageVersion) {
        throw common::StorageVersionException(common::stringFormat(
            "This database was last written by the engine {} in storage format {}; this engine "
            "({}) reads and writes format {}: open it with an engine at least as recent.",
            writer.empty() ? std::string{"of an unknown build"} : writer, savedStorageVersion,
            StorageVersionInfo::getBuildID(), storageVersion));
    }
    if (savedStorageVersion < StorageVersionInfo::OLDEST_READABLE_STORAGE_VERSION) {
        throw common::StorageVersionException(common::stringFormat(
            "This database is in storage format {}, older than the oldest this engine ({}, "
            "format {}) can read ({}): export it with an engine of its time, then import it here.",
            savedStorageVersion, StorageVersionInfo::getBuildID(), storageVersion,
            StorageVersionInfo::OLDEST_READABLE_STORAGE_VERSION));
    }
    return {savedStorageVersion, writer};
}

static void validateMagicBytes(common::Deserializer& deSer) {
    std::string key;
    deSer.validateDebuggingInfo(key, "magic");
    const auto numMagicBytes = strlen(StorageVersionInfo::MAGIC_BYTES);
    uint8_t magicBytes[8];
    KU_ASSERT(numMagicBytes <= sizeof(magicBytes));
    for (auto i = 0u; i < numMagicBytes; i++) {
        deSer.deserializeValue<uint8_t>(magicBytes[i]);
    }
    if (memcmp(magicBytes, StorageVersionInfo::MAGIC_BYTES, numMagicBytes) != 0) {
        throw common::RuntimeException(
            "Unable to open database. The file is not a valid Rag3db database file!");
    }
}

void DatabaseHeader::updateCatalogPageRange(PageManager& pageManager, PageRange newPageRange) {
    if (catalogPageRange.startPageIdx != common::INVALID_PAGE_IDX) {
        pageManager.freePageRange(catalogPageRange);
    }
    catalogPageRange = newPageRange;
}

void DatabaseHeader::freeMetadataPageRange(PageManager& pageManager) const {
    if (metadataPageRange.startPageIdx != common::INVALID_PAGE_IDX) {
        pageManager.freePageRange(metadataPageRange);
    }
}

static void writeMagicBytes(common::Serializer& serializer) {
    serializer.writeDebuggingInfo("magic");
    const auto numMagicBytes = strlen(StorageVersionInfo::MAGIC_BYTES);
    for (auto i = 0u; i < numMagicBytes; i++) {
        serializer.serializeValue<uint8_t>(StorageVersionInfo::MAGIC_BYTES[i]);
    }
}

void DatabaseHeader::serialize(common::Serializer& ser) const {
    serializeAs(ser, StorageVersionInfo::getStorageVersion(), StorageVersionInfo::getBuildID());
}

void DatabaseHeader::serializeAs(common::Serializer& ser, uint64_t version,
    const std::string& writer) const {
    writeMagicBytes(ser);
    ser.writeDebuggingInfo("storage_version");
    ser.serializeValue<storage_version_t>(version);
    if (version > StorageVersionInfo::VERSION_BEFORE_WRITER_BUILD_ID) {
        ser.writeDebuggingInfo("writer");
        ser.serializeValue(writer);
    }
    ser.writeDebuggingInfo("catalog");
    ser.serializeValue(catalogPageRange.startPageIdx);
    ser.serializeValue(catalogPageRange.numPages);
    ser.writeDebuggingInfo("metadata");
    ser.serializeValue(metadataPageRange.startPageIdx);
    ser.serializeValue(metadataPageRange.numPages);
    ser.writeDebuggingInfo("databaseID");
    ser.serializeValue(databaseID.value);
    if (version > StorageVersionInfo::VERSION_BEFORE_DATA_FILE_EXTENT) {
        ser.writeDebuggingInfo("data_pages");
        ser.serializeValue(numDataPages);
    }
}

DatabaseHeader DatabaseHeader::deserialize(common::Deserializer& deSer) {
    validateMagicBytes(deSer);
    const auto [savedStorageVersion, writer] = readStorageVersion(deSer);
    PageRange catalogPageRange{}, metaPageRange{};
    common::ku_uuid_t databaseID{};
    std::string key;
    deSer.validateDebuggingInfo(key, "catalog");
    deSer.deserializeValue(catalogPageRange.startPageIdx);
    deSer.deserializeValue(catalogPageRange.numPages);
    deSer.validateDebuggingInfo(key, "metadata");
    deSer.deserializeValue(metaPageRange.startPageIdx);
    deSer.deserializeValue(metaPageRange.numPages);
    deSer.validateDebuggingInfo(key, "databaseID");
    deSer.deserializeValue(databaseID.value);
    DatabaseHeader header{catalogPageRange, metaPageRange, databaseID};
    header.storageVersion = savedStorageVersion;
    header.writerBuildID = writer;
    if (savedStorageVersion > StorageVersionInfo::VERSION_BEFORE_DATA_FILE_EXTENT) {
        deSer.validateDebuggingInfo(key, "data_pages");
        deSer.deserializeValue(header.numDataPages);
    }
    return header;
}

DatabaseHeader DatabaseHeader::createInitialHeader(common::RandomEngine* randomEngine) {
    // We generate a random UUID to act as the database ID
    return DatabaseHeader{{}, {}, common::UUID::generateRandomUUID(randomEngine)};
}

std::optional<DatabaseHeader> DatabaseHeader::readDatabaseHeader(common::FileInfo& dataFileInfo) {
    if (dataFileInfo.getFileSize() < common::RAG3DB_PAGE_SIZE) {
        // If the data file hasn't been written to there is no existing database header
        return std::nullopt;
    }
    auto reader = std::make_unique<common::BufferedFileReader>(dataFileInfo);
    common::Deserializer deSer(std::move(reader));
    try {
        return DatabaseHeader::deserialize(deSer);
    } catch (const common::RuntimeException&) {
        // It is possible we optimistically write to the database file before the first checkpoint
        // In this case the magic bytes check will fail and we assume there is no existing header
        return std::nullopt;
    }
}
} // namespace rag3db::storage
