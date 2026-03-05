#include "function/search_function.h"

#include "binder/expression/literal_expression.h"
#include "binder/expression/property_expression.h"
#include "function/scalar_function.h"
#include "catalog/catalog.h"
#include "common/exception/binder.h"
#include "common/index_search_types.h"
#include "common/string_format.h"
#include "index/lucivy_index.h"
#include "main/client_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"
#include "util/highlights_util.h"

namespace rag3db {
namespace lucivy_fts_extension {

using namespace common;
using namespace function;

// ── SearchBindData ──────────────────────────────────────────────────────────

struct SearchBindData final : IndexSearchBindData {
    table_id_t tableID;
    std::string fieldName;

    SearchBindData(LogicalType resultType, table_id_t tableID, std::string fieldName,
        IndexSearchFunc searchFunc, std::vector<VirtualExprSpec> virtualExprSpecs)
        : IndexSearchBindData{std::move(resultType), std::move(searchFunc),
              std::move(virtualExprSpecs)},
          tableID{tableID}, fieldName{std::move(fieldName)} {}

    std::unique_ptr<FunctionBindData> copy() const override {
        std::vector<VirtualExprSpec> specsCopy;
        specsCopy.reserve(virtualExprSpecs.size());
        for (auto& spec : virtualExprSpecs) {
            specsCopy.push_back(spec.copy());
        }
        return std::make_unique<SearchBindData>(
            resultType.copy(), tableID, fieldName, searchFunc, std::move(specsCopy));
    }
};

// ── SEARCH bind ─────────────────────────────────────────────────────────────

static std::unique_ptr<FunctionBindData> searchBindFunc(const ScalarBindFuncInput& input) {
    auto& arguments = input.arguments;
    auto* context = input.context;

    // Arg 0: must be a property expression (e.g. d.body)
    if (arguments[0]->expressionType != ExpressionType::PROPERTY) {
        throw BinderException("SEARCH() first argument must be a property reference (e.g. d.body).");
    }
    auto& propExpr = arguments[0]->constCast<binder::PropertyExpression>();
    if (!propExpr.isSingleLabel()) {
        throw BinderException("SEARCH() requires a single-label property (one table only).");
    }
    auto tableID = propExpr.getSingleTableID();
    auto fieldName = propExpr.getPropertyName();

    // Arg 1: query text (must be literal string)
    if (arguments[1]->expressionType != ExpressionType::LITERAL) {
        throw BinderException("SEARCH() query text must be a literal string.");
    }
    auto queryText =
        arguments[1]->constCast<binder::LiteralExpression>().getValue().getValue<std::string>();

    // Arg 2 (optional): mode — default "contains"
    std::string mode = "contains";
    if (arguments.size() > 2) {
        if (arguments[2]->expressionType != ExpressionType::LITERAL) {
            throw BinderException("SEARCH() mode must be a literal string.");
        }
        mode = arguments[2]->constCast<binder::LiteralExpression>().getValue()
                   .getValue<std::string>();
    }

    // Arg 3 (optional): fuzzy distance — default 1
    int64_t distance = 1;
    if (arguments.size() > 3) {
        if (arguments[3]->expressionType != ExpressionType::LITERAL) {
            throw BinderException("SEARCH() distance must be a literal integer.");
        }
        distance =
            arguments[3]->constCast<binder::LiteralExpression>().getValue().getValue<int64_t>();
    }

    // Resolve table and Lucivy index.
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
            "No Lucivy full-text index on table '{}'. "
            "Create one with CREATE_LUCIVY_INDEX first.", tableName));
    }

    // Create search lambda — typed bridge call, no JSON.
    auto& lucivyIndex = indexOpt.value()->cast<LucivyIndex>();
    LucivyIndex* indexPtr = &lucivyIndex;

    IndexSearchFunc searchFunc =
        [indexPtr, fieldName, queryText, mode,
            dist = static_cast<uint8_t>(distance)](int64_t limit)
        -> std::vector<IndexSearchResult> {
        indexPtr->flushIfDirty();
        auto rustResults = search_typed_with_highlights(
            indexPtr->getHandle(), fieldName, queryText, mode, dist,
            static_cast<uint32_t>(limit));
        std::vector<IndexSearchResult> results;
        results.reserve(rustResults.size());
        for (const auto& r : rustResults) {
            results.push_back(IndexSearchResult{
                static_cast<offset_t>(r.node_id),
                static_cast<double>(r.score),
                highlightsToJson(r.highlights)});
        }
        return results;
    };

    std::vector<VirtualExprSpec> virtualSpecs;
    virtualSpecs.emplace_back("SEARCH_SCORE", LogicalType::DOUBLE());
    virtualSpecs.emplace_back("SEARCH_HIGHLIGHTS", LogicalType::STRING());

    return std::make_unique<SearchBindData>(
        LogicalType::BOOL(), tableID, fieldName,
        std::move(searchFunc), std::move(virtualSpecs));
}

// ── SEARCH exec (fallback — optimizer should convert to FTS_SCAN) ───────────

static void searchExecFunc(
    const std::vector<std::shared_ptr<ValueVector>>& /*params*/,
    const std::vector<SelectionVector*>& /*paramSelVectors*/,
    ValueVector& result, SelectionVector* resultSelVector, void* /*dataPtr*/) {
    // If optimizer didn't intercept, return false (no match) for safety.
    for (auto i = 0u; i < resultSelVector->getSelSize(); i++) {
        result.setValue(resultSelVector->operator[](i), false);
    }
}

// ── SEARCH_SCORE exec (returns NULL when no FTS_SCAN) ───────────────────────

static void searchScoreExecFunc(
    const std::vector<std::shared_ptr<ValueVector>>& /*params*/,
    const std::vector<SelectionVector*>& /*paramSelVectors*/,
    ValueVector& result, SelectionVector* resultSelVector, void* /*dataPtr*/) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); i++) {
        result.setNull(resultSelVector->operator[](i), true);
    }
}

// ── SEARCH_HIGHLIGHTS exec (returns NULL when no FTS_SCAN) ──────────────────

static void searchHighlightsExecFunc(
    const std::vector<std::shared_ptr<ValueVector>>& /*params*/,
    const std::vector<SelectionVector*>& /*paramSelVectors*/,
    ValueVector& result, SelectionVector* resultSelVector, void* /*dataPtr*/) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); i++) {
        result.setNull(resultSelVector->operator[](i), true);
    }
}

// ── getFunctionSet ──────────────────────────────────────────────────────────

function_set SearchFunction::getFunctionSet() {
    function_set functionSet;
    // SEARCH(property, query [, mode [, distance]])
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector{LogicalTypeID::ANY, LogicalTypeID::STRING},
        LogicalTypeID::BOOL, searchExecFunc);
    func->isVarLength = true;
    func->isIndexScanPredicate = true;
    func->bindFunc = searchBindFunc;
    functionSet.push_back(std::move(func));
    return functionSet;
}

function_set SearchScoreFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{},
        LogicalTypeID::DOUBLE, searchScoreExecFunc);
    func->isNonFoldable = true;
    functionSet.push_back(std::move(func));
    return functionSet;
}

function_set SearchHighlightsFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{},
        LogicalTypeID::STRING, searchHighlightsExecFunc);
    func->isNonFoldable = true;
    functionSet.push_back(std::move(func));
    return functionSet;
}

} // namespace lucivy_fts_extension
} // namespace rag3db
