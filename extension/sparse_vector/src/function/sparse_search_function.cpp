#include "function/sparse_search_function.h"

#include "binder/expression/literal_expression.h"
#include "binder/expression/property_expression.h"
#include "function/scalar_function.h"
#include "catalog/catalog.h"
#include "common/exception/binder.h"
#include "common/index_search_types.h"
#include "common/string_format.h"
#include "common/types/value/nested.h"
#include "index/sparse_vector_index.h"
#include "main/client_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

namespace rag3db {
namespace sparse_vector_extension {

using namespace common;
using namespace function;

// ── SparseSearchBindData ────────────────────────────────────────────────────

struct SparseSearchBindData final : IndexSearchBindData {
    table_id_t tableID;

    SparseSearchBindData(LogicalType resultType, table_id_t tableID,
        IndexSearchFunc searchFunc, std::vector<VirtualExprSpec> virtualExprSpecs)
        : IndexSearchBindData{std::move(resultType), std::move(searchFunc),
              std::move(virtualExprSpecs)},
          tableID{tableID} {}

    std::unique_ptr<FunctionBindData> copy() const override {
        std::vector<VirtualExprSpec> specsCopy;
        specsCopy.reserve(virtualExprSpecs.size());
        for (auto& spec : virtualExprSpecs) {
            specsCopy.push_back(spec.copy());
        }
        return std::make_unique<SparseSearchBindData>(
            resultType.copy(), tableID, searchFunc, std::move(specsCopy));
    }
};

// ── SPARSE_SEARCH bind ──────────────────────────────────────────────────────

static std::unique_ptr<FunctionBindData> sparseSearchBindFunc(
    const ScalarBindFuncInput& input) {
    auto& arguments = input.arguments;
    auto* context = input.context;

    // Arg 0: must be a property expression (e.g. d.ID)
    if (arguments[0]->expressionType != ExpressionType::PROPERTY) {
        throw BinderException(
            "SPARSE_SEARCH() first argument must be a property reference (e.g. d.ID).");
    }
    auto& propExpr = arguments[0]->constCast<binder::PropertyExpression>();
    if (!propExpr.isSingleLabel()) {
        throw BinderException(
            "SPARSE_SEARCH() requires a single-label property (one table only).");
    }
    auto tableID = propExpr.getSingleTableID();

    // Arg 1: query indices LIST[INT64] (must be literal)
    if (arguments[1]->expressionType != ExpressionType::LITERAL) {
        throw BinderException("SPARSE_SEARCH() query indices must be a literal list.");
    }
    auto indicesVal =
        arguments[1]->constCast<binder::LiteralExpression>().getValue();
    std::vector<uint32_t> queryIndices;
    for (auto i = 0u; i < indicesVal.getChildrenSize(); i++) {
        queryIndices.push_back(
            static_cast<uint32_t>(NestedVal::getChildVal(&indicesVal, i)->getValue<int64_t>()));
    }

    // Arg 2: query weights LIST[DOUBLE] (must be literal)
    if (arguments[2]->expressionType != ExpressionType::LITERAL) {
        throw BinderException("SPARSE_SEARCH() query weights must be a literal list.");
    }
    auto weightsVal =
        arguments[2]->constCast<binder::LiteralExpression>().getValue();
    std::vector<float> queryWeights;
    for (auto i = 0u; i < weightsVal.getChildrenSize(); i++) {
        queryWeights.push_back(
            static_cast<float>(NestedVal::getChildVal(&weightsVal, i)->getValue<double>()));
    }

    // Arg 3 (optional): limit — default 1000
    int64_t limit = 1000;
    if (arguments.size() > 3) {
        if (arguments[3]->expressionType != ExpressionType::LITERAL) {
            throw BinderException("SPARSE_SEARCH() limit must be a literal integer.");
        }
        limit =
            arguments[3]->constCast<binder::LiteralExpression>().getValue().getValue<int64_t>();
    }

    // Resolve table and sparse vector index.
    auto* catalog = catalog::Catalog::Get(*context);
    auto* transaction = transaction::Transaction::Get(*context);
    auto* tableEntry = catalog->getTableCatalogEntry(transaction, tableID);
    auto tableName = tableEntry->getName();

    auto* storageManager = storage::StorageManager::Get(*context);
    auto& nodeTable =
        storageManager->getTable(tableEntry->getTableID())->cast<storage::NodeTable>();
    auto indexOpt = nodeTable.getIndex(tableName);
    if (!indexOpt.has_value()) {
        throw BinderException(stringFormat(
            "No sparse vector index on table '{}'. "
            "Create one with CREATE_SPARSE_VECTOR_INDEX first.",
            tableName));
    }

    auto& sparseIndex = indexOpt.value()->cast<SparseVectorIndex>();
    SparseVectorIndex* indexPtr = &sparseIndex;

    IndexSearchFunc searchFunc =
        [indexPtr, queryIndices = std::move(queryIndices),
            queryWeights = std::move(queryWeights),
            limit](int64_t searchLimit) -> std::vector<IndexSearchResult> {
        indexPtr->flushIfDirty();
        auto effectiveLimit = std::min(limit, searchLimit);
        auto rustResults = sparse_search(indexPtr->getHandle(),
            rust::Slice<const uint32_t>(queryIndices.data(), queryIndices.size()),
            rust::Slice<const float>(queryWeights.data(), queryWeights.size()),
            static_cast<uint32_t>(effectiveLimit));
        std::vector<IndexSearchResult> results;
        results.reserve(rustResults.size());
        for (const auto& r : rustResults) {
            results.push_back(IndexSearchResult{
                static_cast<offset_t>(r.node_id), static_cast<double>(r.score), ""});
        }
        return results;
    };

    std::vector<VirtualExprSpec> virtualSpecs;
    virtualSpecs.emplace_back("SPARSE_SCORE", LogicalType::DOUBLE());

    return std::make_unique<SparseSearchBindData>(
        LogicalType::BOOL(), tableID, std::move(searchFunc), std::move(virtualSpecs));
}

// ── SPARSE_SEARCH exec (fallback — optimizer should convert to INDEX_SCAN) ──

static void sparseSearchExecFunc(
    const std::vector<std::shared_ptr<ValueVector>>& /*params*/,
    const std::vector<SelectionVector*>& /*paramSelVectors*/,
    ValueVector& result, SelectionVector* resultSelVector, void* /*dataPtr*/) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); i++) {
        result.setValue(resultSelVector->operator[](i), false);
    }
}

// ── SPARSE_SCORE exec (returns NULL when no INDEX_SCAN) ─────────────────────

static void sparseScoreExecFunc(
    const std::vector<std::shared_ptr<ValueVector>>& /*params*/,
    const std::vector<SelectionVector*>& /*paramSelVectors*/,
    ValueVector& result, SelectionVector* resultSelVector, void* /*dataPtr*/) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); i++) {
        result.setNull(resultSelVector->operator[](i), true);
    }
}

// ── getFunctionSet ──────────────────────────────────────────────────────────

function_set SparseSearchFunction::getFunctionSet() {
    function_set functionSet;
    // SPARSE_SEARCH(property, queryIndices, queryWeights [, limit])
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector{LogicalTypeID::ANY, LogicalTypeID::LIST, LogicalTypeID::LIST},
        LogicalTypeID::BOOL, sparseSearchExecFunc);
    func->isVarLength = true;
    func->isIndexScanPredicate = true;
    func->bindFunc = sparseSearchBindFunc;
    functionSet.push_back(std::move(func));
    return functionSet;
}

function_set SparseScoreFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{},
        LogicalTypeID::DOUBLE, sparseScoreExecFunc);
    func->isNonFoldable = true;
    functionSet.push_back(std::move(func));
    return functionSet;
}

} // namespace sparse_vector_extension
} // namespace rag3db
