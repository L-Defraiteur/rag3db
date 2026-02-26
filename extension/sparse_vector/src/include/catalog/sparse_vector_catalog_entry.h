#pragma once

#include "catalog/catalog_entry/index_catalog_entry.h"

namespace rag3db {
namespace sparse_vector_extension {

struct SparseVectorCatalogEntry {
    static constexpr char TYPE_NAME[] = "SPARSE_VECTOR";
};

struct SparseVectorIndexAuxInfo final : catalog::IndexAuxInfo {
    std::string indexPath;
    std::string indicesColumnName;
    std::string weightsColumnName;

    SparseVectorIndexAuxInfo(std::string indexPath, std::string indicesCol,
        std::string weightsCol)
        : indexPath{std::move(indexPath)}, indicesColumnName{std::move(indicesCol)},
          weightsColumnName{std::move(weightsCol)} {}

    std::shared_ptr<common::BufferWriter> serialize() const override;
    static std::unique_ptr<SparseVectorIndexAuxInfo> deserialize(
        std::unique_ptr<common::BufferReader> reader);

    std::unique_ptr<IndexAuxInfo> copy() override;

    std::string toCypher(const catalog::IndexCatalogEntry& indexEntry,
        const catalog::ToCypherInfo& info) const override;
};

} // namespace sparse_vector_extension
} // namespace rag3db
