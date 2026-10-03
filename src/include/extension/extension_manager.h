#pragma once

#include "loaded_extension.h"
#include "storage/storage_extension.h"

namespace rag3db {
namespace main {}
namespace extension {

struct ExtensionEntry {
    const char* name;
    const char* extensionName;
};

class ExtensionManager {
public:
    void loadExtension(const std::string& path, main::ClientContext* context);

    RAG3DB_API std::string toCypher();

    RAG3DB_API void addExtensionOption(std::string name, common::LogicalTypeID type,
        common::Value defaultValue, bool isConfidential);

    const main::ExtensionOption* getExtensionOption(std::string name) const;

    RAG3DB_API void registerStorageExtension(std::string name,
        std::unique_ptr<storage::StorageExtension> storageExtension);

    std::vector<storage::StorageExtension*> getStorageExtensions();

    // Une extension que la reprise n'a pas pu charger : son nom s'il est connu, ce qui a été
    // demandé (un chemin, ou le nom d'une extension officielle), et l'erreur.
    struct RecoveryLoadFailure {
        std::string name;
        std::string path;
        std::string error;
    };

    // Le rejeu du journal a lieu à l'ouverture, avant que l'appelant puisse charger quoi que
    // ce soit ; il a pourtant besoin des extensions dont viennent les index des tables où il
    // écrit. Le journal note bien chaque LOAD EXTENSION, mais un point de reprise le vide.
    // Chaque extension chargée est donc aussi notée dans un fichier à côté de la base
    // (StorageUtils::getExtensionsFilePath), que le rejeu relit avant de rejouer. Ce fichier
    // ne peut qu'aider : absent, illisible, ou portant un chemin qui ne vaut plus, la reprise
    // continue sans l'extension, et l'index qu'elle n'a pas pu tenir à jour se déclare à
    // rebâtir (IndexHolder::detach). Rien n'est chargé d'office quand il n'y a pas de journal
    // à rejouer.
    //
    // Charger une bibliothèque depuis un fichier posé à côté de la base, c'est exécuter du
    // code pour qui peut écrire dans ce dossier. Ce n'est pas nouveau — le journal, posé au
    // même endroit, fait charger ses LOAD EXTENSION de la même façon — mais c'est à savoir
    // le jour où une base vient d'ailleurs.
    RAG3DB_API void loadExtensionsNotedBesideTheDatabase(main::ClientContext* context);
    // Charge pour le rejeu ; un échec est retenu au lieu d'empêcher d'ouvrir la base.
    void loadExtensionForRecovery(const std::string& name, const std::string& path,
        main::ClientContext* context);
    // Ce que la reprise n'a pas pu charger, pour l'appelant et pour les messages d'erreur des
    // index restés en retard.
    RAG3DB_API const std::vector<RecoveryLoadFailure>& getRecoveryLoadFailures() const {
        return recoveryLoadFailures;
    }

    RAG3DB_API const std::vector<LoadedExtension>& getLoadedExtensions() const {
        return loadedExtensions;
    }

    static std::optional<ExtensionEntry> lookupExtensionsByFunctionName(
        std::string_view functionName);
    static std::optional<ExtensionEntry> lookupExtensionsByTypeName(std::string_view typeName);

    void autoLoadLinkedExtensions(main::ClientContext* context);

    RAG3DB_API static ExtensionManager* Get(const main::ClientContext& context);

private:
    std::vector<LoadedExtension> loadedExtensions;
    // Ce qui est noté à côté de la base : le nom de chaque extension et ce qu'il faut redonner
    // à loadExtension pour la recharger.
    struct NotedExtension {
        std::string name;
        std::string path;
    };
    std::vector<NotedExtension> notedExtensions;
    std::vector<RecoveryLoadFailure> recoveryLoadFailures;
    void noteBesideTheDatabase(const std::string& name, const std::string& path,
        main::ClientContext* context);
    static std::vector<NotedExtension> readNotedExtensions(const std::string& filePath);
    std::unordered_map<std::string, main::ExtensionOption> extensionOptions;
    common::case_insensitive_map_t<std::unique_ptr<storage::StorageExtension>> storageExtensions;
};

} // namespace extension
} // namespace rag3db
