#include "main/vector_extension.h"

#include <chrono>
#include <cstdlib>
#include <thread>

#include "catalog/hnsw_index_catalog_entry.h"
#include "function/hnsw_index_functions.h"
#include "function/vector_search_function.h"
#include "main/client_context.h"
#include "main/database.h"
#include "storage/storage_manager.h"

namespace rag3db {
namespace vector_extension {

static void initHNSWEntries(main::ClientContext* context) {
    auto storageManager = storage::StorageManager::Get(*context);
    auto catalog = catalog::Catalog::Get(*context);
    for (auto& indexEntry : catalog->getIndexEntries(transaction::Transaction::Get(*context))) {
        if (indexEntry->getIndexType() == HNSWIndexCatalogEntry::TYPE_NAME &&
            !indexEntry->isLoaded()) {
            indexEntry->setAuxInfo(HNSWIndexAuxInfo::deserialize(indexEntry->getAuxBufferReader()));
            // Should load the index in storage side as well.
            auto& nodeTable =
                storageManager->getTable(indexEntry->getTableID())->cast<storage::NodeTable>();
            // Sans exemplaire dans la table, une entrée au catalogue n'a rien à charger : le rejeu
            // du journal a écrit dans la table pendant que cette extension n'était pas chargée, et
            // l'index a été détaché. Il est à rebâtir, et HNSWIndexUtils le dit à qui veut s'en
            // servir. Le chargement se fait sous le verrou de la liste des index.
            nodeTable.loadIndex(indexEntry->getIndexName(), context, storageManager);
        }
    }
}

// Le crochet du témoin du chargement (IndexReopen.AQueryDuringTheLoadIsNotToldTheIndexIsGone) :
// une pause juste après l'enregistrement des fonctions, en millisecondes, lue dans
// RAG3DB_TEST_PAUSE_AFTER_VECTOR_FUNCTIONS_MS. Une requête s'y glisse, liée aux fonctions qu'elle
// voit. Sans la variable, rien.
static void pauseAfterTheFunctionsForATest() {
    if (const auto* ms = std::getenv("RAG3DB_TEST_PAUSE_AFTER_VECTOR_FUNCTIONS_MS")) {
        std::this_thread::sleep_for(std::chrono::milliseconds(std::atoi(ms)));
    }
}

void VectorExtension::load(main::ClientContext* context) {
    auto& db = *context->getDatabase();
    // Les index d'abord, les fonctions ensuite : une requête ne peut se lier à QUERY_VECTOR_INDEX
    // qu'une fois les index chargés. Enregistrées avant, elles laissaient une fenêtre où la
    // requête liée trouvait l'index pas encore chargé et le prenait pour un index retiré (ticket
    // 2026-10-19-un-chargement-d-extension-a-la-reouverture-se-fait-passer-pour-un-retrait-de-l-index).
    extension::ExtensionUtils::registerIndexType(db, OnDiskHNSWIndex::getIndexType());
    initHNSWEntries(context);
    extension::ExtensionUtils::addTableFunc<QueryVectorIndexFunction>(db);
    extension::ExtensionUtils::addInternalStandaloneTableFunc<InternalCreateHNSWIndexFunction>(db);
    extension::ExtensionUtils::addInternalStandaloneTableFunc<InternalFinalizeHNSWIndexFunction>(
        db);
    extension::ExtensionUtils::addStandaloneTableFunc<CreateVectorIndexFunction>(db);
    extension::ExtensionUtils::addInternalStandaloneTableFunc<InternalDropHNSWIndexFunction>(db);
    extension::ExtensionUtils::addStandaloneTableFunc<DropVectorIndexFunction>(db);
    extension::ExtensionUtils::addScalarFunc<VectorSearchFunction>(db);
    extension::ExtensionUtils::addScalarFunc<VectorDistanceFunction>(db);
    pauseAfterTheFunctionsForATest();
}

} // namespace vector_extension
} // namespace rag3db

#if defined(BUILD_DYNAMIC_LOAD)
#include "common/build_id.h"

extern "C" {
// Because we link against the static library on windows, we implicitly inherit RAG3DB_STATIC_DEFINE,
// which cancels out any exporting, so we can't use RAG3DB_API.
#if defined(_WIN32)
#define INIT_EXPORT __declspec(dllexport)
#else
#define INIT_EXPORT __attribute__((visibility("default")))
#endif
INIT_EXPORT void init(rag3db::main::ClientContext* context) {
    rag3db::vector_extension::VectorExtension::load(context);
}

INIT_EXPORT const char* name() {
    return rag3db::vector_extension::VectorExtension::EXTENSION_NAME;
}

// L'identifiant de bâti de cette extension, que LOAD EXTENSION compare à celui du moteur.
INIT_EXPORT const char* build_id() {
    return RAG3DB_BUILD_ID;
}
}
#endif
