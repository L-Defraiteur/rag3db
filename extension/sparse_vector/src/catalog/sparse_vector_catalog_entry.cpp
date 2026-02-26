#include "catalog/sparse_vector_catalog_entry.h"

#include "common/serializer/buffer_reader.h"
#include "common/serializer/buffer_writer.h"
#include "common/serializer/deserializer.h"
#include "common/serializer/serializer.h"

namespace rag3db {
namespace sparse_vector_extension {

using namespace common;

std::shared_ptr<BufferWriter> SparseVectorIndexAuxInfo::serialize() const {
    auto writer = std::make_shared<BufferWriter>();
    auto ser = Serializer(writer);
    ser.serializeValue(indexPath);
    ser.serializeValue(indicesColumnName);
    ser.serializeValue(weightsColumnName);
    return writer;
}

std::unique_ptr<SparseVectorIndexAuxInfo> SparseVectorIndexAuxInfo::deserialize(
    std::unique_ptr<BufferReader> reader) {
    Deserializer deSer{std::move(reader)};
    std::string indexPath, indicesCol, weightsCol;
    deSer.deserializeValue(indexPath);
    deSer.deserializeValue(indicesCol);
    deSer.deserializeValue(weightsCol);
    return std::make_unique<SparseVectorIndexAuxInfo>(
        std::move(indexPath), std::move(indicesCol), std::move(weightsCol));
}

std::unique_ptr<catalog::IndexAuxInfo> SparseVectorIndexAuxInfo::copy() {
    return std::make_unique<SparseVectorIndexAuxInfo>(
        indexPath, indicesColumnName, weightsColumnName);
}

std::string SparseVectorIndexAuxInfo::toCypher(const catalog::IndexCatalogEntry& indexEntry,
    const catalog::ToCypherInfo&) const {
    auto tableName = indexEntry.getIndexName();
    return stringFormat(
        "CALL CREATE_SPARSE_VECTOR_INDEX('{}', '{}', '{}');",
        tableName, indicesColumnName, weightsColumnName);
}

} // namespace sparse_vector_extension
} // namespace rag3db
