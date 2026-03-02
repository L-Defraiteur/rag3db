#pragma once

#include <cstdint>
#include <functional>
#include <string>
#include <vector>

#include "common/types/types.h"
#include "function/function.h"

namespace rag3db {

struct IndexSearchResult {
    common::offset_t nodeOffset;
    double score;
    std::string metadata; // Optional (e.g., highlights JSON). Empty for vector/sparse.
};

using IndexSearchFunc = std::function<std::vector<IndexSearchResult>(int64_t limit)>;

// Specifies a virtual expression to create for an index scan (e.g., SEARCH_SCORE, VECTOR_DISTANCE).
struct VirtualExprSpec {
    std::string functionName; // e.g., "SEARCH_SCORE", "VECTOR_DISTANCE"
    common::LogicalType type; // e.g., DOUBLE, STRING

    VirtualExprSpec(std::string functionName, common::LogicalType type)
        : functionName{std::move(functionName)}, type{std::move(type)} {}

    VirtualExprSpec copy() const { return VirtualExprSpec{functionName, type.copy()}; }
};

// Base bind data for index scan scalar functions. Defined in core so the optimizer can
// downcast without knowing the extension's full bind data type.
struct IndexSearchBindData : function::FunctionBindData {
    IndexSearchFunc searchFunc;
    std::vector<VirtualExprSpec> virtualExprSpecs;

    IndexSearchBindData(common::LogicalType resultType, IndexSearchFunc searchFunc,
        std::vector<VirtualExprSpec> virtualExprSpecs)
        : FunctionBindData{std::move(resultType)}, searchFunc{std::move(searchFunc)},
          virtualExprSpecs{std::move(virtualExprSpecs)} {}

    std::unique_ptr<function::FunctionBindData> copy() const override {
        std::vector<VirtualExprSpec> specsCopy;
        specsCopy.reserve(virtualExprSpecs.size());
        for (auto& spec : virtualExprSpecs) {
            specsCopy.push_back(spec.copy());
        }
        return std::make_unique<IndexSearchBindData>(
            resultType.copy(), searchFunc, std::move(specsCopy));
    }
};

// Backward-compatible aliases for existing code that uses FTS names.
using FTSSearchResult = IndexSearchResult;
using FTSSearchFunc = IndexSearchFunc;
using FTSSearchBindData = IndexSearchBindData;

} // namespace rag3db
