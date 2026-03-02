#include "processor/operator/scan/index_scan_node_table.h"

#include "processor/execution_context.h"

using namespace rag3db::common;
using namespace rag3db::storage;

namespace rag3db {
namespace processor {

void IndexScanNodeTable::initLocalStateInternal(ResultSet* resultSet, ExecutionContext* context) {
    ScanTable::initLocalStateInternal(resultSet, context);
    auto nodeIDVector = resultSet->getValueVector(opInfo.nodeIDPos).get();
    scanState = std::make_unique<NodeTableScanState>(nodeIDVector, std::vector<ValueVector*>{},
        nodeIDVector->state);
    // Execute the index search.
    results = searchFunc(limit);
    cursor = 0;
}

bool IndexScanNodeTable::getNextTuplesInternal(ExecutionContext* context) {
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

        // Fill virtual vectors.
        // Must clear null flag because lookup sets INVALID_COLUMN_ID vectors to all-null.
        // virtualVectorIndices[0] = score (always present).
        if (virtualVectorIndices.size() > 0) {
            outVectors[virtualVectorIndices[0]]->setNull(pos, false);
            outVectors[virtualVectorIndices[0]]->setValue<double>(pos, result.score);
        }
        // virtualVectorIndices[1] = metadata (optional, e.g., highlights).
        if (virtualVectorIndices.size() > 1) {
            outVectors[virtualVectorIndices[1]]->setNull(pos, false);
            outVectors[virtualVectorIndices[1]]->setValue(pos, result.metadata);
        }

        metrics->numOutputTuple.incrementByOne();
        return true;
    }
    return false;
}

} // namespace processor
} // namespace rag3db
