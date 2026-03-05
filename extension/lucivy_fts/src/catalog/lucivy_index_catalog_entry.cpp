#include "catalog/lucivy_index_catalog_entry.h"

#include "common/serializer/buffer_reader.h"
#include "common/serializer/buffer_writer.h"
#include "common/serializer/deserializer.h"
#include "common/serializer/serializer.h"

namespace rag3db {
namespace lucivy_fts_extension {

using namespace common;

std::shared_ptr<BufferWriter> LucivyIndexAuxInfo::serialize() const {
    auto writer = std::make_shared<BufferWriter>();
    auto ser = Serializer(writer);
    ser.serializeValue(indexPath);
    ser.serializeValue(schemaJson);
    ser.serializeValue(stemmer);
    return writer;
}

std::unique_ptr<LucivyIndexAuxInfo> LucivyIndexAuxInfo::deserialize(
    std::unique_ptr<BufferReader> reader) {
    Deserializer deSer{std::move(reader)};
    std::string indexPath, schemaJson, stemmer;
    deSer.deserializeValue(indexPath);
    deSer.deserializeValue(schemaJson);
    deSer.deserializeValue(stemmer);
    return std::make_unique<LucivyIndexAuxInfo>(
        std::move(indexPath), std::move(schemaJson), std::move(stemmer));
}

std::unique_ptr<catalog::IndexAuxInfo> LucivyIndexAuxInfo::copy() {
    return std::make_unique<LucivyIndexAuxInfo>(indexPath, schemaJson, stemmer);
}

std::string LucivyIndexAuxInfo::toCypher(const catalog::IndexCatalogEntry& indexEntry,
    const catalog::ToCypherInfo&) const {
    auto tableName = indexEntry.getIndexName();
    return stringFormat(
        "CALL CREATE_LUCIVY_INDEX('{}', '{}', stemmer := '{}');", tableName, tableName, stemmer);
}

} // namespace lucivy_fts_extension
} // namespace rag3db
