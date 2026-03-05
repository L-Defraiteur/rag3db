#pragma once

#include "catalog/catalog_entry/index_catalog_entry.h"

namespace rag3db {
namespace lucivy_fts_extension {

struct LucivyIndexCatalogEntry {
    static constexpr char TYPE_NAME[] = "LUCIVY";
};

struct LucivyIndexAuxInfo final : catalog::IndexAuxInfo {
    std::string indexPath;
    std::string schemaJson;
    std::string stemmer;

    LucivyIndexAuxInfo(std::string indexPath, std::string schemaJson, std::string stemmer)
        : indexPath{std::move(indexPath)}, schemaJson{std::move(schemaJson)},
          stemmer{std::move(stemmer)} {}

    std::shared_ptr<common::BufferWriter> serialize() const override;
    static std::unique_ptr<LucivyIndexAuxInfo> deserialize(
        std::unique_ptr<common::BufferReader> reader);

    std::unique_ptr<IndexAuxInfo> copy() override;

    std::string toCypher(const catalog::IndexCatalogEntry& indexEntry,
        const catalog::ToCypherInfo& info) const override;
};

} // namespace lucivy_fts_extension
} // namespace rag3db
