#include "function/query_lucivy_index.h"

#include "binder/binder.h"
#include "binder/expression/literal_expression.h"
#include "common/exception/binder.h"
#include "common/string_format.h"
#include "common/types/value/nested.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/simple_table_function.h"
#include "index/lucivy_index.h"
#include "main/client_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"
#include "util/highlights_util.h"

namespace rag3db {
namespace lucivy_fts_extension {

using namespace common;
using namespace main;
using namespace function;

// ── Result struct ───────────────────────────────────────────────────────────

struct LucivyResult {
    uint64_t nodeId;
    double score;
    std::string highlights; // JSON string
};

// ── BindData ────────────────────────────────────────────────────────────────

struct QueryLucivyBindData final : TableFuncBindData {
    std::vector<LucivyResult> results;

    QueryLucivyBindData(binder::expression_vector columns,
        std::vector<LucivyResult> results)
        : TableFuncBindData{std::move(columns), static_cast<row_idx_t>(results.size())},
          results{std::move(results)} {}

    std::unique_ptr<TableFuncBindData> copy() const override {
        return std::make_unique<QueryLucivyBindData>(*this);
    }
};

// ── Bind ────────────────────────────────────────────────────────────────────

static std::unique_ptr<TableFuncBindData> bindFunc(ClientContext* context,
    const TableFuncBindInput* input) {
    auto tableName = input->getLiteralVal<std::string>(0);
    auto queryJson = input->getLiteralVal<std::string>(1);
    int64_t limit = 10;
    if (input->params.size() > 2) {
        limit = input->getLiteralVal<int64_t>(2);
    }

    auto catalog = catalog::Catalog::Get(*context);
    auto transaction = transaction::Transaction::Get(*context);
    auto tableEntry = catalog->getTableCatalogEntry(transaction, tableName);
    if (!tableEntry) {
        throw BinderException{stringFormat("Table '{}' does not exist.", tableName)};
    }

    auto* storageManager = storage::StorageManager::Get(*context);
    auto& nodeTable =
        storageManager->getTable(tableEntry->getTableID())->cast<storage::NodeTable>();
    auto indexOpt = nodeTable.getIndex(tableName);
    if (!indexOpt.has_value()) {
        throw BinderException{
            stringFormat("No Lucivy index found on table '{}'.", tableName)};
    }
    auto& lucivyIndex = indexOpt.value()->cast<LucivyIndex>();

    // Flush pending writes (insert/delete/update) so the reader sees them.
    lucivyIndex.flushIfDirty();

    // Extract optional allowed_ids for filtered search.
    std::vector<uint64_t> allowedIds;
    bool hasFilter = false;
    for (auto& [name, val] : input->optionalParams) {
        if (name == "allowed_ids") {
            hasFilter = true;
            for (auto i = 0u; i < val.getChildrenSize(); i++) {
                allowedIds.push_back(
                    NestedVal::getChildVal(&val, i)->getValue<uint64_t>());
            }
        }
    }

    // Execute search.
    rust::Vec<SearchResultWithHighlights> rustResults;
    if (hasFilter) {
        rustResults = search_filtered_with_highlights(lucivyIndex.getHandle(), queryJson,
            static_cast<uint32_t>(limit),
            rust::Slice<const uint64_t>(allowedIds.data(), allowedIds.size()));
    } else {
        rustResults = search_with_highlights(
            lucivyIndex.getHandle(), queryJson, static_cast<uint32_t>(limit));
    }

    std::vector<LucivyResult> results;
    results.reserve(rustResults.size());
    for (const auto& r : rustResults) {
        results.push_back(LucivyResult{
            r.node_id, static_cast<double>(r.score), highlightsToJson(r.highlights)});
    }

    // Output columns: node_id (UINT64), score (DOUBLE), highlights (STRING).
    binder::expression_vector columns;
    columns.push_back(input->binder->createVariable("node_id", LogicalType::UINT64()));
    columns.push_back(input->binder->createVariable("score", LogicalType::DOUBLE()));
    columns.push_back(input->binder->createVariable("highlights", LogicalType::STRING()));

    return std::make_unique<QueryLucivyBindData>(std::move(columns), std::move(results));
}

// ── Table function ──────────────────────────────────────────────────────────

static offset_t internalTableFunc(const TableFuncMorsel& morsel,
    const TableFuncInput& input, DataChunk& output) {
    auto& bd = *input.bindData->constPtrCast<QueryLucivyBindData>();
    auto numResults = static_cast<offset_t>(bd.results.size());
    if (morsel.startOffset >= numResults) {
        return 0;
    }
    auto count = std::min(morsel.endOffset, numResults) - morsel.startOffset;
    for (offset_t i = 0; i < count; i++) {
        auto& result = bd.results[morsel.startOffset + i];
        output.getValueVectorMutable(0).setValue(i, result.nodeId);
        output.getValueVectorMutable(1).setValue(i, result.score);
        output.getValueVectorMutable(2).setValue(i, result.highlights);
    }
    output.state->getSelVectorUnsafe().setSelSize(count);
    return count;
}

// ── getFunctionSet ──────────────────────────────────────────────────────────

function_set QueryLucivyFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<TableFunction>(name,
        std::vector{LogicalTypeID::STRING, LogicalTypeID::STRING, LogicalTypeID::INT64});
    func->tableFunc = SimpleTableFunc::getTableFunc(internalTableFunc);
    func->bindFunc = bindFunc;
    func->initSharedStateFunc = SimpleTableFunc::initSharedState;
    func->initLocalStateFunc = TableFunction::initEmptyLocalState;
    func->canParallelFunc = [] { return false; };
    functionSet.push_back(std::move(func));
    return functionSet;
}

} // namespace lucivy_fts_extension
} // namespace rag3db
