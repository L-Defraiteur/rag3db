#include "extension/extension_manager.h"

#include <algorithm>
#include <filesystem>
#include <fstream>

#include "common/file_system/virtual_file_system.h"
#include "common/string_utils.h"
#include "extension/extension.h"
#include "generated_extension_loader.h"
#include "main/client_context.h"
#include "main/db_config.h"
#include "storage/storage_utils.h"
#include "storage/wal/local_wal.h"
#include "transaction/transaction.h"
#include "transaction/transaction_context.h"

namespace rag3db {
namespace extension {

static void executeExtensionLoader(main::ClientContext* context, const std::string& extensionName) {
    auto loaderPath = ExtensionUtils::getLocalPathForExtensionLoader(context, extensionName);
    if (common::VirtualFileSystem::GetUnsafe(*context)->fileOrPathExists(loaderPath)) {
        auto libLoader = ExtensionLibLoader(extensionName, loaderPath);
        auto load = libLoader.getLoadFunc();
        (*load)(context);
    }
}

void ExtensionManager::loadExtension(const std::string& path, main::ClientContext* context) {
    auto fullPath = path;
    bool isOfficial = ExtensionUtils::isOfficialExtension(path);
    if (isOfficial) {
        auto localPathForSharedLib = ExtensionUtils::getLocalPathForSharedLib(context);
        if (!common::VirtualFileSystem::GetUnsafe(*context)->fileOrPathExists(
                localPathForSharedLib)) {
            common::VirtualFileSystem::GetUnsafe(*context)->createDir(localPathForSharedLib);
        }
        executeExtensionLoader(context, path);
        fullPath = ExtensionUtils::getLocalPathForExtensionLib(context, path);
    }

    auto libLoader = ExtensionLibLoader(path, fullPath);
    auto name = libLoader.getNameFunc();
    std::string extensionName = (*name)();
    if (std::any_of(loadedExtensions.begin(), loadedExtensions.end(),
            [&](const LoadedExtension& ext) { return ext.getExtensionName() == extensionName; })) {
        libLoader.unload();
        return;
    }
    auto init = libLoader.getInitFunc();
    (*init)(context);
    loadedExtensions.push_back(LoadedExtension(extensionName, fullPath,
        isOfficial ? ExtensionSource::OFFICIAL : ExtensionSource::USER));
    auto transaction = transaction::Transaction::Get(*context);
    if (transaction->shouldLogToWAL()) {
        transaction->getLocalWAL().logLoadExtension(path);
    }
    if (!transaction->isRecovery()) {
        noteBesideTheDatabase(extensionName, path, context);
    }
}

std::vector<ExtensionManager::NotedExtension> ExtensionManager::readNotedExtensions(
    const std::string& filePath) {
    std::vector<NotedExtension> noted;
    std::ifstream file(filePath);
    std::string line;
    // Une ligne par extension : son nom, une tabulation, ce qu'il faut redonner à LOAD.
    while (file && std::getline(file, line)) {
        const auto tab = line.find('\t');
        if (tab == std::string::npos || tab == 0 || tab + 1 >= line.size()) {
            continue;
        }
        noted.push_back(NotedExtension{line.substr(0, tab), line.substr(tab + 1)});
    }
    return noted;
}

void ExtensionManager::noteBesideTheDatabase(const std::string& name, const std::string& path,
    main::ClientContext* context) {
    if (context->isInMemory() || context->getDBConfig()->readOnly) {
        return;
    }
    const auto filePath = storage::StorageUtils::getExtensionsFilePath(context->getDatabasePath());
    // Ce que le fichier porte déjà reste : une extension chargée par une session d'avant sert
    // encore à la reprise de celle-ci.
    if (notedExtensions.empty()) {
        notedExtensions = readNotedExtensions(filePath);
    }
    const auto known = std::ranges::find_if(notedExtensions,
        [&](const NotedExtension& noted) { return noted.name == name; });
    if (known != notedExtensions.end()) {
        if (known->path == path) {
            return;
        }
        known->path = path; // la même extension, chargée d'un autre endroit : le dernier vaut
    } else {
        notedExtensions.push_back(NotedExtension{name, path});
    }
    // Écrit à part puis renommé : un arrêt au milieu laisse l'ancien fichier, jamais un bout.
    // Ne pas réussir à l'écrire n'empêche rien : on y perd seulement ce que le fichier apporte.
    const auto tmpPath = filePath + ".tmp";
    {
        std::ofstream file(tmpPath, std::ios::trunc);
        for (const auto& noted : notedExtensions) {
            file << noted.name << '\t' << noted.path << '\n';
        }
        if (!file) {
            return;
        }
    }
    std::error_code error;
    std::filesystem::rename(tmpPath, filePath, error);
}

void ExtensionManager::loadExtensionForRecovery(const std::string& name, const std::string& path,
    main::ClientContext* context) {
    // La liste et le journal peuvent désigner le même fichier : un échec ne se dit qu'une fois.
    if (std::ranges::any_of(recoveryLoadFailures,
            [&](const RecoveryLoadFailure& failure) { return failure.path == path; })) {
        return;
    }
    try {
        loadExtension(path, context);
    } catch (const std::exception& e) {
        recoveryLoadFailures.push_back(RecoveryLoadFailure{name, path, e.what()});
    }
}

void ExtensionManager::loadExtensionsNotedBesideTheDatabase(main::ClientContext* context) {
    if (context->isInMemory()) {
        return;
    }
    const auto noted = readNotedExtensions(
        storage::StorageUtils::getExtensionsFilePath(context->getDatabasePath()));
    if (noted.empty()) {
        return;
    }
    auto trxContext = transaction::TransactionContext::Get(*context);
    trxContext->beginRecoveryTransaction();
    for (const auto& extension : noted) {
        const auto alreadyLoaded = std::ranges::any_of(loadedExtensions,
            [&](const LoadedExtension& loaded) {
                return loaded.getExtensionName() == extension.name;
            });
        if (!alreadyLoaded) {
            loadExtensionForRecovery(extension.name, extension.path, context);
        }
    }
    trxContext->commit();
}

std::string ExtensionManager::toCypher() {
    std::string cypher;
    for (auto& extension : loadedExtensions) {
        cypher += extension.toCypher();
    }
    return cypher;
}

void ExtensionManager::addExtensionOption(std::string name, common::LogicalTypeID type,
    common::Value defaultValue, bool isConfidential) {
    if (getExtensionOption(name) != nullptr) {
        // One extension option can be shared by multiple extensions.
        return;
    }
    common::StringUtils::toLower(name);
    extensionOptions.emplace(name,
        main::ExtensionOption{name, type, std::move(defaultValue), isConfidential});
}

const main::ExtensionOption* ExtensionManager::getExtensionOption(std::string name) const {
    common::StringUtils::toLower(name);
    return extensionOptions.contains(name) ? &extensionOptions.at(name) : nullptr;
}

void ExtensionManager::registerStorageExtension(std::string name,
    std::unique_ptr<storage::StorageExtension> storageExtension) {
    if (storageExtensions.contains(name)) {
        return;
    }
    storageExtensions.emplace(std::move(name), std::move(storageExtension));
}

std::vector<storage::StorageExtension*> ExtensionManager::getStorageExtensions() {
    std::vector<storage::StorageExtension*> storageExtensionsToReturn;
    for (auto& [name, storageExtension] : storageExtensions) {
        storageExtensionsToReturn.push_back(storageExtension.get());
    }
    return storageExtensionsToReturn;
}

void ExtensionManager::autoLoadLinkedExtensions(main::ClientContext* context) {
    auto trxContext = transaction::TransactionContext::Get(*context);
    trxContext->beginRecoveryTransaction();
    loadLinkedExtensions(context, loadedExtensions);
    trxContext->commit();
}

ExtensionManager* ExtensionManager::Get(const main::ClientContext& context) {
    return context.getDatabase()->getExtensionManager();
}

} // namespace extension
} // namespace rag3db
