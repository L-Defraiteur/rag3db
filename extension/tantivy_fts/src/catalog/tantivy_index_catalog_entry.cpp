#include "catalog/tantivy_index_catalog_entry.h"

#include "common/serializer/buffer_reader.h"
#include "common/serializer/buffer_writer.h"
#include "common/serializer/deserializer.h"
#include "common/serializer/serializer.h"

namespace rag3db {
namespace tantivy_fts_extension {

using namespace common;

std::shared_ptr<BufferWriter> TantivyIndexAuxInfo::serialize() const {
    auto writer = std::make_shared<BufferWriter>();
    auto ser = Serializer(writer);
    ser.serializeValue(indexPath);
    ser.serializeValue(schemaJson);
    ser.serializeValue(stemmer);
    return writer;
}

std::unique_ptr<TantivyIndexAuxInfo> TantivyIndexAuxInfo::deserialize(
    std::unique_ptr<BufferReader> reader) {
    Deserializer deSer{std::move(reader)};
    std::string indexPath, schemaJson, stemmer;
    deSer.deserializeValue(indexPath);
    deSer.deserializeValue(schemaJson);
    deSer.deserializeValue(stemmer);
    return std::make_unique<TantivyIndexAuxInfo>(
        std::move(indexPath), std::move(schemaJson), std::move(stemmer));
}

std::unique_ptr<catalog::IndexAuxInfo> TantivyIndexAuxInfo::copy() {
    return std::make_unique<TantivyIndexAuxInfo>(indexPath, schemaJson, stemmer);
}

std::string TantivyIndexAuxInfo::toCypher(const catalog::IndexCatalogEntry& indexEntry,
    const catalog::ToCypherInfo&) const {
    auto tableName = indexEntry.getIndexName();
    return stringFormat(
        "CALL CREATE_TANTIVY_INDEX('{}', '{}', stemmer := '{}');", tableName, tableName, stemmer);
}

} // namespace tantivy_fts_extension
} // namespace rag3db
