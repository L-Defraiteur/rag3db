#include "function/search_function.h"

#include <sstream>

#include "binder/expression/literal_expression.h"
#include "binder/expression/property_expression.h"
#include "function/scalar_function.h"
#include "catalog/catalog.h"
#include "common/exception/binder.h"
#include "common/fts_types.h"
#include "common/string_format.h"
#include "index/tantivy_index.h"
#include "main/client_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

namespace rag3db {
namespace tantivy_fts_extension {

using namespace common;
using namespace function;

// ── Highlights -> JSON (shared with query_tantivy_index.cpp) ────────────────

static std::string highlightsToJson(const rust::Vec<FieldHighlights>& highlights) {
    if (highlights.empty()) return "{}";
    std::string json = "{";
    for (size_t i = 0; i < highlights.size(); i++) {
        if (i > 0) json += ",";
        json += "\"";
        json += std::string(highlights[i].field_name);
        json += "\":[";
        for (size_t j = 0; j < highlights[i].ranges.size(); j++) {
            if (j > 0) json += ",";
            json += "[";
            json += std::to_string(highlights[i].ranges[j].start);
            json += ",";
            json += std::to_string(highlights[i].ranges[j].end);
            json += "]";
        }
        json += "]";
    }
    json += "}";
    return json;
}

// ── JSON escape ─────────────────────────────────────────────────────────────

static std::string escapeJson(const std::string& s) {
    std::string r;
    r.reserve(s.size());
    for (char c : s) {
        switch (c) {
        case '"': r += "\\\""; break;
        case '\\': r += "\\\\"; break;
        case '\n': r += "\\n"; break;
        case '\r': r += "\\r"; break;
        case '\t': r += "\\t"; break;
        default: r += c;
        }
    }
    return r;
}

// ── JSON query builder ──────────────────────────────────────────────────────

static std::string buildQueryJson(const std::string& field, const std::string& value,
    const std::string& mode, int64_t distance) {
    auto ef = escapeJson(field);
    auto ev = escapeJson(value);

    if (mode == "contains") {
        return "{\"type\":\"contains\",\"field\":\"" + ef + "\",\"value\":\"" + ev + "\"}";
    }
    if (mode == "contains_split") {
        std::istringstream iss(value);
        std::string word;
        std::string should;
        bool first = true;
        while (iss >> word) {
            if (!first) should += ",";
            should += "{\"type\":\"contains\",\"field\":\"" + ef +
                      "\",\"value\":\"" + escapeJson(word) + "\"}";
            first = false;
        }
        if (first) {
            return "{\"type\":\"contains\",\"field\":\"" + ef + "\",\"value\":\"\"}";
        }
        return "{\"type\":\"boolean\",\"should\":[" + should + "]}";
    }
    if (mode == "fuzzy") {
        return "{\"type\":\"contains\",\"field\":\"" + ef + "\",\"value\":\"" + ev +
               "\",\"distance\":" + std::to_string(distance) + "}";
    }
    if (mode == "regex") {
        return "{\"type\":\"contains\",\"field\":\"" + ef +
               "\",\"value\":\"" + ev + "\",\"regex\":true}";
    }
    if (mode == "parse") {
        return "{\"type\":\"parse\",\"fields\":[\"" + ef + "\"],\"value\":\"" + ev + "\"}";
    }
    throw BinderException(stringFormat(
        "Unknown SEARCH mode: '{}'. Valid: contains, contains_split, fuzzy, regex, parse.", mode));
}

// ── SearchBindData ──────────────────────────────────────────────────────────

struct SearchBindData final : FTSSearchBindData {
    table_id_t tableID;
    std::string fieldName;
    std::string queryJson;

    SearchBindData(LogicalType resultType, table_id_t tableID, std::string fieldName,
        std::string queryJson, FTSSearchFunc searchFunc)
        : FTSSearchBindData{std::move(resultType), std::move(searchFunc)}, tableID{tableID},
          fieldName{std::move(fieldName)}, queryJson{std::move(queryJson)} {}

    std::unique_ptr<FunctionBindData> copy() const override {
        return std::make_unique<SearchBindData>(
            resultType.copy(), tableID, fieldName, queryJson, searchFunc);
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

    // Resolve table and Tantivy index.
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
            "No Tantivy full-text index on table '{}'. "
            "Create one with CREATE_TANTIVY_INDEX first.", tableName));
    }

    // Build query JSON from Cypher parameters.
    auto queryJson = buildQueryJson(fieldName, queryText, mode, distance);

    // Create search lambda (captures raw pointer — valid for DB lifetime).
    auto& tantivyIndex = indexOpt.value()->cast<TantivyIndex>();
    TantivyIndex* indexPtr = &tantivyIndex;

    FTSSearchFunc searchFunc = [indexPtr, queryJson](int64_t limit)
        -> std::vector<FTSSearchResult> {
        indexPtr->flushIfDirty();
        auto rustResults = search_with_highlights(
            indexPtr->getHandle(), queryJson, static_cast<uint32_t>(limit));
        std::vector<FTSSearchResult> results;
        results.reserve(rustResults.size());
        for (const auto& r : rustResults) {
            results.push_back(FTSSearchResult{
                static_cast<offset_t>(r.node_id),
                static_cast<double>(r.score),
                highlightsToJson(r.highlights)});
        }
        return results;
    };

    return std::make_unique<SearchBindData>(
        LogicalType::BOOL(), tableID, fieldName,
        std::move(queryJson), std::move(searchFunc));
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

} // namespace tantivy_fts_extension
} // namespace rag3db
