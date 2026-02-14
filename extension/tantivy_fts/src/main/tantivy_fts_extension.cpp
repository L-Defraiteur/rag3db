#include "main/tantivy_fts_extension.h"

#include "catalog/catalog.h"
#include "catalog/tantivy_index_catalog_entry.h"
#include "function/create_tantivy_index.h"
#include "function/drop_tantivy_index.h"
#include "function/query_tantivy_index.h"
#include "index/tantivy_index.h"
#include "main/client_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

namespace rag3db {
namespace tantivy_fts_extension {

using namespace extension;

static void initTantivyEntries(main::ClientContext* context, catalog::Catalog& catalog) {
    auto storageManager = storage::StorageManager::Get(*context);
    for (auto& indexEntry : catalog.getIndexEntries(transaction::Transaction::Get(*context))) {
        if (indexEntry->getIndexType() == TantivyIndexCatalogEntry::TYPE_NAME &&
            !indexEntry->isLoaded()) {
            indexEntry->setAuxInfo(
                TantivyIndexAuxInfo::deserialize(indexEntry->getAuxBufferReader()));
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

void TantivyFtsExtension::load(main::ClientContext* context) {
    auto& db = *context->getDatabase();
    ExtensionUtils::addTableFunc<QueryTantivyFunction>(db);
    ExtensionUtils::addStandaloneTableFunc<CreateTantivyFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalCreateTantivyFunction>(db);
    ExtensionUtils::addStandaloneTableFunc<DropTantivyFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalDropTantivyFunction>(db);
    ExtensionUtils::registerIndexType(db, TantivyIndex::getIndexType());
    initTantivyEntries(context, *db.getCatalog());
}

} // namespace tantivy_fts_extension
} // namespace rag3db

#if defined(BUILD_DYNAMIC_LOAD)
extern "C" {
#if defined(_WIN32)
#define INIT_EXPORT __declspec(dllexport)
#else
#define INIT_EXPORT __attribute__((visibility("default")))
#endif
INIT_EXPORT void init(rag3db::main::ClientContext* context) {
    rag3db::tantivy_fts_extension::TantivyFtsExtension::load(context);
}
INIT_EXPORT const char* name() {
    return rag3db::tantivy_fts_extension::TantivyFtsExtension::EXTENSION_NAME;
}
}
#endif
