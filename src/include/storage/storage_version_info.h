#pragma once

#include <cstdint>
#include <string>
#include <unordered_map>

#include "common/api.h"

namespace rag3db {
namespace storage {

using storage_version_t = uint64_t;

struct StorageVersionInfo {
    static std::unordered_map<std::string, storage_version_t> getStorageVersionInfo() {
        return {{"0.11.2.2", 40}, {"0.11.1", 39}, {"0.11.0", 39}, {"0.10.0", 38}, {"0.9.0", 37}, {"0.8.0", 36},
            {"0.7.1.1", 35}, {"0.7.0", 34}, {"0.6.0.6", 33}, {"0.6.0.5", 32}, {"0.6.0.2", 31},
            {"0.6.0.1", 31}, {"0.6.0", 28}, {"0.5.0", 28}, {"0.4.2", 27}, {"0.4.1", 27},
            {"0.4.0", 27}, {"0.3.2", 26}, {"0.3.1", 26}, {"0.3.0", 26}, {"0.2.1", 25},
            {"0.2.0", 25}, {"0.1.0", 24}, {"0.0.12.3", 24}, {"0.0.12.2", 24}, {"0.0.12.1", 24},
            {"0.0.12", 23}, {"0.0.11", 23}, {"0.0.10", 23}, {"0.0.9", 23}, {"0.0.8", 17},
            {"0.0.7", 15}, {"0.0.6", 9}, {"0.0.5", 8}, {"0.0.4", 7}, {"0.0.3", 1}};
    }

    // **La version du format**, posée à la main à chaque changement de ce qui est écrit (le
    // CATALOG_VERSION_NO de PostgreSQL), indépendante de la chaîne de version du produit : la
    // table ci-dessus ne sert plus qu'à dire ce que les versions du produit ont écrit.
    //   39 : avant l'étendue du fichier de données ;
    //   40 : l'en-tête porte l'étendue du fichier de données au point de reprise
    //        (DatabaseHeader::numDataPages) ;
    //   41 : l'en-tête porte le bâti du dernier moteur écrivain (juste après la version, et
    //        toute version suivante garde cet ordre : un moteur plus ancien lit les deux avant
    //        de refuser) ; le journal commence par un enregistrement de format (FormatRecord) ;
    //   42 : les numéros de colonnes ne changent jamais (plus de renumérotation au point de
    //        reprise) ; un bloc sur disque garde une entrée par numéro, et dit pour chacune si
    //        la colonne y est (une colonne supprimée laisse un trou).
    // Une base plus récente que ce moteur est refusée en le disant, en lecture comme en
    // écriture ; une base plus ancienne, jusqu'à OLDEST_READABLE_STORAGE_VERSION, s'ouvre et son
    // premier point de reprise la passe à la version courante, au bâti courant.
    static constexpr storage_version_t STORAGE_FORMAT_VERSION = 42;
    static constexpr storage_version_t OLDEST_READABLE_STORAGE_VERSION = 39;
    static constexpr storage_version_t VERSION_BEFORE_DATA_FILE_EXTENT = 39;
    static constexpr storage_version_t VERSION_BEFORE_WRITER_BUILD_ID = 40;
    static constexpr storage_version_t VERSION_BEFORE_COLUMN_HOLES = 41;
    static RAG3DB_API storage_version_t getStorageVersion();
    // L'identifiant de bâti de ce moteur (cmake/build_id.cmake), celui que l'en-tête de base et
    // le journal portent comme « dernier écrivain ».
    static RAG3DB_API std::string getBuildID();

    static constexpr const char* MAGIC_BYTES = "RAG3DB";
};

} // namespace storage
} // namespace rag3db
