#include "processor/operator/persistent/insert_executor.h"

#include "transaction/transaction.h"

using namespace rag3db::common;
using namespace rag3db::transaction;

namespace rag3db {
namespace processor {

void NodeInsertInfo::init(const ResultSet& resultSet) {
    nodeIDVector = resultSet.getValueVector(nodeIDPos).get();
    for (auto& pos : columnsPos) {
        if (pos.isValid()) {
            columnVectors.push_back(resultSet.getValueVector(pos).get());
        } else {
            columnVectors.push_back(nullptr);
        }
    }
}

void NodeInsertInfo::updateNodeID(nodeID_t nodeID) const {
    KU_ASSERT(nodeIDVector->state->getSelVector().getSelSize() == 1);
    auto pos = nodeIDVector->state->getSelVector()[0];
    nodeIDVector->setNull(pos, false);
    nodeIDVector->setValue<nodeID_t>(pos, nodeID);
}

nodeID_t NodeInsertInfo::getNodeID() const {
    auto& nodeIDSelVector = nodeIDVector->state->getSelVector();
    KU_ASSERT(nodeIDSelVector.getSelSize() == 1);
    if (nodeIDVector->isNull(nodeIDSelVector[0])) {
        return {INVALID_OFFSET, INVALID_TABLE_ID};
    }
    return nodeIDVector->getValue<nodeID_t>(nodeIDSelVector[0]);
}

void NodeTableInsertInfo::init(const ResultSet& resultSet, main::ClientContext* context) {
    for (auto& evaluator : columnDataEvaluators) {
        evaluator->init(resultSet, context);
        columnDataVectors.push_back(evaluator->resultVector.get());
    }
    pkVector = columnDataVectors[table->getPKColumnID()];
}

void NodeInsertExecutor::init(ResultSet* resultSet, const ExecutionContext* context) {
    info.init(*resultSet);
    tableInfo.init(*resultSet, context->clientContext);
}

static void writeColumnVector(ValueVector* columnVector, const ValueVector* dataVector) {
    auto& columnSelVector = columnVector->state->getSelVector();
    auto& dataSelVector = dataVector->state->getSelVector();
    KU_ASSERT(columnSelVector.getSelSize() == 1 && dataSelVector.getSelSize() == 1);
    auto columnPos = columnSelVector[0];
    auto dataPos = dataSelVector[0];
    if (dataVector->isNull(dataPos)) {
        columnVector->setNull(columnPos, true);
    } else {
        columnVector->setNull(columnPos, false);
        columnVector->copyFromVectorData(columnPos, dataVector, dataPos);
    }
}

// TODO(Guodong/Xiyang): think we can reference data vector instead of copy.
static void writeColumnVectors(const std::vector<ValueVector*>& columnVectors,
    const std::vector<ValueVector*>& dataVectors) {
    KU_ASSERT(columnVectors.size() == dataVectors.size());
    for (auto i = 0u; i < columnVectors.size(); ++i) {
        if (columnVectors[i] == nullptr) { // No need to project
            continue;
        }
        writeColumnVector(columnVectors[i], dataVectors[i]);
    }
}

static void writeColumnVectorsToNull(const std::vector<ValueVector*>& columnVectors) {
    for (auto i = 0u; i < columnVectors.size(); ++i) {
        auto columnVector = columnVectors[i];
        if (columnVector == nullptr) { // No need to project
            continue;
        }
        auto& columnSelVector = columnVector->state->getSelVector();
        KU_ASSERT(columnSelVector.getSelSize() == 1);
        columnVector->setNull(columnSelVector[0], true);
    }
}

void NodeInsertExecutor::setNodeIDVectorToNonNull() const {
    info.nodeIDVector->setNull(info.nodeIDVector->state->getSelVector()[0], false);
}

nodeID_t NodeInsertExecutor::insert(main::ClientContext* context) {
    for (auto& evaluator : tableInfo.columnDataEvaluators) {
        evaluator->evaluate();
    }
    auto transaction = Transaction::Get(*context);
    if (checkConflict(transaction)) {
        return info.getNodeID();
    }
    auto insertState = std::make_unique<storage::NodeTableInsertState>(*info.nodeIDVector,
        *tableInfo.pkVector, tableInfo.columnDataVectors);
    tableInfo.table->initInsertState(context, *insertState);
    tableInfo.table->insert(transaction, *insertState);
    writeColumnVectors(info.columnVectors, tableInfo.columnDataVectors);
    return info.getNodeID();
}

void NodeInsertExecutor::skipInsert(nodeID_t createdNodeID, main::ClientContext* context) const {
    // Le vecteur d'identifiant gardait celui du dernier nœud inséré : un SET ou un RETURN qui
    // suit aurait lu un autre nœud. On repose celui que le lot a créé pour cette clé.
    info.updateNodeID(createdNodeID);
    std::vector<column_id_t> columnIDs;
    std::vector<ValueVector*> outputVectors;
    for (auto i = 0u; i < info.columnVectors.size(); ++i) {
        if (info.columnVectors[i] != nullptr) {
            columnIDs.push_back(i);
            outputVectors.push_back(info.columnVectors[i]);
        }
    }
    if (outputVectors.empty()) {
        return;
    }
    auto transaction = Transaction::Get(*context);
    storage::NodeTableScanState scanState{info.nodeIDVector, std::move(outputVectors),
        info.nodeIDVector->state};
    scanState.setToTable(transaction, tableInfo.table, std::move(columnIDs));
    tableInfo.table->initScanState(transaction, scanState, createdNodeID.tableID,
        createdNodeID.offset);
    tableInfo.table->lookup(transaction, scanState);
}

bool NodeInsertExecutor::checkConflict(const Transaction* transaction) const {
    if (info.conflictAction == ConflictAction::ON_CONFLICT_DO_NOTHING) {
        auto offset =
            tableInfo.table->validateUniquenessConstraint(transaction, tableInfo.columnDataVectors);
        if (offset != INVALID_OFFSET) {
            // Conflict. Skip insertion.
            info.updateNodeID({offset, tableInfo.table->getTableID()});
            return true;
        }
    }
    return false;
}

void RelInsertInfo::init(const ResultSet& resultSet) {
    srcNodeIDVector = resultSet.getValueVector(srcNodeIDPos).get();
    dstNodeIDVector = resultSet.getValueVector(dstNodeIDPos).get();
    for (auto& pos : columnsPos) {
        if (pos.isValid()) {
            columnVectors.push_back(resultSet.getValueVector(pos).get());
        } else {
            columnVectors.push_back(nullptr);
        }
    }
}

void RelTableInsertInfo::init(const ResultSet& resultSet, main::ClientContext* context) {
    for (auto& evaluator : columnDataEvaluators) {
        evaluator->init(resultSet, context);
        columnDataVectors.push_back(evaluator->resultVector.get());
    }
}

internalID_t RelTableInsertInfo::getRelID() const {
    auto relIDVector = columnDataVectors[0];
    auto& nodeIDSelVector = relIDVector->state->getSelVector();
    KU_ASSERT(nodeIDSelVector.getSelSize() == 1);
    if (relIDVector->isNull(nodeIDSelVector[0])) {
        return {INVALID_OFFSET, INVALID_TABLE_ID};
    }
    return relIDVector->getValue<nodeID_t>(nodeIDSelVector[0]);
}

void RelInsertExecutor::init(ResultSet* resultSet, const ExecutionContext* context) {
    info.init(*resultSet);
    tableInfo.init(*resultSet, context->clientContext);
}

internalID_t RelInsertExecutor::insert(main::ClientContext* context) {
    KU_ASSERT(info.srcNodeIDVector->state->getSelVector().getSelSize() == 1);
    KU_ASSERT(info.dstNodeIDVector->state->getSelVector().getSelSize() == 1);
    auto srcNodeIDPos = info.srcNodeIDVector->state->getSelVector()[0];
    auto dstNodeIDPos = info.dstNodeIDVector->state->getSelVector()[0];
    if (info.srcNodeIDVector->isNull(srcNodeIDPos) || info.dstNodeIDVector->isNull(dstNodeIDPos)) {
        // No need to insert.
        writeColumnVectorsToNull(info.columnVectors);
        return tableInfo.getRelID();
    }
    for (auto i = 1u; i < tableInfo.columnDataEvaluators.size(); ++i) {
        tableInfo.columnDataEvaluators[i]->evaluate();
    }
    auto insertState = std::make_unique<storage::RelTableInsertState>(*info.srcNodeIDVector,
        *info.dstNodeIDVector, tableInfo.columnDataVectors);
    tableInfo.table->initInsertState(context, *insertState);
    tableInfo.table->insert(Transaction::Get(*context), *insertState);
    writeColumnVectors(info.columnVectors, tableInfo.columnDataVectors);
    return tableInfo.getRelID();
}

void RelInsertExecutor::skipInsert(internalID_t createdRelID) const {
    for (auto i = 1u; i < tableInfo.columnDataEvaluators.size(); ++i) {
        tableInfo.columnDataEvaluators[i]->evaluate();
    }
    // La colonne 0 est l'identifiant de la relation, que seule l'insertion remplit : elle
    // gardait celui de la dernière relation insérée, et un SET qui suit visait alors une
    // relation qui ne joint pas ces deux nœuds (mise à jour perdue sans erreur).
    auto relIDVector = tableInfo.columnDataVectors[0];
    auto pos = relIDVector->state->getSelVector()[0];
    relIDVector->setNull(pos, false);
    relIDVector->setValue<internalID_t>(pos, createdRelID);
    writeColumnVectors(info.columnVectors, tableInfo.columnDataVectors);
}

} // namespace processor
} // namespace rag3db
