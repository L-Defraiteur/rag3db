#pragma once

#include "common/fts_types.h"
#include "processor/operator/scan/scan_node_table.h"

namespace rag3db {
namespace processor {

struct FTSScanPrintInfo final : OPPrintInfo {
    std::string tableName;

    explicit FTSScanPrintInfo(std::string tableName) : tableName{std::move(tableName)} {}

    std::string toString() const override { return "FTS Scan: " + tableName; }

    std::unique_ptr<OPPrintInfo> copy() const override {
        return std::make_unique<FTSScanPrintInfo>(tableName);
    }
};

class FTSScanNodeTable : public ScanTable {
    static constexpr PhysicalOperatorType type_ = PhysicalOperatorType::FTS_SCAN_NODE_TABLE;

public:
    FTSScanNodeTable(ScanOpInfo opInfo, ScanNodeTableInfo tableInfo,
        FTSSearchFunc searchFunc, int64_t limit,
        common::idx_t scoreVectorIdx, common::idx_t highlightsVectorIdx,
        physical_op_id id, std::unique_ptr<OPPrintInfo> printInfo)
        : ScanTable{type_, std::move(opInfo), id, std::move(printInfo)},
          tableInfo{std::move(tableInfo)}, searchFunc{std::move(searchFunc)},
          limit{limit}, scoreVectorIdx{scoreVectorIdx},
          highlightsVectorIdx{highlightsVectorIdx} {}

    bool isSource() const override { return true; }
    bool isParallel() const override { return false; }

    void initLocalStateInternal(ResultSet*, ExecutionContext*) override;
    bool getNextTuplesInternal(ExecutionContext* context) override;

    std::unique_ptr<PhysicalOperator> copy() override {
        return std::make_unique<FTSScanNodeTable>(opInfo.copy(), tableInfo.copy(),
            searchFunc, limit, scoreVectorIdx, highlightsVectorIdx,
            id, printInfo->copy());
    }

private:
    ScanNodeTableInfo tableInfo;
    FTSSearchFunc searchFunc;
    int64_t limit;
    common::idx_t scoreVectorIdx;
    common::idx_t highlightsVectorIdx;

    // Search results, populated at init time.
    std::vector<FTSSearchResult> results;
    common::offset_t cursor = 0;
    std::unique_ptr<storage::NodeTableScanState> scanState;
};

} // namespace processor
} // namespace rag3db
