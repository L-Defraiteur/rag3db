#include "main/lucivy_fts_extension.h"

#include "catalog/catalog.h"
#include "catalog/lucivy_index_catalog_entry.h"
#include "function/create_lucivy_index.h"
#include "function/drop_lucivy_index.h"
#include "function/flush_lucivy_index.h"
#include "function/query_lucivy_index.h"
#include "function/search_function.h"
#include "index/lucivy_index.h"
#include "main/client_context.h"
#include "storage/storage_manager.h"
#include "storage/table/node_table.h"

namespace rag3db {
namespace lucivy_fts_extension {

using namespace extension;

static void initLucivyEntries(main::ClientContext* context, catalog::Catalog& catalog) {
    auto storageManager = storage::StorageManager::Get(*context);
    for (auto& indexEntry : catalog.getIndexEntries(transaction::Transaction::Get(*context))) {
        if (indexEntry->getIndexType() == LucivyIndexCatalogEntry::TYPE_NAME &&
            !indexEntry->isLoaded()) {
            indexEntry->setAuxInfo(
                LucivyIndexAuxInfo::deserialize(indexEntry->getAuxBufferReader()));
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

void LucivyFtsExtension::load(main::ClientContext* context) {
    auto& db = *context->getDatabase();
    ExtensionUtils::addTableFunc<QueryLucivyFunction>(db);
    ExtensionUtils::addStandaloneTableFunc<CreateLucivyFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalCreateLucivyFunction>(db);
    ExtensionUtils::addStandaloneTableFunc<DropLucivyFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalDropLucivyFunction>(db);
    ExtensionUtils::addStandaloneTableFunc<FlushLucivyFunction>(db);
    ExtensionUtils::addInternalStandaloneTableFunc<InternalFlushLucivyFunction>(db);
    ExtensionUtils::addScalarFunc<SearchFunction>(db);
    ExtensionUtils::addScalarFunc<SearchScoreFunction>(db);
    ExtensionUtils::addScalarFunc<SearchHighlightsFunction>(db);
    ExtensionUtils::registerIndexType(db, LucivyIndex::getIndexType());
    initLucivyEntries(context, *db.getCatalog());
}

} // namespace lucivy_fts_extension
} // namespace rag3db

#if defined(BUILD_DYNAMIC_LOAD)
extern "C" {
#if defined(_WIN32)
#define INIT_EXPORT __declspec(dllexport)
#else
#define INIT_EXPORT __attribute__((visibility("default")))
#endif
INIT_EXPORT void init(rag3db::main::ClientContext* context) {
    rag3db::lucivy_fts_extension::LucivyFtsExtension::load(context);
}
INIT_EXPORT const char* name() {
    return rag3db::lucivy_fts_extension::LucivyFtsExtension::EXTENSION_NAME;
}
}
#endif
