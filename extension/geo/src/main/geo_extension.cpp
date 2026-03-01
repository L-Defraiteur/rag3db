#include "main/geo_extension.h"

#include "catalog/catalog.h"
#include "catalog/spatial_index_catalog_entry.h"
#include "function/create_spatial_index.h"
#include "function/drop_spatial_index.h"
#include "function/geo_contains.h"
#include "function/geo_distance.h"
#include "function/geo_distance_euclidean.h"
#include "function/geo_frustum_from_camera.h"
#include "function/geo_matrix_multiply.h"
#include "function/geo_matrix_rotate.h"
#include "function/geo_matrix_transpose.h"
#include "function/geo_quat_from_axis_angle.h"
#include "function/geo_quat_inverse.h"
#include "function/geo_quat_multiply.h"
#include "function/geo_quat_rotate.h"
#include "function/geo_quat_to_matrix.h"
#include "function/geo_within_bbox.h"
#include "function/geo_within_bbox_nd.h"
#include "function/geo_within_circle.h"
#include "function/geo_within_convex.h"
#include "function/geo_within_frustum.h"
#include "function/geo_within_obb.h"
#include "function/geo_within_obb_matrix.h"
#include "function/geo_within_polygon_3d.h"
#include "function/geo_within_polygon_3d_matrix.h"
#include "function/geo_within_sphere.h"
#include "function/query_spatial_index.h"
#include "index/rtree_index.h"
#include "main/client_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

namespace rag3db {
namespace geo_extension {

using namespace extension;

static void initSpatialEntries(main::ClientContext* context, catalog::Catalog& catalog) {
    auto storageManager = storage::StorageManager::Get(*context);
    for (auto& indexEntry : catalog.getIndexEntries(transaction::Transaction::Get(*context))) {
        if (indexEntry->getIndexType() == SpatialIndexCatalogEntry::TYPE_NAME &&
            !indexEntry->isLoaded()) {
            indexEntry->setAuxInfo(
                SpatialIndexAuxInfo::deserialize(indexEntry->getAuxBufferReader()));
            auto& nodeTable =
                storageManager->getTable(indexEntry->getTableID())->cast<storage::NodeTable>();
            auto optionalIndex = nodeTable.getIndexHolder(indexEntry->getIndexName());
            KU_ASSERT_UNCONDITIONAL(
                optionalIndex.has_value() && !optionalIndex.value().get().isLoaded());
            auto& unloadedIndex = optionalIndex.value().get();
            unloadedIndex.load(context, storageManager);
        }
    }
}

void GeoExtension::load(main::ClientContext* context) {
    auto& db = *context->getDatabase();

    // Scalar functions — distance.
    ExtensionUtils::addScalarFunc<GeoDistanceFunction>(db);
    ExtensionUtils::addScalarFunc<GeoDistanceEuclideanFunction>(db);

    // Scalar functions — 2D containment.
    ExtensionUtils::addScalarFunc<GeoContainsFunction>(db);
    ExtensionUtils::addScalarFunc<GeoWithinBboxFunction>(db);
    ExtensionUtils::addScalarFunc<GeoWithinBboxNdFunction>(db);
    ExtensionUtils::addScalarFunc<GeoWithinCircleFunction>(db);
    ExtensionUtils::addScalarFunc<GeoWithinSphereFunction>(db);

    // Scalar functions — quaternion.
    ExtensionUtils::addScalarFunc<GeoQuatRotateFunction>(db);
    ExtensionUtils::addScalarFunc<GeoQuatInverseFunction>(db);
    ExtensionUtils::addScalarFunc<GeoQuatMultiplyFunction>(db);
    ExtensionUtils::addScalarFunc<GeoQuatFromAxisAngleFunction>(db);
    ExtensionUtils::addScalarFunc<GeoQuatToMatrixFunction>(db);

    // Scalar functions — matrix.
    ExtensionUtils::addScalarFunc<GeoMatrixRotateFunction>(db);
    ExtensionUtils::addScalarFunc<GeoMatrixMultiplyFunction>(db);
    ExtensionUtils::addScalarFunc<GeoMatrixTransposeFunction>(db);

    // Scalar functions — 3D containment.
    ExtensionUtils::addScalarFunc<GeoWithinObbFunction>(db);
    ExtensionUtils::addScalarFunc<GeoWithinObbMatrixFunction>(db);
    ExtensionUtils::addScalarFunc<GeoWithinPolygon3dFunction>(db);
    ExtensionUtils::addScalarFunc<GeoWithinPolygon3dMatrixFunction>(db);

    // Scalar functions — frustum / convex.
    ExtensionUtils::addScalarFunc<GeoWithinFrustumFunction>(db);
    ExtensionUtils::addScalarFunc<GeoWithinConvexFunction>(db);
    ExtensionUtils::addScalarFunc<GeoFrustumFromCameraFunction>(db);

    // Table functions.
    ExtensionUtils::addStandaloneTableFunc<CreateSpatialIndexFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalCreateSpatialIndexFunction>(db);
    ExtensionUtils::addTableFunc<QuerySpatialIndexFunction>(db);
    ExtensionUtils::addStandaloneTableFunc<DropSpatialIndexFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalDropSpatialIndexFunction>(db);

    // Index type.
    ExtensionUtils::registerIndexType(db, RTreeIndex::getIndexType());
    initSpatialEntries(context, *db.getCatalog());
}

} // namespace geo_extension
} // namespace rag3db

#if defined(BUILD_DYNAMIC_LOAD)
extern "C" {
#if defined(_WIN32)
#define INIT_EXPORT __declspec(dllexport)
#else
#define INIT_EXPORT __attribute__((visibility("default")))
#endif
INIT_EXPORT void init(rag3db::main::ClientContext* context) {
    rag3db::geo_extension::GeoExtension::load(context);
}
INIT_EXPORT const char* name() {
    return rag3db::geo_extension::GeoExtension::EXTENSION_NAME;
}
}
#endif
