#pragma once

#include "catalog/catalog_entry/index_catalog_entry.h"

namespace rag3db {
namespace geo_extension {

struct SpatialIndexAuxInfo final : catalog::IndexAuxInfo {
    uint32_t dimensions;
    std::vector<std::string> columnNames; // e.g. ["lat", "lon"] or ["x", "y", "z"]
    std::string metric;                   // "euclidean" or "haversine"

    SpatialIndexAuxInfo(uint32_t dimensions, std::vector<std::string> columnNames,
        std::string metric)
        : dimensions{dimensions}, columnNames{std::move(columnNames)}, metric{std::move(metric)} {}

    SpatialIndexAuxInfo(const SpatialIndexAuxInfo& other)
        : dimensions{other.dimensions}, columnNames{other.columnNames}, metric{other.metric} {}

    std::shared_ptr<common::BufferWriter> serialize() const override;
    static std::unique_ptr<SpatialIndexAuxInfo> deserialize(
        std::unique_ptr<common::BufferReader> reader);

    std::unique_ptr<IndexAuxInfo> copy() override {
        return std::make_unique<SpatialIndexAuxInfo>(*this);
    }

    std::string toCypher(const catalog::IndexCatalogEntry& indexEntry,
        const catalog::ToCypherInfo& info) const override;
};

struct SpatialIndexCatalogEntry {
    static constexpr char TYPE_NAME[] = "SPATIAL";
};

} // namespace geo_extension
} // namespace rag3db
