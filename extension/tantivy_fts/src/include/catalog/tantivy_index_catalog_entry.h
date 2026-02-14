#pragma once

#include "catalog/catalog_entry/index_catalog_entry.h"

namespace rag3db {
namespace tantivy_fts_extension {

struct TantivyIndexCatalogEntry {
    static constexpr char TYPE_NAME[] = "TANTIVY";
};

struct TantivyIndexAuxInfo final : catalog::IndexAuxInfo {
    std::string indexPath;
    std::string schemaJson;
    std::string stemmer;

    TantivyIndexAuxInfo(std::string indexPath, std::string schemaJson, std::string stemmer)
        : indexPath{std::move(indexPath)}, schemaJson{std::move(schemaJson)},
          stemmer{std::move(stemmer)} {}

    std::shared_ptr<common::BufferWriter> serialize() const override;
    static std::unique_ptr<TantivyIndexAuxInfo> deserialize(
        std::unique_ptr<common::BufferReader> reader);

    std::unique_ptr<IndexAuxInfo> copy() override;

    std::string toCypher(const catalog::IndexCatalogEntry& indexEntry,
        const catalog::ToCypherInfo& info) const override;
};

} // namespace tantivy_fts_extension
} // namespace rag3db
