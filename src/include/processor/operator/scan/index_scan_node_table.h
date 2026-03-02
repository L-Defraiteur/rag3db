#pragma once

#include "common/index_search_types.h"
#include "processor/operator/scan/scan_node_table.h"

namespace rag3db {
namespace processor {

struct IndexScanPrintInfo final : OPPrintInfo {
    std::string tableName;

    explicit IndexScanPrintInfo(std::string tableName) : tableName{std::move(tableName)} {}

    std::string toString() const override { return "Index Scan: " + tableName; }

    std::unique_ptr<OPPrintInfo> copy() const override {
        return std::make_unique<IndexScanPrintInfo>(tableName);
    }
};

class IndexScanNodeTable : public ScanTable {
    static constexpr PhysicalOperatorType type_ = PhysicalOperatorType::INDEX_SCAN_NODE_TABLE;

public:
    IndexScanNodeTable(ScanOpInfo opInfo, ScanNodeTableInfo tableInfo,
        IndexSearchFunc searchFunc, int64_t limit,
        std::vector<common::idx_t> virtualVectorIndices,
        physical_op_id id, std::unique_ptr<OPPrintInfo> printInfo)
        : ScanTable{type_, std::move(opInfo), id, std::move(printInfo)},
          tableInfo{std::move(tableInfo)}, searchFunc{std::move(searchFunc)},
          limit{limit}, virtualVectorIndices{std::move(virtualVectorIndices)} {}

    bool isSource() const override { return true; }
    bool isParallel() const override { return false; }

    void initLocalStateInternal(ResultSet*, ExecutionContext*) override;
    bool getNextTuplesInternal(ExecutionContext* context) override;

    std::unique_ptr<PhysicalOperator> copy() override {
        return std::make_unique<IndexScanNodeTable>(opInfo.copy(), tableInfo.copy(),
            searchFunc, limit, virtualVectorIndices,
            id, printInfo->copy());
    }

private:
    ScanNodeTableInfo tableInfo;
    IndexSearchFunc searchFunc;
    int64_t limit;
    // Indices into outVectors for virtual columns.
    // [0] = score (DOUBLE), [1] = metadata (STRING, optional).
    std::vector<common::idx_t> virtualVectorIndices;

    std::vector<IndexSearchResult> results;
    common::offset_t cursor = 0;
    std::unique_ptr<storage::NodeTableScanState> scanState;
};

} // namespace processor
} // namespace rag3db
