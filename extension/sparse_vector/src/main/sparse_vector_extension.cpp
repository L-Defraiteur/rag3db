#include "main/sparse_vector_extension.h"

#include "catalog/catalog.h"
#include "catalog/sparse_vector_catalog_entry.h"
#include "function/create_sparse_vector_index.h"
#include "function/drop_sparse_vector_index.h"
#include "function/query_sparse_vector_index.h"
#include "index/sparse_vector_index.h"
#include "main/client_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

namespace rag3db {
namespace sparse_vector_extension {

using namespace extension;

static void initSparseVectorEntries(main::ClientContext* context, catalog::Catalog& catalog) {
    auto storageManager = storage::StorageManager::Get(*context);
    for (auto& indexEntry : catalog.getIndexEntries(transaction::Transaction::Get(*context))) {
        if (indexEntry->getIndexType() == SparseVectorCatalogEntry::TYPE_NAME &&
            !indexEntry->isLoaded()) {
            indexEntry->setAuxInfo(
                SparseVectorIndexAuxInfo::deserialize(indexEntry->getAuxBufferReader()));
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

void SparseVectorExtension::load(main::ClientContext* context) {
    auto& db = *context->getDatabase();
    ExtensionUtils::addTableFunc<QuerySparseVectorFunction>(db);
    ExtensionUtils::addStandaloneTableFunc<CreateSparseVectorFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalCreateSparseVectorFunction>(db);
    ExtensionUtils::addStandaloneTableFunc<DropSparseVectorFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalDropSparseVectorFunction>(db);
    ExtensionUtils::registerIndexType(db, SparseVectorIndex::getIndexType());
    initSparseVectorEntries(context, *db.getCatalog());
}

} // namespace sparse_vector_extension
} // namespace rag3db

#if defined(BUILD_DYNAMIC_LOAD)
extern "C" {
#if defined(_WIN32)
#define INIT_EXPORT __declspec(dllexport)
#else
#define INIT_EXPORT __attribute__((visibility("default")))
#endif
INIT_EXPORT void init(rag3db::main::ClientContext* context) {
    rag3db::sparse_vector_extension::SparseVectorExtension::load(context);
}
INIT_EXPORT const char* name() {
    return rag3db::sparse_vector_extension::SparseVectorExtension::EXTENSION_NAME;
}
}
#endif
