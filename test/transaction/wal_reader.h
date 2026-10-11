#pragma once

// Le lecteur du journal : un outil de diagnostic, en lecture seule. Il lit un fichier .wal comme
// le rejeu le lit (WALReplayer::dryReplay), sans rien rejouer, et rend ses enregistrements : rang,
// décalage, taille, type, transaction, table, et les identifiants internes qu'ils portent (les
// extrémités d'une relation, le nœud d'un détachement). Il dit aussi comment le fichier finit :
// sur un point de reprise, proprement, déchiré au milieu d'un enregistrement, ou sur une erreur.
// Né du ticket C7 (11 octobre 2026), trouvé à la main là où ce lecteur aurait montré la relation
// pendante.
//
// Le contexte d'un client ne sert qu'à sa mémoire : un journal se lit depuis n'importe quelle
// base, sans ouvrir celle qui l'a écrit.

#include <filesystem>
#include <sstream>
#include <string>
#include <vector>

#include "catalog/catalog_entry/catalog_entry.h"
#include "catalog/catalog_entry/catalog_entry_type.h"
#include "common/exception/end_of_file.h"
#include "common/exception/exception.h"
#include "common/file_system/file_info.h"
#include "common/file_system/virtual_file_system.h"
#include "common/serializer/buffered_file.h"
#include "common/serializer/deserializer.h"
#include "common/types/internal_id_t.h"
#include "common/vector/value_vector.h"
#include "main/client_context.h"
#include "storage/buffer_manager/memory_manager.h"
#include "storage/wal/checksum_reader.h"
#include "storage/wal/wal_record.h"

namespace rag3db {
namespace testing {

struct WalEntry {
    uint64_t index = 0;
    uint64_t offset = 0; // le début de l'enregistrement dans le fichier
    uint64_t size = 0;
    int64_t transaction = -1; // le rang du BEGIN qui l'ouvre, -1 hors transaction
    storage::WALRecordType type = storage::WALRecordType::INVALID_RECORD;
    std::string detail;
};

enum class WalEnd {
    CHECKPOINT, // le dernier enregistrement est un point de reprise
    CLEAN,      // le fichier finit sur la fin d'un enregistrement
    TORN,       // le fichier finit au milieu d'un enregistrement
    ERROR,      // une erreur de lecture (somme de contrôle, type inconnu)
    LIMIT,      // la borne d'enregistrements est atteinte
};

struct WalReading {
    bool checksums = false;
    std::vector<WalEntry> entries;
    WalEnd end = WalEnd::CLEAN;
    uint64_t endOffset = 0; // là où la lecture s'arrête
    uint64_t fileSize = 0;
    std::string error;
    // Le décalage de la fin du dernier COMMIT : ce que le rejeu rejouerait.
    uint64_t lastCommitEnd = 0;
};

inline std::string walRecordTypeName(storage::WALRecordType type) {
    using storage::WALRecordType;
    switch (type) {
    case WALRecordType::BEGIN_TRANSACTION_RECORD:
        return "BEGIN";
    case WALRecordType::COMMIT_RECORD:
        return "COMMIT";
    case WALRecordType::COPY_TABLE_RECORD:
        return "COPY_TABLE";
    case WALRecordType::CREATE_CATALOG_ENTRY_RECORD:
        return "CREATE_CATALOG_ENTRY";
    case WALRecordType::DROP_CATALOG_ENTRY_RECORD:
        return "DROP_CATALOG_ENTRY";
    case WALRecordType::ALTER_TABLE_ENTRY_RECORD:
        return "ALTER_TABLE_ENTRY";
    case WALRecordType::UPDATE_SEQUENCE_RECORD:
        return "UPDATE_SEQUENCE";
    case WALRecordType::TABLE_INSERTION_RECORD:
    case WALRecordType::TABLE_INSERTION_RECORD_RAW_ARRAYS:
        return "TABLE_INSERTION";
    case WALRecordType::NODE_DELETION_RECORD:
        return "NODE_DELETION";
    case WALRecordType::NODE_UPDATE_RECORD:
    case WALRecordType::NODE_UPDATE_RECORD_RAW_ARRAYS:
        return "NODE_UPDATE";
    case WALRecordType::REL_DELETION_RECORD:
        return "REL_DELETION";
    case WALRecordType::REL_DETACH_DELETE_RECORD:
        return "REL_DETACH_DELETE";
    case WALRecordType::REL_UPDATE_RECORD:
    case WALRecordType::REL_UPDATE_RECORD_RAW_ARRAYS:
        return "REL_UPDATE";
    case WALRecordType::LOAD_EXTENSION_RECORD:
        return "LOAD_EXTENSION";
    case WALRecordType::CHECKPOINT_RECORD:
        return "CHECKPOINT";
    default:
        return "TYPE_" + std::to_string(static_cast<unsigned>(type));
    }
}

inline const char* walEndName(WalEnd end) {
    switch (end) {
    case WalEnd::CHECKPOINT:
        return "checkpoint";
    case WalEnd::CLEAN:
        return "clean";
    case WalEnd::TORN:
        return "torn";
    case WalEnd::ERROR:
        return "error";
    case WalEnd::LIMIT:
        return "limit";
    }
    return "?";
}

// Les identifiants internes d'un vecteur (au plus maxValues), « table:décalage » ; un vecteur d'un
// autre type ne dit rien.
inline std::string walInternalIDs(const common::ValueVector* vector, size_t maxValues = 4) {
    if (vector == nullptr || vector->dataType.getLogicalTypeID() != common::LogicalTypeID::INTERNAL_ID) {
        return "";
    }
    std::ostringstream out;
    const auto& selVector = vector->state->getSelVector();
    const auto size = selVector.getSelSize();
    out << "[";
    for (auto i = 0u; i < size && i < maxValues; ++i) {
        const auto pos = selVector[i];
        if (i > 0) {
            out << " ";
        }
        if (vector->isNull(pos)) {
            out << "null";
        } else {
            const auto id = vector->getValue<common::internalID_t>(pos);
            out << id.tableID << ":" << id.offset;
        }
    }
    if (size > maxValues) {
        out << " …+" << size - maxValues;
    }
    out << "]";
    return out.str();
}

inline std::string walRecordDetail(const storage::WALRecord& record) {
    using namespace storage;
    std::ostringstream out;
    switch (record.type) {
    case WALRecordType::CREATE_CATALOG_ENTRY_RECORD: {
        const auto& r = record.constCast<CreateCatalogEntryRecord>();
        const auto* entry = r.ownedCatalogEntry ? r.ownedCatalogEntry.get() : r.catalogEntry;
        if (entry) {
            out << catalog::CatalogEntryTypeUtils::toString(entry->getType()) << " "
                << entry->getName();
        }
        if (r.isInternal) {
            out << " (internal)";
        }
    } break;
    case WALRecordType::DROP_CATALOG_ENTRY_RECORD: {
        const auto& r = record.constCast<DropCatalogEntryRecord>();
        out << catalog::CatalogEntryTypeUtils::toString(r.entryType) << " id " << r.entryID;
    } break;
    case WALRecordType::COPY_TABLE_RECORD: {
        out << "table " << record.constCast<CopyTableRecord>().tableID;
    } break;
    case WALRecordType::UPDATE_SEQUENCE_RECORD: {
        const auto& r = record.constCast<UpdateSequenceRecord>();
        out << "sequence " << r.sequenceID << " count " << r.kCount;
    } break;
    case WALRecordType::TABLE_INSERTION_RECORD:
    case WALRecordType::TABLE_INSERTION_RECORD_RAW_ARRAYS: {
        const auto& r = record.constCast<TableInsertionRecord>();
        out << "table " << r.tableID << " rows " << r.numRows;
        for (const auto& vector : r.ownedVectors) {
            const auto ids = walInternalIDs(vector.get());
            if (!ids.empty()) {
                out << " " << ids;
            }
        }
    } break;
    case WALRecordType::NODE_DELETION_RECORD: {
        const auto& r = record.constCast<NodeDeletionRecord>();
        out << "table " << r.tableID << " offset " << r.nodeOffset;
    } break;
    case WALRecordType::NODE_UPDATE_RECORD:
    case WALRecordType::NODE_UPDATE_RECORD_RAW_ARRAYS: {
        const auto& r = record.constCast<NodeUpdateRecord>();
        out << "table " << r.tableID << " column " << r.columnID << " offset " << r.nodeOffset;
    } break;
    case WALRecordType::REL_DELETION_RECORD: {
        const auto& r = record.constCast<RelDeletionRecord>();
        out << "table " << r.tableID << " src " << walInternalIDs(r.ownedSrcNodeIDVector.get())
            << " dst " << walInternalIDs(r.ownedDstNodeIDVector.get()) << " rel "
            << walInternalIDs(r.ownedRelIDVector.get());
    } break;
    case WALRecordType::REL_DETACH_DELETE_RECORD: {
        const auto& r = record.constCast<RelDetachDeleteRecord>();
        out << "table " << r.tableID << " "
            << (r.direction == common::RelDataDirection::FWD ? "fwd" : "bwd") << " node "
            << walInternalIDs(r.ownedSrcNodeIDVector.get());
    } break;
    case WALRecordType::REL_UPDATE_RECORD:
    case WALRecordType::REL_UPDATE_RECORD_RAW_ARRAYS: {
        const auto& r = record.constCast<RelUpdateRecord>();
        out << "table " << r.tableID << " column " << r.columnID << " src "
            << walInternalIDs(r.ownedSrcNodeIDVector.get()) << " dst "
            << walInternalIDs(r.ownedDstNodeIDVector.get()) << " rel "
            << walInternalIDs(r.ownedRelIDVector.get());
    } break;
    case WALRecordType::LOAD_EXTENSION_RECORD: {
        out << record.constCast<LoadExtensionRecord>().path;
    } break;
    default:
        break;
    }
    return out.str();
}

namespace wal_reader_detail {

inline uint64_t readOffset(common::Deserializer& deserializer, bool checksums) {
    if (checksums) {
        return deserializer.getReader()->cast<storage::ChecksumReader>()->getReadOffset();
    }
    return deserializer.getReader()->cast<common::BufferedFileReader>()->getReadOffset();
}

inline uint64_t lastEntryEnd(const WalReading& reading) {
    return reading.entries.empty() ? uint64_t{0} :
                                     reading.entries.back().offset + reading.entries.back().size;
}

// Une lecture, sommes de contrôle ou non. Rend faux si l'en-tête dit l'autre forme.
inline bool readWith(const std::string& path, main::ClientContext& context, bool checksums,
    uint64_t maxRecords, WalReading& reading) {
    reading = WalReading{};
    reading.checksums = checksums;
    reading.fileSize = std::filesystem::file_size(path);
    auto fileInfo = common::VirtualFileSystem::GetUnsafe(context)->openFile(path,
        common::FileOpenFlags(common::FileFlags::READ_ONLY));
    std::unique_ptr<common::Reader> reader;
    if (checksums) {
        reader = std::make_unique<storage::ChecksumReader>(*fileInfo,
            *storage::MemoryManager::Get(context), "checksum mismatch");
    } else {
        reader = std::make_unique<common::BufferedFileReader>(*fileInfo);
    }
    common::Deserializer deserializer{std::move(reader)};
    int64_t transaction = -1;
    int64_t numBegins = 0;
    try {
        // L'en-tête : l'identifiant de la base, puis la forme (sommes de contrôle ou non).
        deserializer.getReader()->onObjectBegin();
        common::ku_uuid_t databaseID{};
        deserializer.deserializeValue(databaseID);
        uint8_t checksumsByte = 0;
        deserializer.deserializeValue(checksumsByte);
        deserializer.getReader()->onObjectEnd();
        if ((checksumsByte != 0) != checksums) {
            return false;
        }
        while (!deserializer.finished()) {
            if (reading.entries.size() >= maxRecords) {
                reading.end = WalEnd::LIMIT;
                reading.endOffset = readOffset(deserializer, checksums);
                return true;
            }
            WalEntry entry;
            entry.index = reading.entries.size();
            entry.offset = readOffset(deserializer, checksums);
            auto record = storage::WALRecord::deserialize(deserializer, context);
            entry.size = readOffset(deserializer, checksums) - entry.offset;
            entry.type = record->type;
            if (record->type == storage::WALRecordType::BEGIN_TRANSACTION_RECORD) {
                transaction = numBegins++;
            }
            entry.transaction = transaction;
            entry.detail = walRecordDetail(*record);
            reading.entries.push_back(std::move(entry));
            if (record->type == storage::WALRecordType::COMMIT_RECORD) {
                reading.lastCommitEnd = readOffset(deserializer, checksums);
                transaction = -1;
            }
            if (record->type == storage::WALRecordType::CHECKPOINT_RECORD) {
                reading.end = WalEnd::CHECKPOINT;
                reading.endOffset = readOffset(deserializer, checksums);
                return true;
            }
        }
        reading.end = WalEnd::CLEAN;
        reading.endOffset = readOffset(deserializer, checksums);
    } catch (const common::EndOfFileException& e) {
        // Le fichier finit au milieu d'un enregistrement (comme dryReplay) : la lecture s'arrête à
        // la fin du dernier enregistrement entier.
        reading.end = WalEnd::TORN;
        reading.endOffset = lastEntryEnd(reading);
        reading.error = e.what();
    } catch (const std::exception& e) {
        // Une somme de contrôle fausse, un type inconnu : une corruption, pas une fin.
        reading.end = WalEnd::ERROR;
        reading.endOffset = lastEntryEnd(reading);
        reading.error = e.what();
    }
    return true;
}

} // namespace wal_reader_detail

// Lit le journal de path, au plus maxRecords enregistrements. Essaie d'abord la forme de la base
// du contexte, puis l'autre si l'en-tête la dit.
inline WalReading readWal(const std::string& path, main::ClientContext& context,
    uint64_t maxRecords = 100000) {
    WalReading reading;
    const bool checksums = context.getDBConfig()->enableChecksums;
    if (!wal_reader_detail::readWith(path, context, checksums, maxRecords, reading)) {
        wal_reader_detail::readWith(path, context, !checksums, maxRecords, reading);
    }
    return reading;
}

inline std::string formatWal(const WalReading& reading) {
    std::ostringstream out;
    out << "wal: " << reading.fileSize << " bytes, checksums " << (reading.checksums ? "on" : "off")
        << ", " << reading.entries.size() << " records\n";
    for (const auto& e : reading.entries) {
        out << "  #" << e.index << " @" << e.offset << " +" << e.size << " tx "
            << (e.transaction < 0 ? std::string("-") : std::to_string(e.transaction)) << " "
            << walRecordTypeName(e.type);
        if (!e.detail.empty()) {
            out << " " << e.detail;
        }
        out << "\n";
    }
    out << "end: " << walEndName(reading.end) << " at " << reading.endOffset
        << ", last commit ends at " << reading.lastCommitEnd;
    if (!reading.error.empty()) {
        out << " (" << reading.error << ")";
    }
    out << "\n";
    return out.str();
}

} // namespace testing
} // namespace rag3db
