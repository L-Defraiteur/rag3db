#include "function/query_spatial_index.h"

#include "binder/expression/literal_expression.h"
#include "catalog/spatial_index_catalog_entry.h"
#include "common/exception/binder.h"
#include "common/string_format.h"
#include "common/types/value/nested.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/simple_table_function.h"
#include "index/rtree.h"
#include "index/rtree_index.h"
#include "main/client_context.h"
#include "math/quaternion.h"
#include "processor/execution_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

namespace rag3db {
namespace geo_extension {

using namespace common;
using namespace main;
using namespace function;

// ── BindData ─────────────────────────────────────────────────────────────────

struct QuerySpatialBindData final : TableFuncBindData {
    std::string tableName;
    table_id_t tableID;
    std::string indexName;
    std::vector<double> queryPoint;
    uint32_t k;

    // Query mode flags.
    double radius = -1.0;   // Radius mode if >= 0.
    bool hasBBox = false;
    std::vector<double> bboxMin;
    std::vector<double> bboxMax;

    bool hasOBB = false;
    std::vector<double> obbCenter;
    std::vector<double> obbHalfExtents;
    math::Quat obbQuat;

    bool hasFrustum = false;
    std::vector<double> frustumPlanes; // Flat N*4.

    QuerySpatialBindData(binder::expression_vector columns, std::string tableName,
        table_id_t tableID, std::string indexName, std::vector<double> queryPoint, uint32_t k)
        : TableFuncBindData{std::move(columns), 1},
          tableName{std::move(tableName)}, tableID{tableID}, indexName{std::move(indexName)},
          queryPoint{std::move(queryPoint)}, k{k} {}

    std::unique_ptr<TableFuncBindData> copy() const override {
        return std::make_unique<QuerySpatialBindData>(*this);
    }
};

// ── Local state ──────────────────────────────────────────────────────────────

struct QuerySpatialLocalState final : TableFuncLocalState {
    std::vector<std::pair<uint64_t, double>> results;
    uint64_t numRowsOutput = 0;
    bool executed = false;
};

// ── Helper: extract list from optional param value ───────────────────────────

static std::vector<double> extractOptionalList(const Value& val) {
    std::vector<double> result;
    for (auto i = 0u; i < val.getChildrenSize(); i++) {
        result.push_back(NestedVal::getChildVal(&val, i)->getValue<double>());
    }
    return result;
}

// ── Bind ─────────────────────────────────────────────────────────────────────

static std::unique_ptr<TableFuncBindData> bindFunc(ClientContext* context,
    const TableFuncBindInput* input) {
    auto tableName = input->getLiteralVal<std::string>(0);
    auto indexName = input->getLiteralVal<std::string>(1);
    auto queryPointValue =
        input->getParam(2)->constPtrCast<binder::LiteralExpression>()->getValue();
    auto k = static_cast<uint32_t>(input->getLiteralVal<int64_t>(3));

    // Parse query point.
    std::vector<double> queryPoint;
    for (auto i = 0u; i < queryPointValue.getChildrenSize(); i++) {
        queryPoint.push_back(NestedVal::getChildVal(&queryPointValue, i)->getValue<double>());
    }

    auto catalog = catalog::Catalog::Get(*context);
    auto transaction = transaction::Transaction::Get(*context);
    auto tableEntry = catalog->getTableCatalogEntry(transaction, tableName);
    if (!tableEntry) {
        throw BinderException{stringFormat("Table '{}' does not exist.", tableName)};
    }

    binder::expression_vector columns;
    auto bd = std::make_unique<QuerySpatialBindData>(std::move(columns), tableName,
        tableEntry->getTableID(), indexName, std::move(queryPoint), k);

    // Optional params.
    for (auto& [name, val] : input->optionalParams) {
        if (name == "radius") {
            bd->radius = val.getValue<double>();
        } else if (name == "bbox_min") {
            bd->hasBBox = true;
            bd->bboxMin = extractOptionalList(val);
        } else if (name == "bbox_max") {
            bd->bboxMax = extractOptionalList(val);
        } else if (name == "obb_center") {
            bd->hasOBB = true;
            bd->obbCenter = extractOptionalList(val);
        } else if (name == "obb_half_extents") {
            bd->obbHalfExtents = extractOptionalList(val);
        } else if (name == "obb_quaternion") {
            // Three.js convention: [x,y,z,w].
            auto v = extractOptionalList(val);
            if (v.size() == 4) {
                bd->obbQuat = {v[3], v[0], v[1], v[2]};
            }
        } else if (name == "frustum_planes") {
            bd->hasFrustum = true;
            bd->frustumPlanes = extractOptionalList(val);
        }
    }

    return bd;
}

// ── Table function ───────────────────────────────────────────────────────────

static std::unique_ptr<TableFuncLocalState> initLocalState(
    const TableFuncInitLocalStateInput&) {
    return std::make_unique<QuerySpatialLocalState>();
}

static offset_t tableFunc(const TableFuncInput& input, TableFuncOutput& output) {
    auto localState = input.localState->ptrCast<QuerySpatialLocalState>();
    auto& bd = *input.bindData->constPtrCast<QuerySpatialBindData>();

    if (!localState->executed) {
        localState->executed = true;

        // Get the index from the node table.
        auto* storageManager = storage::StorageManager::Get(*input.context->clientContext);
        auto* nodeTable =
            storageManager->getTable(bd.tableID)->ptrCast<storage::NodeTable>();
        auto indexOpt = nodeTable->getIndex(bd.indexName);
        if (!indexOpt.has_value()) {
            return 0;
        }
        auto& index = indexOpt.value()->cast<RTreeIndex>();

        // Execute the appropriate query by priority: OBB > frustum > bbox > radius > KNN.
        if (bd.hasOBB && !bd.obbCenter.empty() && !bd.obbHalfExtents.empty()) {
            localState->results = index.searchOBB(bd.obbCenter, bd.obbHalfExtents, bd.obbQuat);
            if (localState->results.size() > bd.k) {
                localState->results.resize(bd.k);
            }
        } else if (bd.hasFrustum && bd.frustumPlanes.size() >= 4) {
            uint32_t numPlanes = static_cast<uint32_t>(bd.frustumPlanes.size() / 4);
            localState->results =
                index.searchFrustum(bd.frustumPlanes.data(), numPlanes, bd.queryPoint);
            if (localState->results.size() > bd.k) {
                localState->results.resize(bd.k);
            }
        } else if (bd.hasBBox && !bd.bboxMin.empty() && !bd.bboxMax.empty()) {
            BoundingBox bbox;
            bbox.minCoords = bd.bboxMin;
            bbox.maxCoords = bd.bboxMax;
            auto offsets = index.searchBBox(bbox);
            for (auto offset : offsets) {
                localState->results.push_back({offset, 0.0});
            }
            if (localState->results.size() > bd.k) {
                localState->results.resize(bd.k);
            }
        } else if (bd.radius >= 0) {
            localState->results = index.searchRadius(bd.queryPoint, bd.radius);
            if (localState->results.size() > bd.k) {
                localState->results.resize(bd.k);
            }
        } else {
            localState->results = index.knn(bd.queryPoint, bd.k);
        }
    }

    // Output results.
    if (localState->numRowsOutput >= localState->results.size()) {
        return 0;
    }
    auto numToOutput = std::min(localState->results.size() - localState->numRowsOutput,
        static_cast<size_t>(DEFAULT_VECTOR_CAPACITY));
    for (auto i = 0u; i < numToOutput; i++) {
        auto& [nodeOffset, distance] = localState->results[i + localState->numRowsOutput];
        output.dataChunk.getValueVectorMutable(0).setValue<internalID_t>(i,
            internalID_t{nodeOffset, bd.tableID});
        output.dataChunk.getValueVectorMutable(1).setValue<double>(i, distance);
    }
    localState->numRowsOutput += numToOutput;
    output.dataChunk.state->getSelVectorUnsafe().setToUnfiltered(numToOutput);
    return numToOutput;
}

// ── getFunctionSet ───────────────────────────────────────────────────────────

function_set QuerySpatialIndexFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<TableFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::STRING, LogicalTypeID::STRING,
            LogicalTypeID::LIST, LogicalTypeID::INT64});
    func->tableFunc = tableFunc;
    func->bindFunc = bindFunc;
    func->initLocalStateFunc = initLocalState;
    result.push_back(std::move(func));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
