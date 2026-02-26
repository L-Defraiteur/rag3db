#include "function/query_sparse_vector_index.h"

#include "binder/binder.h"
#include "binder/expression/literal_expression.h"
#include "common/exception/binder.h"
#include "common/string_format.h"
#include "common/types/value/nested.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/simple_table_function.h"
#include "index/sparse_vector_index.h"
#include "main/client_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

namespace rag3db {
namespace sparse_vector_extension {

using namespace common;
using namespace main;
using namespace function;

// ── Result struct ───────────────────────────────────────────────────────────

struct SparseVectorResult {
    uint64_t nodeId;
    double score;
};

// ── BindData ────────────────────────────────────────────────────────────────

struct QuerySparseVectorBindData final : TableFuncBindData {
    std::vector<SparseVectorResult> results;

    QuerySparseVectorBindData(binder::expression_vector columns,
        std::vector<SparseVectorResult> results)
        : TableFuncBindData{std::move(columns), static_cast<row_idx_t>(results.size())},
          results{std::move(results)} {}

    std::unique_ptr<TableFuncBindData> copy() const override {
        return std::make_unique<QuerySparseVectorBindData>(*this);
    }
};

// ── Bind ────────────────────────────────────────────────────────────────────

static std::unique_ptr<TableFuncBindData> bindFunc(ClientContext* context,
    const TableFuncBindInput* input) {
    auto tableName = input->getLiteralVal<std::string>(0);

    // Extract query indices from LIST[INT64] parameter.
    auto queryIndicesVal =
        input->getParam(1)->constPtrCast<binder::LiteralExpression>()->getValue();
    std::vector<uint32_t> queryIndices;
    for (auto i = 0u; i < queryIndicesVal.getChildrenSize(); i++) {
        queryIndices.push_back(
            static_cast<uint32_t>(NestedVal::getChildVal(&queryIndicesVal, i)->getValue<int64_t>()));
    }

    // Extract query weights from LIST[DOUBLE] parameter.
    auto queryWeightsVal =
        input->getParam(2)->constPtrCast<binder::LiteralExpression>()->getValue();
    std::vector<float> queryWeights;
    for (auto i = 0u; i < queryWeightsVal.getChildrenSize(); i++) {
        queryWeights.push_back(
            static_cast<float>(NestedVal::getChildVal(&queryWeightsVal, i)->getValue<double>()));
    }

    int64_t limit = 10;
    if (input->params.size() > 3) {
        limit = input->getLiteralVal<int64_t>(3);
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
            stringFormat("No sparse vector index found on table '{}'.", tableName)};
    }
    auto& sparseIndex = indexOpt.value()->cast<SparseVectorIndex>();

    // Flush pending writes so the reader sees them.
    sparseIndex.flushIfDirty();

    // Extract optional allowed_ids.
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
    rust::Vec<::SparseSearchResult> rustResults;
    if (hasFilter) {
        rustResults = sparse_search_filtered(sparseIndex.getHandle(),
            rust::Slice<const uint32_t>(queryIndices.data(), queryIndices.size()),
            rust::Slice<const float>(queryWeights.data(), queryWeights.size()),
            static_cast<uint32_t>(limit),
            rust::Slice<const uint64_t>(allowedIds.data(), allowedIds.size()));
    } else {
        rustResults = sparse_search(sparseIndex.getHandle(),
            rust::Slice<const uint32_t>(queryIndices.data(), queryIndices.size()),
            rust::Slice<const float>(queryWeights.data(), queryWeights.size()),
            static_cast<uint32_t>(limit));
    }

    std::vector<SparseVectorResult> results;
    results.reserve(rustResults.size());
    for (const auto& r : rustResults) {
        results.push_back(SparseVectorResult{r.node_id, static_cast<double>(r.score)});
    }

    // Output columns: node_id (UINT64), score (DOUBLE).
    binder::expression_vector columns;
    columns.push_back(input->binder->createVariable("node_id", LogicalType::UINT64()));
    columns.push_back(input->binder->createVariable("score", LogicalType::DOUBLE()));

    return std::make_unique<QuerySparseVectorBindData>(std::move(columns), std::move(results));
}

// ── Table function ──────────────────────────────────────────────────────────

static offset_t internalTableFunc(const TableFuncMorsel& morsel,
    const TableFuncInput& input, DataChunk& output) {
    auto& bd = *input.bindData->constPtrCast<QuerySparseVectorBindData>();
    auto numResults = static_cast<offset_t>(bd.results.size());
    if (morsel.startOffset >= numResults) {
        return 0;
    }
    auto count = std::min(morsel.endOffset, numResults) - morsel.startOffset;
    for (offset_t i = 0; i < count; i++) {
        auto& result = bd.results[morsel.startOffset + i];
        output.getValueVectorMutable(0).setValue(i, result.nodeId);
        output.getValueVectorMutable(1).setValue(i, result.score);
    }
    output.state->getSelVectorUnsafe().setSelSize(count);
    return count;
}

// ── inferInputTypes ─────────────────────────────────────────────────────────

static std::vector<LogicalType> inferInputTypes(const binder::expression_vector&) {
    std::vector<LogicalType> types;
    types.push_back(LogicalType::STRING());
    types.push_back(LogicalType::LIST(LogicalType::INT64()));
    types.push_back(LogicalType::LIST(LogicalType::DOUBLE()));
    types.push_back(LogicalType::INT64());
    return types;
}

// ── getFunctionSet ──────────────────────────────────────────────────────────

function_set QuerySparseVectorFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<TableFunction>(name,
        std::vector{LogicalTypeID::STRING, LogicalTypeID::LIST,
            LogicalTypeID::LIST, LogicalTypeID::INT64});
    func->tableFunc = SimpleTableFunc::getTableFunc(internalTableFunc);
    func->bindFunc = bindFunc;
    func->initSharedStateFunc = SimpleTableFunc::initSharedState;
    func->initLocalStateFunc = TableFunction::initEmptyLocalState;
    func->canParallelFunc = [] { return false; };
    func->inferInputTypes = inferInputTypes;
    functionSet.push_back(std::move(func));
    return functionSet;
}

} // namespace sparse_vector_extension
} // namespace rag3db
