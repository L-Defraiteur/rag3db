#include "binder/binder.h"
#include "binder/expression/property_expression.h"
#include "binder/expression_binder.h"
#include "common/mask.h"
#include "planner/operator/scan/logical_scan_node_table.h"
#include "processor/expression_mapper.h"
#include "processor/operator/scan/fts_scan_node_table.h"
#include "processor/operator/scan/primary_key_scan_node_table.h"
#include "processor/operator/scan/scan_node_table.h"
#include "processor/plan_mapper.h"
#include "storage/storage_manager.h"

using namespace rag3db::binder;
using namespace rag3db::common;
using namespace rag3db::planner;

namespace rag3db {
namespace processor {

std::unique_ptr<PhysicalOperator> PlanMapper::mapScanNodeTable(
    const LogicalOperator* logicalOperator) {
    auto storageManager = storage::StorageManager::Get(*clientContext);
    auto catalog = catalog::Catalog::Get(*clientContext);
    auto transaction = transaction::Transaction::Get(*clientContext);
    auto& scan = logicalOperator->constCast<LogicalScanNodeTable>();
    const auto outSchema = scan.getSchema();
    auto nodeIDPos = getDataPos(*scan.getNodeID(), *outSchema);
    std::vector<DataPos> outVectorsPos;
    for (auto& expression : scan.getProperties()) {
        outVectorsPos.emplace_back(getDataPos(*expression, *outSchema));
    }
    auto scanInfo = ScanOpInfo(nodeIDPos, outVectorsPos);
    const auto tableIDs = scan.getTableIDs();
    std::vector<std::string> tableNames;
    std::vector<ScanNodeTableInfo> tableInfos;
    auto binder = Binder(clientContext);
    auto expressionBinder = ExpressionBinder(&binder, clientContext);
    for (const auto& tableID : tableIDs) {
        auto tableEntry = catalog->getTableCatalogEntry(transaction, tableID);
        tableNames.push_back(tableEntry->getName());
        auto table = storageManager->getTable(tableID)->ptrCast<storage::NodeTable>();
        auto tableInfo = ScanNodeTableInfo(table, copyVector(scan.getPropertyPredicates()));
        for (auto& expr : scan.getProperties()) {
            if (expr->expressionType != ExpressionType::PROPERTY) {
                // Virtual property (e.g., FTS score/highlights).
                tableInfo.addColumnInfo(INVALID_COLUMN_ID, ColumnCaster(LogicalType::ANY()));
                continue;
            }
            auto& property = expr->constCast<PropertyExpression>();
            if (property.hasProperty(tableEntry->getTableID())) {
                auto propertyName = property.getPropertyName();
                auto& columnType = tableEntry->getProperty(propertyName).getType();
                auto columnCaster = ColumnCaster(columnType.copy());
                if (property.getDataType() != columnType) {
                    auto columnExpr = std::make_shared<PropertyExpression>(property);
                    columnExpr->dataType = columnType.copy();
                    columnCaster.setCastExpr(
                        expressionBinder.forceCast(columnExpr, property.getDataType()));
                }
                tableInfo.addColumnInfo(tableEntry->getColumnID(propertyName),
                    std::move(columnCaster));
            } else {
                tableInfo.addColumnInfo(INVALID_COLUMN_ID, ColumnCaster(LogicalType::ANY()));
            }
        }
        tableInfos.push_back(std::move(tableInfo));
    }
    std::vector<std::shared_ptr<ScanNodeTableSharedState>> sharedStates;
    for (auto& tableID : tableIDs) {
        auto table = storageManager->getTable(tableID)->ptrCast<storage::NodeTable>();
        auto semiMask = SemiMaskUtil::createMask(table->getNumTotalRows(transaction));
        sharedStates.push_back(std::make_shared<ScanNodeTableSharedState>(std::move(semiMask)));
    }
    auto alias = scan.getNodeID()->cast<PropertyExpression>().getRawVariableName();
    std::unique_ptr<PhysicalOperator> result;
    switch (scan.getScanType()) {
    case LogicalScanNodeTableType::SCAN: {
        auto printInfo =
            std::make_unique<ScanNodeTablePrintInfo>(tableNames, alias, scan.getProperties());
        auto progressSharedState = std::make_shared<ScanNodeTableProgressSharedState>();
        return std::make_unique<ScanNodeTable>(std::move(scanInfo), std::move(tableInfos),
            std::move(sharedStates), getOperatorID(), std::move(printInfo), progressSharedState);
    }
    case LogicalScanNodeTableType::PRIMARY_KEY_SCAN: {
        auto& primaryKeyScanInfo = scan.getExtraInfo()->constCast<PrimaryKeyScanInfo>();
        auto exprMapper = ExpressionMapper(outSchema);
        auto evaluator = exprMapper.getEvaluator(primaryKeyScanInfo.key);
        auto sharedState = std::make_shared<PrimaryKeyScanSharedState>(tableInfos.size());
        auto printInfo = std::make_unique<PrimaryKeyScanPrintInfo>(scan.getProperties(),
            primaryKeyScanInfo.key->toString(), alias);
        return std::make_unique<PrimaryKeyScanNodeTable>(std::move(scanInfo), std::move(tableInfos),
            std::move(evaluator), std::move(sharedState), getOperatorID(), std::move(printInfo));
    }
    case LogicalScanNodeTableType::FTS_SCAN: {
        auto& ftsInfo = scan.getExtraInfo()->constCast<FTSScanInfo>();
        KU_ASSERT(tableInfos.size() == 1);
        // Find virtual property positions (score and highlights in outVectors).
        idx_t scoreVectorIdx = INVALID_IDX;
        idx_t highlightsVectorIdx = INVALID_IDX;
        const auto& properties = scan.getProperties();
        for (idx_t i = 0; i < properties.size(); i++) {
            if (properties[i].get() == ftsInfo.scoreExpr.get()) {
                scoreVectorIdx = i;
            } else if (properties[i].get() == ftsInfo.highlightsExpr.get()) {
                highlightsVectorIdx = i;
            }
        }
        KU_ASSERT(scoreVectorIdx != INVALID_IDX && highlightsVectorIdx != INVALID_IDX);
        auto printInfo = std::make_unique<FTSScanPrintInfo>(tableNames[0]);
        return std::make_unique<FTSScanNodeTable>(std::move(scanInfo), std::move(tableInfos[0]),
            ftsInfo.searchFunc, ftsInfo.limit, scoreVectorIdx, highlightsVectorIdx,
            getOperatorID(), std::move(printInfo));
    }
    default:
        KU_UNREACHABLE;
    }
}

} // namespace processor
} // namespace rag3db
