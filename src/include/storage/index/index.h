#pragma once

#include <functional>
#include <memory>

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

// L'état de construction d'un index ((A), le CREATE concurrent : page 07 de coeur-cpp-tickets).
// READY vaut 0 : un holder écrit avant le format 43, ou sans construction, est prêt.
// - NOTED : l'entrée existe, le graphe n'est pas encore bâti ; les recherches sont refusées, et
//   les écrivains ne touchent pas à l'index (la validation du bâti les rattrape).
// - MAINTAINED : bâti, entretenu par les écrivains, pas encore rattrapé (lot 4).
// - INCOMPLETE : la construction n'a pas fini (requête interrompue, connexion tombée, processus
//   mort) ; refusé aux recherches, à retirer puis à refaire.
enum class IndexBuildState : uint8_t {
    READY = 0,
    NOTED = 1,
    MAINTAINED = 2,
    INCOMPLETE = 3,
};
RAG3DB_API std::string indexBuildStateToString(IndexBuildState state);

// Ce qu'un enregistrement d'annulation INDEX_CHANGE défait ou date : la création d'un holder, son
// retrait, ou une transition de son état de construction.
enum class IndexChangeKind : uint8_t {
    DROP = 0,
    CREATE = 1,
    BUILD_STATE = 2,
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

    // L'état de construction, versionné comme la création : une transition d'une transaction
    // (markBuildState) n'est vue que d'elle jusqu'à sa validation (commitBuildState), et son
    // annulation la défait. Une seule transition en cours à la fois (le bâtisseur).
    // Le jeton du bâtisseur : un holder NOTED ou MAINTAINED n'est en construction que tant que la
    // requête qui le construit vit (ClientContext::newQueryScopedToken) ; sans elle — requête
    // interrompue ou en échec, connexion tombée, holder relu du disque ou du journal — il est
    // INCOMPLETE. Le jeton n'est qu'en mémoire : à la réouverture, tout état autre que READY vaut
    // donc INCOMPLETE.
    void initBuildState(IndexBuildState state, std::weak_ptr<void> token) {
        buildState = state;
        builderToken = std::move(token);
    }
    void setBuilderToken(std::weak_ptr<void> token) { builderToken = std::move(token); }
    void markBuildState(common::transaction_t transactionID, IndexBuildState state) {
        previousBuildState = buildState;
        buildState = state;
        buildStateBy = transactionID;
        buildStateTS = NO_VERSION;
    }
    void commitBuildState(common::transaction_t commitTS) { buildStateTS = commitTS; }
    void rollbackBuildState() {
        buildState = previousBuildState;
        buildStateBy = NO_VERSION;
        buildStateTS = NO_VERSION;
    }
    // Posé sans version (le rejeu du journal, la lecture du disque).
    void setBuildState(IndexBuildState state) {
        buildState = state;
        buildStateBy = NO_VERSION;
        buildStateTS = NO_VERSION;
    }
    // L'état vu d'une transaction (identifiant, début de son instantané).
    IndexBuildState buildStateFor(common::transaction_t transactionID,
        common::transaction_t startTS) const {
        const auto seesTransition = buildStateBy == NO_VERSION || buildStateBy == transactionID ||
                                    (buildStateTS != NO_VERSION && buildStateTS <= startTS);
        return withBuilder(seesTransition ? buildState : previousBuildState);
    }
    // Le dernier état validé : ce qu'un écrivain lit au moment d'écrire, sous le verrou d'index
    // (page 07, §3 bis.1), et non à son instantané.
    IndexBuildState committedBuildState() const {
        const auto pending = buildStateBy != NO_VERSION && buildStateTS == NO_VERSION;
        return withBuilder(pending ? previousBuildState : buildState);
    }
    IndexBuildState rawBuildState() const { return buildState; }
    // Un holder ne reçoit les écritures qu'une fois bâti : READY, ou MAINTAINED (lot 4). NOTED et
    // INCOMPLETE ne sont pas écrits (la validation du bâti rattrape ce que NOTED a laissé passer).
    bool receivesWrites() const {
        const auto state = committedBuildState();
        return state == IndexBuildState::READY || state == IndexBuildState::MAINTAINED;
    }
    // T_B : l'index bâti prend la place de l'index vide posé par T_A.
    void replaceIndex(std::unique_ptr<Index> built) {
        index = std::move(built);
        loaded = true;
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
    IndexBuildState buildState = IndexBuildState::READY;
    IndexBuildState previousBuildState = IndexBuildState::READY;
    common::transaction_t buildStateBy = NO_VERSION;
    common::transaction_t buildStateTS = NO_VERSION;
    std::weak_ptr<void> builderToken;

    IndexBuildState withBuilder(IndexBuildState state) const {
        if ((state == IndexBuildState::NOTED || state == IndexBuildState::MAINTAINED) &&
            builderToken.expired()) {
            return IndexBuildState::INCOMPLETE;
        }
        return state;
    }

    // Loaded index structure. Partagée : une recherche la capture (getSharedIndex) et la garde
    // vivante jusqu'à sa fin, même si un DROP concurrent retire le holder de la table.
    std::shared_ptr<Index> index;
};

} // namespace storage
} // namespace rag3db
