#pragma once

#include <functional>

#include "common/serializer/buffer_writer.h"
#include "common/types/types.h"
#include "common/vector/value_vector.h"
#include "in_mem_hash_index.h"
#include <span>

namespace rag3db::storage {
class StorageManager;
}
namespace rag3db {
namespace transaction {
class Transaction;
} // namespace transaction

namespace storage {

enum class RAG3DB_API IndexConstraintType : uint8_t {
    PRIMARY = 0,              // Primary key index
    SECONDARY_NON_UNIQUE = 1, // Secondary index that is not unique
};

enum class RAG3DB_API IndexDefinitionType : uint8_t {
    BUILTIN = 0,
    EXTENSION = 1,
};

class Index;
struct IndexInfo;
using index_load_func_t = std::function<std::unique_ptr<Index>(main::ClientContext* context,
    StorageManager* storageManager, IndexInfo, std::span<uint8_t>)>;

struct RAG3DB_API IndexType {
    std::string typeName;
    IndexConstraintType constraintType;
    IndexDefinitionType definitionType;
    index_load_func_t loadFunc;

    IndexType(std::string typeName, IndexConstraintType constraintType,
        IndexDefinitionType definitionType, index_load_func_t loadFunc)
        : typeName{std::move(typeName)}, constraintType{constraintType},
          definitionType{definitionType}, loadFunc{std::move(loadFunc)} {}
};

struct RAG3DB_API IndexInfo {
    std::string name;
    std::string indexType;
    common::table_id_t tableID;
    std::vector<common::column_id_t> columnIDs;
    std::vector<common::PhysicalTypeID> keyDataTypes;
    bool isPrimary;
    bool isBuiltin;

    IndexInfo(std::string name, std::string indexType, common::table_id_t tableID,
        std::vector<common::column_id_t> columnIDs,
        std::vector<common::PhysicalTypeID> keyDataTypes, bool isPrimary, bool isBuiltin)
        : name{std::move(name)}, indexType{std::move(indexType)}, tableID{tableID},
          columnIDs{std::move(columnIDs)}, keyDataTypes{std::move(keyDataTypes)},
          isPrimary{isPrimary}, isBuiltin{isBuiltin} {}

    void serialize(common::Serializer& ser) const;
    static IndexInfo deserialize(common::Deserializer& deSer);
};

struct RAG3DB_API IndexStorageInfo {
    IndexStorageInfo() {}
    virtual ~IndexStorageInfo();
    DELETE_COPY_DEFAULT_MOVE(IndexStorageInfo);

    virtual std::shared_ptr<common::BufferWriter> serialize() const;

    template<typename TARGET>
    TARGET& cast() {
        return common::ku_dynamic_cast<TARGET&>(*this);
    }

    template<typename TARGET>
    const TARGET& constCast() const {
        return common::ku_dynamic_cast<const TARGET&>(*this);
    }
};

class RAG3DB_API Index {
public:
    struct InsertState {
        virtual ~InsertState();
        template<typename TARGET>
        TARGET& cast() {
            return common::ku_dynamic_cast<TARGET&>(*this);
        }
    };

    struct DeleteState {
        virtual ~DeleteState();
        template<typename TARGET>
        TARGET& cast() {
            return common::ku_dynamic_cast<TARGET&>(*this);
        }
    };

    struct UpdateState {
        virtual ~UpdateState();
        template<typename TARGET>
        TARGET& cast() {
            return common::ku_dynamic_cast<TARGET&>(*this);
        }
    };

    Index(IndexInfo indexInfo, std::unique_ptr<IndexStorageInfo> storageInfo)
        : indexInfo{std::move(indexInfo)}, storageInfo{std::move(storageInfo)},
          storageInfoBuffer{nullptr}, storageInfoBufferSize{0}, loaded{true} {}
    Index(IndexInfo indexInfo, std::unique_ptr<uint8_t[]> storageBuffer, uint32_t storageBufferSize)
        : indexInfo{std::move(indexInfo)}, storageInfo{nullptr},
          storageInfoBuffer{std::move(storageBuffer)}, storageInfoBufferSize{storageBufferSize},
          loaded{false} {}
    virtual ~Index();

    DELETE_COPY_AND_MOVE(Index);

    bool isPrimary() const { return indexInfo.isPrimary; }
    bool isExtension() const { return indexInfo.isBuiltin; }
    bool isLoaded() const { return loaded; }
    std::string getName() const { return indexInfo.name; }
    IndexInfo getIndexInfo() const { return indexInfo; }
    // Les numéros de colonnes de l'index, recalculés depuis le catalogue après une
    // renumérotation (NodeTable::renumberColumns).
    void setColumnIDs(std::vector<common::column_id_t> columnIDs) {
        indexInfo.columnIDs = std::move(columnIDs);
    }

    virtual std::unique_ptr<InsertState> initInsertState(main::ClientContext* context,
        visible_func isVisible) = 0;
    virtual void insert(transaction::Transaction*, const common::ValueVector&,
        const std::vector<common::ValueVector*>&, InsertState&) {
        // DO NOTHING.
    }
    virtual std::unique_ptr<UpdateState> initUpdateState(main::ClientContext* /*context*/,
        common::column_id_t /*columnID*/, visible_func /*isVisible*/) {
        KU_UNREACHABLE;
    }
    virtual void update(transaction::Transaction* /*transaction*/,
        const common::ValueVector& /*nodeIDVector*/, common::ValueVector& /*propertyVector*/,
        UpdateState& /*updateState*/) {
        KU_UNREACHABLE;
    }
    virtual std::unique_ptr<DeleteState> initDeleteState(
        const transaction::Transaction* transaction, MemoryManager* mm, visible_func isVisible) = 0;
    virtual void delete_(transaction::Transaction* transaction,
        const common::ValueVector& nodeIDVector, DeleteState& deleteState) = 0;
    virtual void finalizeDelete(transaction::Transaction* /*transaction*/,
        DeleteState& /*deleteState*/) {
        // DO NOTHING. Override in extensions that need batched cleanup (e.g., HNSW).
    }
    virtual bool needCommitInsert() const { return false; }
    virtual void commitInsert(transaction::Transaction*, const common::ValueVector&,
        const std::vector<common::ValueVector*>&, InsertState&) {
        // DO NOTHING.
    }

    // L'annulation d'une transaction retire de la table les lignes à partir de ce décalage
    // (un COPY annulé, des lignes versées puis annulées, un commit qui échoue après avoir
    // nourri l'index). Un index qui tient un état en mémoire d'après les lignes qu'il a
    // reçues le ramène ici à ce que la table contient encore.
    virtual void rollbackInsert(common::offset_t /*firstRolledBackOffset*/) {
        // DO NOTHING.
    }

    virtual void checkpointInMemory() {
        // DO NOTHING.
    };
    virtual void checkpoint(main::ClientContext*, PageAllocator&) {
        // DO NOTHING.
    }
    virtual void rollbackCheckpoint() {
        // DO NOTHING.
    }
    virtual void finalize(main::ClientContext*) {
        // DO NOTHING.
    }

    std::span<uint8_t> getStorageBuffer() const {
        KU_ASSERT(!loaded);
        return std::span(storageInfoBuffer.get(), storageInfoBufferSize);
    }
    const IndexStorageInfo& getStorageInfo() const { return *storageInfo; }
    bool isBuiltOnColumn(common::column_id_t columnID) const;

    virtual void serialize(common::Serializer& ser) const;

    template<typename TARGET>
    TARGET& cast() {
        return common::ku_dynamic_cast<TARGET&>(*this);
    }

protected:
    IndexInfo indexInfo;
    std::unique_ptr<IndexStorageInfo> storageInfo;
    std::unique_ptr<uint8_t[]> storageInfoBuffer;
    uint64_t storageInfoBufferSize;
    bool loaded;
};

class IndexHolder {
public:
    explicit IndexHolder(std::unique_ptr<Index> loadedIndex);
    IndexHolder(IndexInfo indexInfo, std::unique_ptr<uint8_t[]> storageInfoBuffer,
        uint32_t storageInfoBufferSize);

    std::string getName() const { return indexInfo.name; }
    const IndexInfo& getIndexInfo() const { return indexInfo; }
    // Rend vrai si les numéros changent ; l'index chargé suit (NodeTable::renumberColumns).
    bool setColumnIDs(std::vector<common::column_id_t> columnIDs) {
        if (columnIDs == indexInfo.columnIDs) {
            return false;
        }
        indexInfo.columnIDs = columnIDs;
        if (index) {
            index->setColumnIDs(std::move(columnIDs));
        }
        return true;
    }
    bool isLoaded() const { return loaded; }
    // Détaché : le rejeu du journal a écrit dans la table sans pouvoir tenir cet index à
    // jour (son extension n'était pas chargée). Il n'est plus un index de la table : il
    // n'est plus écrit au point de reprise, et son entrée au catalogue reste seule — c'est
    // l'état « à rebâtir », que l'extension nomme quand on veut s'en servir.
    bool isDetached() const { return detached; }
    void detach() { detached = true; }

    // La version du holder, comme celle d'une ligne (ticket
    // 2026-10-12-un-index-retire-ou-recree-reste-visible-a-tous-dans-le-stockage). Un CREATE ou un
    // DROP d'une transaction pose son identifiant (createdBy, droppedBy) ; sa validation pose
    // l'horodatage (createdTS, droppedTS), son annulation défait. Sans version (chargé du disque,
    // posé par le rejeu), le holder est visible de tous, comme avant.
    static constexpr common::transaction_t NO_VERSION = common::INVALID_TRANSACTION;
    uint64_t getHolderID() const { return holderID; }
    void setHolderID(uint64_t id) { holderID = id; }
    void markCreatedBy(common::transaction_t transactionID) { createdBy = transactionID; }
    void markDroppedBy(common::transaction_t transactionID) { droppedBy = transactionID; }
    void unmarkDropped() {
        droppedBy = NO_VERSION;
        droppedTS = NO_VERSION;
    }
    void commitCreate(common::transaction_t commitTS) { createdTS = commitTS; }
    void commitDrop(common::transaction_t commitTS) { droppedTS = commitTS; }
    // Retiré par une transaction, validé ou non : plus un index de la table pour qui commence
    // après, ni pour celle qui l'a retiré.
    bool isDropped() const { return droppedBy != NO_VERSION; }
    bool isDropCommitted() const { return droppedTS != NO_VERSION; }
    // Visible à la transaction (identifiant, début de son instantané).
    bool isVisibleTo(common::transaction_t transactionID, common::transaction_t startTS) const {
        const auto created = createdBy == NO_VERSION || createdBy == transactionID ||
                             (createdTS != NO_VERSION && createdTS <= startTS);
        const auto dropped = droppedBy != NO_VERSION &&
                             (droppedBy == transactionID ||
                                 (droppedTS != NO_VERSION && droppedTS <= startTS));
        return created && !dropped;
    }

    void serialize(common::Serializer& ser) const;
    RAG3DB_API void load(main::ClientContext* context, StorageManager* storageManager);
    bool needCommitInsert() const { return index && index->needCommitInsert(); }
    // NOLINTNEXTLINE(readability-make-member-function-const): Semantically non-const.
    void checkpoint(main::ClientContext* context, PageAllocator& pageAllocator) {
        if (loaded) {
            KU_ASSERT(index);
            index->checkpoint(context, pageAllocator);
        }
    }
    // NOLINTNEXTLINE(readability-make-member-function-const): Semantically non-const.
    void rollbackCheckpoint() {
        if (loaded) {
            KU_ASSERT(index);
            index->rollbackCheckpoint();
        }
    }
    // NOLINTNEXTLINE(readability-make-member-function-const): Semantically non-const.
    void finalize(main::ClientContext* context) {
        if (loaded) {
            KU_ASSERT(index);
            index->finalize(context);
        }
    }

    Index* getIndex() const {
        KU_ASSERT(index);
        return index.get();
    }
    std::shared_ptr<Index> getSharedIndex() const { return index; }

private:
    IndexInfo indexInfo;
    std::unique_ptr<uint8_t[]> storageInfoBuffer;
    uint64_t storageInfoBufferSize;
    bool loaded;
    bool detached = false;
    uint64_t holderID = 0;
    common::transaction_t createdBy = NO_VERSION;
    common::transaction_t createdTS = NO_VERSION;
    common::transaction_t droppedBy = NO_VERSION;
    common::transaction_t droppedTS = NO_VERSION;

    // Loaded index structure. Partagée : une recherche la capture (getSharedIndex) et la garde
    // vivante jusqu'à sa fin, même si un DROP concurrent retire le holder de la table.
    std::shared_ptr<Index> index;
};

} // namespace storage
} // namespace rag3db
