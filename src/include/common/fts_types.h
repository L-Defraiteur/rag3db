#pragma once

#include <cstdint>
#include <functional>
#include <string>
#include <vector>

#include "common/types/types.h"
#include "function/function.h"

namespace rag3db {

struct FTSSearchResult {
    common::offset_t nodeOffset;
    double score;
    std::string highlights; // JSON string
};

using FTSSearchFunc = std::function<std::vector<FTSSearchResult>(int64_t limit)>;

// Base bind data for FTS scalar functions. Defined in core so the optimizer can
// downcast without knowing the extension's full SearchBindData type.
struct FTSSearchBindData : function::FunctionBindData {
    FTSSearchFunc searchFunc;

    FTSSearchBindData(common::LogicalType resultType, FTSSearchFunc searchFunc)
        : FunctionBindData{std::move(resultType)}, searchFunc{std::move(searchFunc)} {}

    std::unique_ptr<function::FunctionBindData> copy() const override {
        return std::make_unique<FTSSearchBindData>(resultType.copy(), searchFunc);
    }
};

} // namespace rag3db
