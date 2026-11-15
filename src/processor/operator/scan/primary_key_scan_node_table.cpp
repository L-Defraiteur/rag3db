#include "processor/operator/scan/primary_key_scan_node_table.h"

#include "binder/expression/expression_util.h"
#include "processor/execution_context.h"

using namespace rag3db::common;
using namespace rag3db::storage;

namespace rag3db {
namespace processor {

std::string PrimaryKeyScanPrintInfo::toString() const {
    std::string result = "Key: ";
    result += key;
    if (!alias.empty()) {
        result += ",Alias: ";
        result += alias;
    }
    result += ", Expressions: ";
    result += binder::ExpressionUtil::toString(expressions);
    return result;
}

idx_t PrimaryKeyScanSharedState::getTableIdx() {
    std::unique_lock lck{mtx};
    if (cursor < numTables) {
        return cursor++;
    }
    return numTables;
}

void PrimaryKeyScanNodeTable::initLocalStateInternal(ResultSet* resultSet,
    ExecutionContext* context) {
    ScanTable::initLocalStateInternal(resultSet, context);
    auto nodeIDVector = resultSet->getValueVector(opInfo.nodeIDPos).get();
    scanState = std::make_unique<NodeTableScanState>(nodeIDVector, std::vector<ValueVector*>{},
        nodeIDVector->state);
    indexEvaluator->init(*resultSet, context->clientContext);
    listStarted = false;
    listVector = nullptr;
    listCursor = 0;
    listSeen.clear();
}

bool PrimaryKeyScanNodeTable::getNextTupleFromList(ExecutionContext* context) {
    auto transaction = transaction::Transaction::Get(*context->clientContext);
    if (!listStarted) {
        listStarted = true;
        listTableIdx = sharedState->getTableIdx();
        if (listTableIdx >= tableInfos.size()) {
            return false;
        }
        indexEvaluator->evaluate();
        listVector = indexEvaluator->resultVector.get();
        const auto pos = listVector->state->getSelVector().getSelectedPositions()[0];
        if (listVector->isNull(pos)) {
            listVector = nullptr;
            return false;
        }
        listEntry = listVector->getValue<list_entry_t>(pos);
        listCursor = 0;
    }
    if (listVector == nullptr || listTableIdx >= tableInfos.size()) {
        return false;
    }
    auto& tableInfo = tableInfos[listTableIdx];
    auto& table = tableInfo.table->cast<NodeTable>();
    auto* keys = ListVector::getDataVector(listVector);
    while (listCursor < listEntry.size) {
        const auto keyPos = listEntry.offset + listCursor++;
        // Un élément NULL ne trouve rien, comme `x IN [..., NULL]` sous un FILTER.
        if (keys->isNull(keyPos)) {
            continue;
        }
        offset_t nodeOffset = 0;
        if (!table.lookupPK(transaction, keys, keyPos, nodeOffset)) {
            continue;
        }
        // Une clé répétée dans la liste ne rend pas deux fois le nœud.
        if (!listSeen.insert(nodeOffset).second) {
            continue;
        }
        const auto nodeID = nodeID_t{nodeOffset, table.getTableID()};
        const auto outPos = scanState->nodeIDVector->state->getSelVector().getSelectedPositions()[0];
        scanState->nodeIDVector->setValue<nodeID_t>(outPos, nodeID);
        tableInfo.initScanState(*scanState, outVectors, context->clientContext);
        table.initScanState(transaction, *scanState, nodeID.tableID, nodeOffset);
        const auto succeeded = table.lookup(transaction, *scanState);
        tableInfo.castColumns();
        if (succeeded) {
            metrics->numOutputTuple.incrementByOne();
            return true;
        }
    }
    return false;
}

bool PrimaryKeyScanNodeTable::getNextTuplesInternal(ExecutionContext* context) {
    if (isList) {
        return getNextTupleFromList(context);
    }
    auto transaction = transaction::Transaction::Get(*context->clientContext);
    auto tableIdx = sharedState->getTableIdx();
    if (tableIdx >= tableInfos.size()) {
        return false;
    }
    KU_ASSERT(tableIdx < tableInfos.size());
    auto& tableInfo = tableInfos[tableIdx];
    // Look up index
    indexEvaluator->evaluate();
    auto indexVector = indexEvaluator->resultVector.get();
    auto& selVector = indexVector->state->getSelVector();
    KU_ASSERT(selVector.getSelSize() == 1);
    auto pos = selVector.getSelectedPositions()[0];
    if (indexVector->isNull(pos)) {
        return false;
    }
    offset_t nodeOffset = 0;
    auto& table = tableInfo.table->cast<NodeTable>();
    if (!table.lookupPK(transaction, indexVector, pos, nodeOffset)) {
        return false;
    }
    auto nodeID = nodeID_t{nodeOffset, table.getTableID()};
    scanState->nodeIDVector->setValue<nodeID_t>(pos, nodeID);
    // Look up properties
    tableInfo.initScanState(*scanState, outVectors, context->clientContext);
    table.initScanState(transaction, *scanState, nodeID.tableID, nodeOffset);
    auto succeeded = table.lookup(transaction, *scanState);
    tableInfo.castColumns();
    metrics->numOutputTuple.incrementByOne();
    return succeeded;
}

} // namespace processor
} // namespace rag3db
