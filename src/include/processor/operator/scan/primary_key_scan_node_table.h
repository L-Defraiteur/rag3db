#pragma once

#include <unordered_set>

#include "expression_evaluator/expression_evaluator.h"
#include "processor/operator/scan/scan_node_table.h"

namespace rag3db {
namespace processor {

struct PrimaryKeyScanPrintInfo final : OPPrintInfo {
    binder::expression_vector expressions;
    std::string key;
    std::string alias;

    PrimaryKeyScanPrintInfo(binder::expression_vector expressions, std::string key,
        std::string alias)
        : expressions(std::move(expressions)), key(std::move(key)), alias{std::move(alias)} {}

    std::string toString() const override;

    std::unique_ptr<OPPrintInfo> copy() const override {
        return std::unique_ptr<PrimaryKeyScanPrintInfo>(new PrimaryKeyScanPrintInfo(*this));
    }

private:
    PrimaryKeyScanPrintInfo(const PrimaryKeyScanPrintInfo& other)
        : OPPrintInfo(other), expressions(other.expressions), alias(other.alias) {}
};

struct PrimaryKeyScanSharedState {
    std::mutex mtx;

    common::idx_t numTables;
    common::idx_t cursor;

    explicit PrimaryKeyScanSharedState(common::idx_t numTables) : numTables{numTables}, cursor{0} {}

    common::idx_t getTableIdx();
};

class PrimaryKeyScanNodeTable : public ScanTable {
    static constexpr PhysicalOperatorType type_ = PhysicalOperatorType::PRIMARY_KEY_SCAN_NODE_TABLE;

public:
    PrimaryKeyScanNodeTable(ScanOpInfo opInfo, std::vector<ScanNodeTableInfo> tableInfos,
        std::unique_ptr<evaluator::ExpressionEvaluator> indexEvaluator,
        std::shared_ptr<PrimaryKeyScanSharedState> sharedState, physical_op_id id,
        std::unique_ptr<OPPrintInfo> printInfo, bool isList = false)
        : ScanTable{type_, std::move(opInfo), id, std::move(printInfo)}, scanState{nullptr},
          tableInfos{std::move(tableInfos)}, indexEvaluator{std::move(indexEvaluator)},
          sharedState{std::move(sharedState)}, isList{isList} {}

    bool isSource() const override { return true; }

    void initLocalStateInternal(ResultSet*, ExecutionContext*) override;

    bool getNextTuplesInternal(ExecutionContext* context) override;

    bool isParallel() const override { return false; }

    std::unique_ptr<PhysicalOperator> copy() override {
        return std::make_unique<PrimaryKeyScanNodeTable>(opInfo.copy(), copyVector(tableInfos),
            indexEvaluator->copy(), sharedState, id, printInfo->copy(), isList);
    }

private:
    // Le mode liste (un IN sur la clé primaire) : la liste évaluée une fois, parcourue d'un appel
    // au suivant, une ligne par clé trouvée, chaque nœud une fois.
    bool getNextTupleFromList(ExecutionContext* context);

    std::unique_ptr<storage::NodeTableScanState> scanState;
    std::vector<ScanNodeTableInfo> tableInfos;
    std::unique_ptr<evaluator::ExpressionEvaluator> indexEvaluator;
    std::shared_ptr<PrimaryKeyScanSharedState> sharedState;
    bool isList;
    bool listStarted = false;
    common::idx_t listTableIdx = 0;
    common::ValueVector* listVector = nullptr;
    common::list_entry_t listEntry{};
    uint64_t listCursor = 0;
    std::unordered_set<common::offset_t> listSeen;
};

} // namespace processor
} // namespace rag3db
