#include "optimizer/filter_push_down_optimizer.h"

#include "binder/expression/literal_expression.h"
#include "binder/expression/property_expression.h"
#include "binder/expression/scalar_function_expression.h"
#include "binder/expression/variable_expression.h"
#include "common/fts_types.h"
#include "main/client_context.h"
#include "planner/operator/extend/logical_extend.h"
#include "planner/operator/logical_empty_result.h"
#include "planner/operator/logical_filter.h"
#include "planner/operator/logical_hash_join.h"
#include "planner/operator/logical_order_by.h"
#include "planner/operator/logical_projection.h"
#include "planner/operator/logical_table_function_call.h"
#include "planner/operator/scan/logical_scan_node_table.h"

using namespace rag3db::binder;
using namespace rag3db::common;
using namespace rag3db::planner;
using namespace rag3db::storage;

namespace rag3db {
namespace optimizer {

void FilterPushDownOptimizer::rewrite(LogicalPlan* plan) {
    visitOperator(plan->getLastOperator());
    resolveSearchExpressions(plan->getLastOperator().get());
}

std::shared_ptr<LogicalOperator> FilterPushDownOptimizer::visitOperator(
    const std::shared_ptr<LogicalOperator>& op) {
    switch (op->getOperatorType()) {
    case LogicalOperatorType::FILTER: {
        return visitFilterReplace(op);
    }
    case LogicalOperatorType::CROSS_PRODUCT: {
        return visitCrossProductReplace(op);
    }
    case LogicalOperatorType::EXTEND: {
        return visitExtendReplace(op);
    }
    case LogicalOperatorType::SCAN_NODE_TABLE: {
        return visitScanNodeTableReplace(op);
    }
    case LogicalOperatorType::TABLE_FUNCTION_CALL: {
        return visitTableFunctionCallReplace(op);
    }
    default: { // Stop current push down for unhandled operator.
        return visitChildren(op);
    }
    }
}

std::shared_ptr<LogicalOperator> FilterPushDownOptimizer::visitChildren(
    const std::shared_ptr<LogicalOperator>& op) {
    for (auto i = 0u; i < op->getNumChildren(); ++i) {
        // Start new push down for child.
        auto optimizer = FilterPushDownOptimizer(context);
        op->setChild(i, optimizer.visitOperator(op->getChild(i)));
    }
    op->computeFlatSchema();
    return finishPushDown(op);
}

std::shared_ptr<LogicalOperator> FilterPushDownOptimizer::visitFilterReplace(
    const std::shared_ptr<LogicalOperator>& op) {
    auto& filter = op->constCast<LogicalFilter>();
    auto predicate = filter.getPredicate();
    if (predicate->expressionType == ExpressionType::LITERAL) {
        // Avoid executing child plan if literal is Null or False.
        auto& literalExpr = predicate->constCast<LiteralExpression>();
        if (literalExpr.isNull() || !literalExpr.getValue().getValue<bool>()) {
            return std::make_shared<LogicalEmptyResult>(*op->getSchema());
        }
        // Ignore if literal is True.
    } else {
        predicateSet.addPredicate(predicate);
    }
    return visitOperator(filter.getChild(0));
}

std::shared_ptr<LogicalOperator> FilterPushDownOptimizer::visitCrossProductReplace(
    const std::shared_ptr<LogicalOperator>& op) {
    auto remainingPSet = PredicateSet();
    auto probePSet = PredicateSet();
    auto buildPSet = PredicateSet();
    for (auto& p : predicateSet.getAllPredicates()) {
        auto inProbe = op->getChild(0)->getSchema()->evaluable(*p);
        auto inBuild = op->getChild(1)->getSchema()->evaluable(*p);
        if (inProbe && !inBuild) {
            probePSet.addPredicate(p);
        } else if (!inProbe && inBuild) {
            buildPSet.addPredicate(p);
        } else {
            remainingPSet.addPredicate(p);
        }
    }
    KU_ASSERT(op->getNumChildren() == 2);
    // Push probe side
    auto probeOptimizer = FilterPushDownOptimizer(context, std::move(probePSet));
    op->setChild(0, probeOptimizer.visitOperator(op->getChild(0)));
    // Push build side
    auto buildOptimizer = FilterPushDownOptimizer(context, std::move(buildPSet));
    op->setChild(1, buildOptimizer.visitOperator(op->getChild(1)));

    auto probeSchema = op->getChild(0)->getSchema();
    auto buildSchema = op->getChild(1)->getSchema();
    expression_vector predicates;
    std::vector<join_condition_t> joinConditions;
    for (auto& predicate : remainingPSet.equalityPredicates) {
        auto left = predicate->getChild(0);
        auto right = predicate->getChild(1);
        // TODO(Xiyang): this can only rewrite left = right, we should also be able to do
        // expr(left), expr(right)
        if (probeSchema->isExpressionInScope(*left) && buildSchema->isExpressionInScope(*right)) {
            joinConditions.emplace_back(left, right);
        } else if (probeSchema->isExpressionInScope(*right) &&
                   buildSchema->isExpressionInScope(*left)) {
            joinConditions.emplace_back(right, left);
        } else {
            // Collect predicates that cannot be rewritten as join conditions.
            predicates.push_back(predicate);
        }
    }
    if (joinConditions.empty()) { // Nothing to push down. Terminate.
        return finishPushDown(op);
    }
    auto hashJoin = std::make_shared<LogicalHashJoin>(joinConditions, JoinType::INNER,
        nullptr /* mark */, op->getChild(0), op->getChild(1), 0 /* cardinality */);
    // For non-id based joins, we disable side way information passing.
    hashJoin->getSIPInfoUnsafe().position = SemiMaskPosition::PROHIBIT;
    hashJoin->computeFlatSchema();
    // Apply remaining predicates.
    predicates.insert(predicates.end(), remainingPSet.nonEqualityPredicates.begin(),
        remainingPSet.nonEqualityPredicates.end());
    if (predicates.empty()) {
        return hashJoin;
    }
    return appendFilters(predicates, hashJoin);
}

static ColumnPredicateSet getPredicateSet(const Expression& column,
    const binder::expression_vector& predicates) {
    auto predicateSet = ColumnPredicateSet();
    for (auto& predicate : predicates) {
        auto columnPredicate = ColumnPredicateUtil::tryConvert(column, *predicate);
        if (columnPredicate == nullptr) {
            continue;
        }
        predicateSet.addPredicate(std::move(columnPredicate));
    }
    return predicateSet;
}

static std::vector<ColumnPredicateSet> getColumnPredicateSets(const expression_vector& columns,
    const expression_vector& predicates) {
    std::vector<ColumnPredicateSet> predicateSets;
    for (auto& column : columns) {
        predicateSets.push_back(getPredicateSet(*column, predicates));
    }
    return predicateSets;
}

static bool isConstantExpression(const std::shared_ptr<Expression> expression) {
    switch (expression->expressionType) {
    case ExpressionType::LITERAL:
    case ExpressionType::PARAMETER: {
        return true;
    }
    // TODO(Xiyang): fold parameter expression in binder.
    case ExpressionType::FUNCTION: {
        auto& func = expression->constCast<ScalarFunctionExpression>();
        if (func.getFunction().name == "CAST") {
            return isConstantExpression(func.getChild(0));
        } else {
            return false;
        }
    }
    default:
        return false;
    }
}

std::shared_ptr<LogicalOperator> FilterPushDownOptimizer::visitScanNodeTableReplace(
    const std::shared_ptr<LogicalOperator>& op) {
    auto& scan = op->cast<LogicalScanNodeTable>();
    auto nodeID = scan.getNodeID();
    // Apply column predicates.
    if (context->getClientConfig()->enableZoneMap) {
        scan.setPropertyPredicates(
            getColumnPredicateSets(scan.getProperties(), predicateSet.getAllPredicates()));
    }
    // Apply index scan
    auto tableIDs = scan.getTableIDs();
    std::shared_ptr<Expression> primaryKeyEqualityComparison = nullptr;
    if (tableIDs.size() == 1) {
        primaryKeyEqualityComparison = predicateSet.popNodePKEqualityComparison(*nodeID);
    }
    if (primaryKeyEqualityComparison != nullptr) { // Try rewrite index scan
        auto rhs = primaryKeyEqualityComparison->getChild(1);
        if (isConstantExpression(rhs)) {
            auto extraInfo = std::make_unique<PrimaryKeyScanInfo>(rhs);
            scan.setScanType(LogicalScanNodeTableType::PRIMARY_KEY_SCAN);
            scan.setExtraInfo(std::move(extraInfo));
            scan.computeFlatSchema();
        } else {
            // Cannot rewrite and add predicate back.
            predicateSet.addPredicate(primaryKeyEqualityComparison);
        }
    }
    // Try FTS scan rewrite (only if still a regular SCAN and single table).
    if (scan.getScanType() == LogicalScanNodeTableType::SCAN && tableIDs.size() == 1) {
        auto searchPredicate = predicateSet.popSearchPredicate();
        if (searchPredicate != nullptr) {
            auto& funcExpr = searchPredicate->constCast<ScalarFunctionExpression>();
            auto* ftsBindData = dynamic_cast<FTSSearchBindData*>(funcExpr.getBindData());
            KU_ASSERT(ftsBindData != nullptr);
            auto searchFunc = ftsBindData->searchFunc;
            int64_t limit = 1000;

            // Create virtual property expressions for score and highlights.
            // Unique names MUST match SEARCH_SCORE() and SEARCH_HIGHLIGHTS() calls
            // so the ExpressionMapper resolves them as references, not evaluations.
            auto scoreExpr = std::make_shared<VariableExpression>(
                LogicalType::DOUBLE(), "SEARCH_SCORE()", "_fts_score");
            auto hlExpr = std::make_shared<VariableExpression>(
                LogicalType::STRING(), "SEARCH_HIGHLIGHTS()", "_fts_highlights");

            scan.addProperty(scoreExpr);
            scan.addProperty(hlExpr);

            auto ftsInfo = std::make_unique<FTSScanInfo>(
                std::move(searchFunc), limit, scoreExpr, hlExpr);
            scan.setScanType(LogicalScanNodeTableType::FTS_SCAN);
            scan.setExtraInfo(std::move(ftsInfo));
            scan.computeFlatSchema();
        }
    }
    return finishPushDown(op);
}

std::shared_ptr<LogicalOperator> FilterPushDownOptimizer::visitTableFunctionCallReplace(
    const std::shared_ptr<LogicalOperator>& op) {
    auto& tableFunctionCall = op->cast<LogicalTableFunctionCall>();
    auto columnPredicates = getColumnPredicateSets(tableFunctionCall.getBindData()->columns,
        predicateSet.getAllPredicates());
    tableFunctionCall.setColumnPredicates(std::move(columnPredicates));
    return finishPushDown(op);
}

std::shared_ptr<LogicalOperator> FilterPushDownOptimizer::visitExtendReplace(
    const std::shared_ptr<LogicalOperator>& op) {
    if (op->ptrCast<BaseLogicalExtend>()->isRecursive() ||
        !context->getClientConfig()->enableZoneMap) {
        return visitChildren(op);
    }
    auto& extend = op->cast<LogicalExtend>();
    // Apply column predicates.
    auto columnPredicates =
        getColumnPredicateSets(extend.getProperties(), predicateSet.getAllPredicates());
    extend.setPropertyPredicates(std::move(columnPredicates));
    return visitChildren(op);
}

std::shared_ptr<LogicalOperator> FilterPushDownOptimizer::finishPushDown(
    std::shared_ptr<LogicalOperator> op) {
    if (predicateSet.isEmpty()) {
        return op;
    }
    auto predicates = predicateSet.getAllPredicates();
    auto root = appendFilters(predicates, op);
    predicateSet.clear();
    return root;
}

std::shared_ptr<LogicalOperator> FilterPushDownOptimizer::appendScanNodeTable(
    std::shared_ptr<binder::Expression> nodeID, std::vector<common::table_id_t> nodeTableIDs,
    binder::expression_vector properties, std::shared_ptr<planner::LogicalOperator> child) {
    if (properties.empty()) {
        return child;
    }
    auto printInfo = std::make_unique<OPPrintInfo>();
    auto scanNodeTable = std::make_shared<LogicalScanNodeTable>(std::move(nodeID),
        std::move(nodeTableIDs), std::move(properties));
    scanNodeTable->computeFlatSchema();
    return scanNodeTable;
}

std::shared_ptr<LogicalOperator> FilterPushDownOptimizer::appendFilters(
    const expression_vector& predicates, std::shared_ptr<LogicalOperator> child) {
    if (predicates.empty()) {
        return child;
    }
    auto root = child;
    for (auto& p : predicates) {
        root = appendFilter(p, root);
    }
    return root;
}

std::shared_ptr<LogicalOperator> FilterPushDownOptimizer::appendFilter(
    std::shared_ptr<Expression> predicate, std::shared_ptr<LogicalOperator> child) {
    auto printInfo = std::make_unique<OPPrintInfo>();
    auto filter = std::make_shared<LogicalFilter>(std::move(predicate), std::move(child));
    filter->computeFlatSchema();
    return filter;
}

void PredicateSet::addPredicate(std::shared_ptr<Expression> predicate) {
    if (predicate->expressionType == ExpressionType::EQUALS) {
        equalityPredicates.push_back(std::move(predicate));
    } else {
        nonEqualityPredicates.push_back(std::move(predicate));
    }
}

static bool isNodePrimaryKey(const Expression& expression, const Expression& nodeID) {
    if (expression.expressionType != ExpressionType::PROPERTY) {
        // not property
        return false;
    }
    auto& property = expression.constCast<PropertyExpression>();
    if (property.getVariableName() != nodeID.constCast<PropertyExpression>().getVariableName()) {
        // not property for node
        return false;
    }
    return property.isPrimaryKey();
}

std::shared_ptr<Expression> PredicateSet::popNodePKEqualityComparison(const Expression& nodeID) {
    // We pop when the first primary key equality comparison is found.
    auto resultPredicateIdx = INVALID_IDX;
    for (auto i = 0u; i < equalityPredicates.size(); ++i) {
        auto predicate = equalityPredicates[i];
        if (isNodePrimaryKey(*predicate->getChild(0), nodeID)) {
            resultPredicateIdx = i;
            break;
        } else if (isNodePrimaryKey(*predicate->getChild(1), nodeID)) {
            // Normalize primary key to LHS.
            auto leftChild = predicate->getChild(0);
            auto rightChild = predicate->getChild(1);
            predicate->setChild(1, leftChild);
            predicate->setChild(0, rightChild);
            resultPredicateIdx = i;
            break;
        }
    }
    if (resultPredicateIdx != INVALID_IDX) {
        auto result = equalityPredicates[resultPredicateIdx];
        equalityPredicates.erase(equalityPredicates.begin() + resultPredicateIdx);
        return result;
    }
    return nullptr;
}

std::shared_ptr<Expression> PredicateSet::popSearchPredicate() {
    for (auto it = nonEqualityPredicates.begin(); it != nonEqualityPredicates.end(); ++it) {
        if ((*it)->expressionType == ExpressionType::FUNCTION) {
            auto& func = (*it)->constCast<ScalarFunctionExpression>();
            if (func.getFunction().isIndexScanPredicate) {
                auto result = *it;
                nonEqualityPredicates.erase(it);
                return result;
            }
        }
    }
    return nullptr;
}

expression_vector PredicateSet::getAllPredicates() {
    expression_vector result;
    result.insert(result.end(), equalityPredicates.begin(), equalityPredicates.end());
    result.insert(result.end(), nonEqualityPredicates.begin(), nonEqualityPredicates.end());
    return result;
}

// Find the FTS_SCAN node in the plan tree and return its FTSScanInfo, or nullptr.
static FTSScanInfo* findFTSScanInfo(LogicalOperator* op) {
    if (op->getOperatorType() == LogicalOperatorType::SCAN_NODE_TABLE) {
        auto& scan = op->cast<LogicalScanNodeTable>();
        if (scan.getScanType() == LogicalScanNodeTableType::FTS_SCAN) {
            return dynamic_cast<FTSScanInfo*>(scan.getExtraInfo());
        }
    }
    for (auto i = 0u; i < op->getNumChildren(); ++i) {
        auto result = findFTSScanInfo(op->getChild(i).get());
        if (result != nullptr) {
            return result;
        }
    }
    return nullptr;
}

// Replace SEARCH_SCORE()/SEARCH_HIGHLIGHTS() function expressions with the corresponding
// VariableExpressions in an expression vector. Returns true if any replacement was made.
static bool replaceInExprVector(expression_vector& exprs,
    const std::shared_ptr<Expression>& scoreExpr,
    const std::shared_ptr<Expression>& hlExpr) {
    bool changed = false;
    for (auto& expr : exprs) {
        if (expr->expressionType == ExpressionType::FUNCTION &&
            expr->getUniqueName() == scoreExpr->getUniqueName()) {
            expr = scoreExpr;
            changed = true;
        } else if (expr->expressionType == ExpressionType::FUNCTION &&
                   expr->getUniqueName() == hlExpr->getUniqueName()) {
            expr = hlExpr;
            changed = true;
        }
    }
    return changed;
}

// Walk the plan tree and replace SEARCH_SCORE()/SEARCH_HIGHLIGHTS() ScalarFunctionExpressions
// with the VariableExpressions created by the FTS_SCAN rewrite.
void FilterPushDownOptimizer::resolveSearchExpressions(LogicalOperator* root) {
    auto* ftsInfo = findFTSScanInfo(root);
    if (ftsInfo == nullptr) {
        return;
    }
    auto& scoreExpr = ftsInfo->scoreExpr;
    auto& hlExpr = ftsInfo->highlightsExpr;

    std::function<void(LogicalOperator*)> walk = [&](LogicalOperator* op) {
        switch (op->getOperatorType()) {
        case LogicalOperatorType::PROJECTION: {
            auto& proj = op->cast<LogicalProjection>();
            auto exprs = proj.getExpressionsToProject();
            if (replaceInExprVector(exprs, scoreExpr, hlExpr)) {
                proj.setExpressionsToProject(exprs);
            }
        } break;
        case LogicalOperatorType::ORDER_BY: {
            auto& orderBy = op->cast<LogicalOrderBy>();
            auto exprs = orderBy.getExpressionsToOrderBy();
            if (replaceInExprVector(exprs, scoreExpr, hlExpr)) {
                orderBy.setExpressionsToOrderBy(std::move(exprs));
            }
        } break;
        default:
            break;
        }
        for (auto i = 0u; i < op->getNumChildren(); ++i) {
            walk(op->getChild(i).get());
        }
    };
    walk(root);
}

} // namespace optimizer
} // namespace rag3db
