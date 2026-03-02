#include "function/vector_search_function.h"

#include "binder/expression/literal_expression.h"
#include "binder/expression/property_expression.h"
#include "catalog/catalog.h"
#include "catalog/catalog_entry/node_table_catalog_entry.h"
#include "catalog/catalog_entry/rel_group_catalog_entry.h"
#include "catalog/hnsw_index_catalog_entry.h"
#include "common/exception/binder.h"
#include "common/index_search_types.h"
#include "common/string_format.h"
#include "common/types/value/nested.h"
#include "expression_evaluator/expression_evaluator_utils.h"
#include "function/scalar_function.h"
#include "index/hnsw_index.h"
#include "index/hnsw_index_utils.h"
#include "main/client_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

namespace rag3db {
namespace vector_extension {

using namespace common;
using namespace function;
using namespace catalog;

// ── VectorSearchBindData ────────────────────────────────────────────────────

struct VectorSearchBindData final : IndexSearchBindData {
    table_id_t tableID;

    VectorSearchBindData(LogicalType resultType, table_id_t tableID,
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
        return std::make_unique<VectorSearchBindData>(
            resultType.copy(), tableID, searchFunc, std::move(specsCopy));
    }
};

// ── Query vector wrapper (same as in query_hnsw_index.cpp) ──────────────────

template<typename T>
struct BindTimeQueryVector : GetEmbeddingsScanState {
    explicit BindTimeQueryVector(std::vector<T> data) : data{std::move(data)} {}

    void* getEmbeddingPtr([[maybe_unused]] const EmbeddingHandle& handle) override {
        return reinterpret_cast<void*>(data.data());
    }
    void addEmbedding(const EmbeddingHandle&) override {}
    void reclaimEmbedding(const EmbeddingHandle&) override {}

    std::vector<T> data;
};

// ── Find HNSW index for a property ──────────────────────────────────────────

static IndexCatalogEntry* findHNSWIndexForProperty(Catalog* catalog,
    const transaction::Transaction* transaction, table_id_t tableID,
    property_id_t propertyID) {
    auto indexEntries = catalog->getIndexEntries(transaction, tableID);
    for (auto* entry : indexEntries) {
        if (entry->getIndexType() == "HNSW") {
            auto propIDs = entry->getPropertyIDs();
            if (propIDs.size() == 1 && propIDs[0] == propertyID) {
                return entry;
            }
        }
    }
    return nullptr;
}

// ── Extract query vector from literal ───────────────────────────────────────

template<typename T>
static std::vector<T> extractQueryVector(const Value& val) {
    std::vector<T> vec;
    vec.resize(val.getChildrenSize());
    for (auto i = 0u; i < val.getChildrenSize(); i++) {
        vec[i] = NestedVal::getChildVal(&val, i)->getValue<T>();
    }
    return vec;
}

// ── VECTOR_SEARCH bind ──────────────────────────────────────────────────────

static std::unique_ptr<FunctionBindData> vectorSearchBindFunc(
    const ScalarBindFuncInput& input) {
    auto& arguments = input.arguments;
    auto* context = input.context;

    // Arg 0: property expression (e.g. d.embedding)
    if (arguments[0]->expressionType != ExpressionType::PROPERTY) {
        throw BinderException(
            "VECTOR_SEARCH() first argument must be a property reference (e.g. d.embedding).");
    }
    auto& propExpr = arguments[0]->constCast<binder::PropertyExpression>();
    if (!propExpr.isSingleLabel()) {
        throw BinderException(
            "VECTOR_SEARCH() requires a single-label property (one table only).");
    }
    auto tableID = propExpr.getSingleTableID();
    auto propertyName = propExpr.getPropertyName();

    // Arg 1: query vector (LIST literal)
    if (arguments[1]->expressionType != ExpressionType::LITERAL) {
        throw BinderException("VECTOR_SEARCH() query vector must be a literal list.");
    }
    auto queryVal = arguments[1]->constCast<binder::LiteralExpression>().getValue();

    // Arg 2: k (INT64 literal)
    if (arguments[2]->expressionType != ExpressionType::LITERAL) {
        throw BinderException("VECTOR_SEARCH() k must be a literal integer.");
    }
    auto k = arguments[2]->constCast<binder::LiteralExpression>().getValue().getValue<int64_t>();
    if (k <= 0) {
        throw BinderException("VECTOR_SEARCH() k must be greater than 0.");
    }

    // Resolve table, property, and HNSW index.
    auto* catalog = Catalog::Get(*context);
    auto* transaction = transaction::Transaction::Get(*context);
    auto* tableEntry =
        catalog->getTableCatalogEntry(transaction, tableID)->ptrCast<NodeTableCatalogEntry>();
    auto tableName = tableEntry->getName();
    auto propertyID = tableEntry->getPropertyID(propertyName);

    auto* indexEntry = findHNSWIndexForProperty(catalog, transaction, tableID, propertyID);
    if (!indexEntry) {
        throw BinderException(stringFormat(
            "No HNSW vector index on property '{}.{}'. "
            "Create one with CREATE_VECTOR_INDEX first.",
            tableName, propertyName));
    }

    auto indexColumnID = tableEntry->getColumnID(propertyID);
    auto indexName = indexEntry->getIndexName();

    // Get NodeTable and index.
    auto* storageManager = storage::StorageManager::Get(*context);
    auto* nodeTable =
        storageManager->getTable(tableEntry->getTableID())->ptrCast<storage::NodeTable>();
    auto indexOpt = nodeTable->getIndex(indexName);
    KU_ASSERT(indexOpt.has_value());
    auto& hnswIndex = indexOpt.value()->cast<OnDiskHNSWIndex>();

    // Build HNSWSearchState (same logic as initQueryHNSWLocalState).
    auto upperRelTableName = HNSWIndexUtils::getUpperGraphTableName(tableID, indexName);
    auto lowerRelTableName = HNSWIndexUtils::getLowerGraphTableName(tableID, indexName);
    auto* upperRelTableEntry =
        catalog->getTableCatalogEntry(transaction, upperRelTableName, true)
            ->ptrCast<RelGroupCatalogEntry>();
    auto* lowerRelTableEntry =
        catalog->getTableCatalogEntry(transaction, lowerRelTableName, true)
            ->ptrCast<RelGroupCatalogEntry>();
    auto numNodes = nodeTable->getStats(transaction).getTableCard();

    HNSWSearchState searchState{context, tableEntry, upperRelTableEntry, lowerRelTableEntry,
        *nodeTable, indexColumnID, numNodes, static_cast<uint64_t>(k), QueryHNSWConfig{}};

    // Get dimension and element type from the index.
    const auto& columnType = tableEntry->getProperty(propertyID).getType();
    auto dimension = ArrayType::getNumElements(columnType);
    auto elementType = hnswIndex.getElementType();

    // Execute HNSW search at bind time.
    std::vector<IndexSearchResult> precomputed;
    TypeUtils::visit(
        elementType,
        [&]<VectorElementType T>(T) {
            auto queryVector = extractQueryVector<T>(queryVal);
            if (queryVector.size() != dimension) {
                throw BinderException(stringFormat(
                    "VECTOR_SEARCH() query vector dimension ({}) does not match "
                    "index dimension ({}).",
                    queryVector.size(), dimension));
            }
            auto queryState = BindTimeQueryVector<T>(std::move(queryVector));
            auto queryHandle = EmbeddingHandle{0, &queryState};
            auto hnswResults = hnswIndex.search(transaction, queryHandle, searchState);
            precomputed.reserve(hnswResults.size());
            for (const auto& r : hnswResults) {
                precomputed.push_back(
                    IndexSearchResult{r.nodeOffset, r.distance, ""});
            }
        },
        [&](auto) { KU_UNREACHABLE; });

    // Lambda returns pre-computed results.
    IndexSearchFunc searchFunc =
        [precomputed = std::move(precomputed)](int64_t limit)
            -> std::vector<IndexSearchResult> {
        auto end = std::min(static_cast<int64_t>(precomputed.size()), limit);
        return std::vector<IndexSearchResult>(precomputed.begin(), precomputed.begin() + end);
    };

    std::vector<VirtualExprSpec> virtualSpecs;
    virtualSpecs.emplace_back("VECTOR_DISTANCE", LogicalType::DOUBLE());

    return std::make_unique<VectorSearchBindData>(
        LogicalType::BOOL(), tableID, std::move(searchFunc), std::move(virtualSpecs));
}

// ── VECTOR_SEARCH exec (fallback) ───────────────────────────────────────────

static void vectorSearchExecFunc(
    const std::vector<std::shared_ptr<ValueVector>>& /*params*/,
    const std::vector<SelectionVector*>& /*paramSelVectors*/,
    ValueVector& result, SelectionVector* resultSelVector, void* /*dataPtr*/) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); i++) {
        result.setValue(resultSelVector->operator[](i), false);
    }
}

// ── VECTOR_DISTANCE exec (returns NULL when no INDEX_SCAN) ──────────────────

static void vectorDistanceExecFunc(
    const std::vector<std::shared_ptr<ValueVector>>& /*params*/,
    const std::vector<SelectionVector*>& /*paramSelVectors*/,
    ValueVector& result, SelectionVector* resultSelVector, void* /*dataPtr*/) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); i++) {
        result.setNull(resultSelVector->operator[](i), true);
    }
}

// ── getFunctionSet ──────────────────────────────────────────────────────────

function_set VectorSearchFunction::getFunctionSet() {
    function_set functionSet;
    // VECTOR_SEARCH(property, queryVector, k)
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector{LogicalTypeID::ANY, LogicalTypeID::LIST, LogicalTypeID::INT64},
        LogicalTypeID::BOOL, vectorSearchExecFunc);
    func->isVarLength = true;
    func->isIndexScanPredicate = true;
    func->bindFunc = vectorSearchBindFunc;
    functionSet.push_back(std::move(func));
    return functionSet;
}

function_set VectorDistanceFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{},
        LogicalTypeID::DOUBLE, vectorDistanceExecFunc);
    func->isNonFoldable = true;
    functionSet.push_back(std::move(func));
    return functionSet;
}

} // namespace vector_extension
} // namespace rag3db
