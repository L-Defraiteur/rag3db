#include "catalog/spatial_index_catalog_entry.h"

#include "common/serializer/buffer_writer.h"
#include "common/serializer/deserializer.h"
#include "common/serializer/serializer.h"

namespace rag3db {
namespace geo_extension {

std::shared_ptr<common::BufferWriter> SpatialIndexAuxInfo::serialize() const {
    auto bufferWriter = std::make_shared<common::BufferWriter>();
    auto serializer = common::Serializer(bufferWriter);
    serializer.write<uint32_t>(dimensions);
    serializer.write<std::string>(metric);
    serializer.write<uint32_t>(static_cast<uint32_t>(columnNames.size()));
    for (const auto& name : columnNames) {
        serializer.write<std::string>(name);
    }
    return bufferWriter;
}

std::unique_ptr<SpatialIndexAuxInfo> SpatialIndexAuxInfo::deserialize(
    std::unique_ptr<common::BufferReader> reader) {
    common::Deserializer deSer{std::move(reader)};
    uint32_t dims;
    std::string metric;
    uint32_t numCols;
    deSer.deserializeValue<uint32_t>(dims);
    deSer.deserializeValue<std::string>(metric);
    deSer.deserializeValue<uint32_t>(numCols);
    std::vector<std::string> columnNames;
    columnNames.reserve(numCols);
    for (uint32_t i = 0; i < numCols; i++) {
        std::string name;
        deSer.deserializeValue<std::string>(name);
        columnNames.push_back(std::move(name));
    }
    return std::make_unique<SpatialIndexAuxInfo>(dims, std::move(columnNames), std::move(metric));
}

std::string SpatialIndexAuxInfo::toCypher(const catalog::IndexCatalogEntry& /*indexEntry*/,
    const catalog::ToCypherInfo& /*info*/) const {
    return ""; // Not exportable as Cypher for now.
}

} // namespace geo_extension
} // namespace rag3db
