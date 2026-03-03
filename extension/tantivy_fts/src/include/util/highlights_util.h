#pragma once

#include <string>

#include "bridge.rs.h"

namespace rag3db {
namespace tantivy_fts_extension {

inline std::string highlightsToJson(const rust::Vec<FieldHighlights>& highlights) {
    if (highlights.empty()) return "{}";
    std::string json = "{";
    for (size_t i = 0; i < highlights.size(); i++) {
        if (i > 0) json += ",";
        json += "\"";
        json += std::string(highlights[i].field_name);
        json += "\":[";
        for (size_t j = 0; j < highlights[i].ranges.size(); j++) {
            if (j > 0) json += ",";
            json += "[";
            json += std::to_string(highlights[i].ranges[j].start);
            json += ",";
            json += std::to_string(highlights[i].ranges[j].end);
            json += "]";
        }
        json += "]";
    }
    json += "}";
    return json;
}

} // namespace tantivy_fts_extension
} // namespace rag3db
