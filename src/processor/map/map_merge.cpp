#include "planner/operator/persistent/logical_merge.h"
#include "processor/operator/persistent/merge.h"
#include "processor/plan_mapper.h"
#include <processor/expression_mapper.h>

using namespace rag3db::planner;

namespace rag3db {
namespace processor {

static FactorizedTableSchema getFactorizedTableSchema(const binder::expression_vector& keys,
    uint64_t numNodeInsertExecutors, uint64_t numRelInsertExecutors) {
    auto tableSchema = FactorizedTableSchema();
    auto isUnFlat = false;
    auto groupID = 0u;
    for (auto& key : keys) {
        auto size = common::LogicalTypeUtils::getRowLayoutSize(key->dataType);
        tableSchema.appendColumn(ColumnSchema(isUnFlat, groupID, size));
    }
    auto numNodeIDFields = numNodeInsertExecutors + numRelInsertExecutors;
    for (auto i = 0u; i < numNodeIDFields; i++) {
        tableSchema.appendColumn(ColumnSchema(isUnFlat, groupID, sizeof(common::nodeID_t)));
    }
    tableSchema.appendColumn(ColumnSchema(isUnFlat, groupID, sizeof(common::hash_t)));
    return tableSchema;
}

std::unique_ptr<PhysicalOperator> PlanMapper::mapMerge(const LogicalOperator* logicalOperator) {
    auto& logicalMerge = logicalOperator->constCast<LogicalMerge>();
    auto outSchema = logicalMerge.getSchema();
    auto inSchema = logicalMerge.getChild(0)->getSchema();
    auto prevOperator = mapOperator(logicalOperator->getChild(0).get());
    auto existenceMarkPos = getDataPos(*logicalMerge.getExistenceMark(), *inSchema);
    std::vector<NodeInsertExecutor> nodeInsertExecutors;
    for (auto& info : logicalMerge.getInsertNodeInfos()) {
        nodeInsertExecutors.push_back(getNodeInsertExecutor(&info, *inSchema, *outSchema));
    }
    std::vector<RelInsertExecutor> relInsertExecutors;
    for (auto& info : logicalMerge.getInsertRelInfos()) {
        relInsertExecutors.push_back(getRelInsertExecutor(&info, *inSchema, *outSchema));
    }
    std::vector<std::unique_ptr<NodeSetExecutor>> onCreateNodeSetExecutors;
    for (auto& info : logicalMerge.getOnCreateSetNodeInfos()) {
        onCreateNodeSetExecutors.push_back(getNodeSetExecutor(info, *inSchema));
    }
    std::vector<std::unique_ptr<RelSetExecutor>> onCreateRelSetExecutors;
    for (auto& info : logicalMerge.getOnCreateSetRelInfos()) {
        onCreateRelSetExecutors.push_back(getRelSetExecutor(info, *inSchema));
    }
    // Une colonne d'identifiant par insertion, dans la table des motifs créés. Avant, il y en
    // avait une par ON MATCH SET : deux SET sur le même motif se disputaient la colonne de
    // leur insertion (emplace ne remplace pas), et la seconde n'était jamais écrite ; et une
    // clause SET hors du MERGE ne retrouvait pas l'identifiant d'un motif créé dans le lot.
    auto numNodeInserts = logicalMerge.getInsertNodeInfos().size();
    auto numRelInserts = logicalMerge.getInsertRelInfos().size();
    common::executor_info executorInfo;
    for (auto j = 0u; j < numNodeInserts + numRelInserts; j++) {
        executorInfo.emplace(j, j);
    }
    std::vector<common::executor_id_t> onMatchInsertIdx;
    std::vector<std::unique_ptr<NodeSetExecutor>> onMatchNodeSetExecutors;
    for (auto& info : logicalMerge.getOnMatchSetNodeInfos()) {
        auto insertIdx = MergeInfo::INVALID_EXECUTOR;
        for (auto j = 0u; j < numNodeInserts; j++) {
            if (*info.pattern == *logicalMerge.getInsertNodeInfos()[j].pattern) {
                insertIdx = j;
            }
        }
        onMatchInsertIdx.push_back(insertIdx);
        onMatchNodeSetExecutors.push_back(getNodeSetExecutor(info, *inSchema));
    }
    std::vector<std::unique_ptr<RelSetExecutor>> onMatchRelSetExecutors;
    for (auto& info : logicalMerge.getOnMatchSetRelInfos()) {
        auto insertIdx = MergeInfo::INVALID_EXECUTOR;
        for (auto j = 0u; j < numRelInserts; j++) {
            if (*info.pattern == *logicalMerge.getInsertRelInfos()[j].pattern) {
                insertIdx = numNodeInserts + j;
            }
        }
        onMatchInsertIdx.push_back(insertIdx);
        onMatchRelSetExecutors.push_back(getRelSetExecutor(info, *inSchema));
    }
    binder::expression_vector expressions;
    for (auto& info : logicalMerge.getInsertNodeInfos()) {
        for (auto& expr : info.columnExprs) {
            expressions.push_back(expr);
        }
    }
    for (auto& info : logicalMerge.getInsertRelInfos()) {
        for (auto& expr : info.columnExprs) {
            expressions.push_back(expr);
        }
    }
    std::vector<binder::expression_pair> onCreateOperation;
    for (auto& info : logicalMerge.getOnCreateSetRelInfos()) {
        onCreateOperation.emplace_back(info.column, info.columnData);
    }
    for (auto& info : logicalMerge.getOnCreateSetNodeInfos()) {
        onCreateOperation.emplace_back(info.column, info.columnData);
    }
    std::vector<binder::expression_pair> onMatchOperation;
    for (auto& info : logicalMerge.getOnMatchSetRelInfos()) {
        onMatchOperation.emplace_back(info.column, info.columnData);
    }
    for (auto& info : logicalMerge.getOnMatchSetNodeInfos()) {
        onMatchOperation.emplace_back(info.column, info.columnData);
    }
    auto printInfo =
        std::make_unique<MergePrintInfo>(expressions, onCreateOperation, onMatchOperation);
    std::vector<std::unique_ptr<evaluator::ExpressionEvaluator>> keyEvaluators;
    auto expressionMapper = ExpressionMapper(inSchema);
    for (auto& key : logicalMerge.getKeys()) {
        keyEvaluators.push_back(expressionMapper.getEvaluator(key));
    }

    MergeInfo mergeInfo{std::move(keyEvaluators),
        getFactorizedTableSchema(logicalMerge.getKeys(), numNodeInserts, numRelInserts),
        std::move(executorInfo), std::move(onMatchInsertIdx), existenceMarkPos};
    return std::make_unique<Merge>(std::move(nodeInsertExecutors), std::move(relInsertExecutors),
        std::move(onCreateNodeSetExecutors), std::move(onCreateRelSetExecutors),
        std::move(onMatchNodeSetExecutors), std::move(onMatchRelSetExecutors), std::move(mergeInfo),
        std::move(prevOperator), getOperatorID(), std::move(printInfo));
}

} // namespace processor
} // namespace rag3db
