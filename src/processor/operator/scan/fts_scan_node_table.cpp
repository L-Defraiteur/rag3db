#include "processor/operator/scan/fts_scan_node_table.h"

#include "processor/execution_context.h"

using namespace rag3db::common;
using namespace rag3db::storage;

namespace rag3db {
namespace processor {

void FTSScanNodeTable::initLocalStateInternal(ResultSet* resultSet, ExecutionContext* context) {
    ScanTable::initLocalStateInternal(resultSet, context);
    auto nodeIDVector = resultSet->getValueVector(opInfo.nodeIDPos).get();
    scanState = std::make_unique<NodeTableScanState>(nodeIDVector, std::vector<ValueVector*>{},
        nodeIDVector->state);
    // Execute the FTS search.
    results = searchFunc(limit);
    cursor = 0;
}

bool FTSScanNodeTable::getNextTuplesInternal(ExecutionContext* context) {
    auto transaction = transaction::Transaction::Get(*context->clientContext);
    auto& table = tableInfo.table->cast<NodeTable>();

    while (cursor < results.size()) {
        auto& result = results[cursor++];

        auto nodeID = nodeID_t{result.nodeOffset, table.getTableID()};
        auto pos = scanState->nodeIDVector->state->getSelVector()[0];
        scanState->nodeIDVector->setValue<nodeID_t>(pos, nodeID);

        // Re-initialize scan state each iteration (like PrimaryKeyScan).
        tableInfo.initScanState(*scanState, outVectors, context->clientContext);
        table.initScanState(transaction, *scanState, nodeID.tableID, result.nodeOffset);
        auto lookupOk = table.lookup(transaction, *scanState);
        if (!lookupOk) {
            continue; // Node deleted/invisible — skip.
        }
        tableInfo.castColumns();

        // Fill virtual vectors (score and highlights).
        // Must clear null flag because lookup sets INVALID_COLUMN_ID vectors to all-null.
        outVectors[scoreVectorIdx]->setNull(pos, false);
        outVectors[scoreVectorIdx]->setValue<double>(pos, result.score);
        outVectors[highlightsVectorIdx]->setNull(pos, false);
        outVectors[highlightsVectorIdx]->setValue(pos, result.highlights);

        metrics->numOutputTuple.incrementByOne();
        return true;
    }
    return false;
}

} // namespace processor
} // namespace rag3db
