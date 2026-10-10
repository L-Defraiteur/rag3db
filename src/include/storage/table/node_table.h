#pragma once

#include "common/types/types.h"
#include "storage/index/hash_index.h"
#include "storage/table/node_group_collection.h"
#include "storage/table/table.h"

namespace rag3db {
namespace evaluator {
class ExpressionEvaluator;
} // namespace evaluator

namespace catalog {
class NodeTableCatalogEntry;
} // namespace catalog

namespace transaction {
class Transaction;
} // namespace transaction

namespace storage {

class LocalNodeTable;

struct RAG3DB_API NodeTableScanState : TableScanState {
    NodeTableScanState(common::ValueVector* nodeIDVector,
        std::vector<common::ValueVector*> outputVectors,
        std::shared_ptr<common::DataChunkState> outChunkState)
        : TableScanState{nodeIDVector, std::move(outputVectors), std::move(outChunkState)} {
        nodeGroupScanState = std::make_unique<NodeGroupScanState>(this->columnIDs.size());
    }

    void setToTable(const transaction::Transaction* transaction, Table* table_,
        std::vector<common::column_id_t> columnIDs_,
        std::vector<ColumnPredicateSet> columnPredicateSets_ = {},
        common::RelDataDirection direction = common::RelDataDirection::INVALID) override;

    bool scanNext(transaction::Transaction* transaction) override;

    NodeGroupScanResult scanNext(transaction::Transaction* transaction,
        common::offset_t startOffset, common::offset_t numNodes);

    // Les numéros de colonnes du balayage, pour le stockage validé. columnIDs les reprend, ou
    // leurs positions dans les groupes locaux quand la source est UNCOMMITTED
    // (NodeTable::initScanState, LocalNodeTable::getLocalColumnIDs).
    std::vector<common::column_id_t> committedColumnIDs;
};

// There is a vtable bug related to the Apple clang v15.0.0+. Adding the `FINAL` specifier to
// derived class causes casting failures in Apple platform.
struct RAG3DB_API NodeTableInsertState : TableInsertState {
    common::ValueVector& nodeIDVector;
    const common::ValueVector& pkVector;
    std::vector<std::unique_ptr<Index::InsertState>> indexInsertStates;
    // Pour chaque index, les positions de ses colonnes parmi les propriétés : propertyVectors
    // est rangé par position, les index par numéro de colonne (initInsertState).
    std::vector<std::vector<common::idx_t>> indexPropertyPositions;

    NodeTableInsertState(common::ValueVector& nodeIDVector, const common::ValueVector& pkVector,
        std::vector<common::ValueVector*> propertyVectors)
        : TableInsertState{std::move(propertyVectors)}, nodeIDVector{nodeIDVector},
          pkVector{pkVector} {}

    NodeTableInsertState(const NodeTableInsertState&) = delete;
};

struct RAG3DB_API NodeTableUpdateState : TableUpdateState {
    common::ValueVector& nodeIDVector;
    std::vector<std::unique_ptr<Index::UpdateState>> indexUpdateState;

    NodeTableUpdateState(common::column_id_t columnID, common::ValueVector& nodeIDVector,
        common::ValueVector& propertyVector)
        : TableUpdateState{columnID, propertyVector}, nodeIDVector{nodeIDVector} {}

    NodeTableUpdateState(const NodeTableUpdateState&) = delete;

    bool needToUpdateIndex(common::idx_t idx) const {
        return idx < indexUpdateState.size() && indexUpdateState[idx] != nullptr;
    }
};

struct RAG3DB_API NodeTableDeleteState : TableDeleteState {
    common::ValueVector& nodeIDVector;
    common::ValueVector& pkVector;
    // Pre-created index delete states, reused across multiple delete_() calls.
    std::vector<std::unique_ptr<Index::DeleteState>> indexDeleteStates;
    bool indexDeleteStatesInitialized = false;

    explicit NodeTableDeleteState(common::ValueVector& nodeIDVector, common::ValueVector& pkVector)
        : nodeIDVector{nodeIDVector}, pkVector{pkVector} {}

    // Comme l'état de mise à jour : clang-cl refuse d'instancier la copie implicite d'un vecteur
    // d'unique_ptr, même jamais appelée.
    NodeTableDeleteState(const NodeTableDeleteState&) = delete;
};

class NodeTable;
struct IndexScanHelper {
    explicit IndexScanHelper(NodeTable* table, Index* index) : table{table}, index(index) {}
    virtual ~IndexScanHelper() = default;

    virtual std::unique_ptr<NodeTableScanState> initScanState(
        const transaction::Transaction* transaction, common::DataChunk& dataChunk);
    virtual bool processScanOutput(main::ClientContext* context, NodeGroupScanResult scanResult,
        const std::vector<common::ValueVector*>& scannedVectors) = 0;

    NodeTable* table;
    Index* index;
};

class NodeTableVersionRecordHandler final : public VersionRecordHandler {
public:
    explicit NodeTableVersionRecordHandler(NodeTable* table);

    void applyFuncToChunkedGroups(version_record_handler_op_t func,
        common::node_group_idx_t nodeGroupIdx, common::row_idx_t startRow,
        common::row_idx_t numRows, common::transaction_t commitTS) const override;
    void rollbackInsert(main::ClientContext* context, common::node_group_idx_t nodeGroupIdx,
        common::row_idx_t startRow, common::row_idx_t numRows) const override;

private:
    NodeTable* table;
};

class StorageManager;

class RAG3DB_API NodeTable final : public Table {
public:
    NodeTable(const StorageManager* storageManager,
        const catalog::NodeTableCatalogEntry* nodeTableEntry, MemoryManager* mm);

    common::row_idx_t getNumTotalRows(const transaction::Transaction* transaction) override;

    // Écrit au journal de la transaction, sous la forme d'une insertion, les lignes de la table
    // dont le décalage est dans [startOffset, endOffset), dans l'ordre de leurs décalages. Pour
    // un chargement en masse, dont les lignes sont écrites hors du journal. Lève si une ligne de
    // la plage n'est pas visible : le rejeu ne retrouverait pas les mêmes décalages.
    void logInsertedRowsToWAL(main::ClientContext* context, common::offset_t startOffset,
        common::offset_t endOffset);

    void initScanState(transaction::Transaction* transaction, TableScanState& scanState,
        bool resetCachedBoundNodeIDs = true) const override;
    void initScanState(transaction::Transaction* transaction, TableScanState& scanState,
        common::table_id_t tableID, common::offset_t startOffset) const;

    bool scanInternal(transaction::Transaction* transaction, TableScanState& scanState) override;
    template<bool lock = true>
    bool lookup(const transaction::Transaction* transaction, const TableScanState& scanState) const;
    // TODO(Guodong): This should be merged together with `lookup`.
    template<bool lock = true>
    bool lookupMultiple(transaction::Transaction* transaction, TableScanState& scanState) const;

    // Return the max node offset during insertions.
    common::offset_t validateUniquenessConstraint(const transaction::Transaction* transaction,
        const common::ValueVector& pkVector) const;
    // Pour chaque index, les positions de ses colonnes parmi les propriétés de tableEntry
    // (NodeTableInsertState::indexPropertyPositions). À poser une fois par instruction.
    std::vector<std::vector<common::idx_t>> getIndexPropertyPositions(
        const catalog::TableCatalogEntry& tableEntry) const;

    void initInsertState(main::ClientContext* context, TableInsertState& insertState) override;
    void insert(transaction::Transaction* transaction, TableInsertState& insertState) override;
    void initUpdateState(main::ClientContext* context, TableUpdateState& updateState) const;
    void update(transaction::Transaction* transaction, TableUpdateState& updateState) override;
    void initDeleteStates(const transaction::Transaction* transaction, TableDeleteState& deleteState);
    bool delete_(transaction::Transaction* transaction, TableDeleteState& deleteState) override;
    void finalizeDelete(transaction::Transaction* transaction, TableDeleteState& deleteState);

    void addColumn(transaction::Transaction* transaction, TableAddColumnState& addColumnState,
        PageAllocator& pageAllocator) override;
    bool isVisible(const transaction::Transaction* transaction, common::offset_t offset) const;
    bool isVisible(common::transaction_t startTS, common::transaction_t transactionID,
        common::offset_t offset) const;
    // La ligne vue depuis le dernier état validé : toute ligne validée, quel que soit
    // l'instantané de la transaction, plus les siennes. C'est ce que regarde le contrôle
    // d'unicité d'une clé primaire sous les verrous (marche A3′).
    bool isVisibleToLatestCommit(const transaction::Transaction* transaction,
        common::offset_t offset) const;
    bool isVisibleNoLock(const transaction::Transaction* transaction,
        common::offset_t offset) const;
    // La clé d'une ligne sous la forme que le gestionnaire de verrous reçoit (marches A3′, A4′).
    static std::string lockKeyOf(const common::ValueVector& pkVector, common::sel_t pos);

    bool lookupPK(const transaction::Transaction* transaction, common::ValueVector* keyVector,
        uint64_t vectorPos, common::offset_t& result) const;

    void addIndex(std::unique_ptr<Index> index);
    void dropIndex(const std::string& name);
    // Le nom du refus d'écrire dans une table dont un index n'est pas chargé.
    static constexpr const char* INDEX_NOT_LOADED_FOR_WRITE =
        "is not loaded: load its extension before writing to table";

    common::column_id_t getPKColumnID() const { return pkColumnID; }
    PrimaryKeyIndex* getPKIndex() const {
        const auto index = getIndex(PrimaryKeyIndex::DEFAULT_NAME);
        KU_ASSERT(index.has_value());
        return &index.value()->cast<PrimaryKeyIndex>();
    }
    std::optional<std::reference_wrapper<IndexHolder>> getIndexHolder(const std::string& name);
    std::optional<Index*> getIndex(const std::string& name) const;
    std::vector<IndexHolder>& getIndexes() { return indexes; }

    common::column_id_t getNumColumns() const { return columns.size(); }
    Column& getColumn(common::column_id_t columnID) {
        KU_ASSERT(columnID < columns.size());
        return *columns[columnID];
    }
    const Column& getColumn(common::column_id_t columnID) const {
        KU_ASSERT(columnID < columns.size());
        return *columns[columnID];
    }

    std::pair<common::offset_t, common::offset_t> appendToLastNodeGroup(
        transaction::Transaction* transaction, const std::vector<common::column_id_t>& columnIDs,
        InMemChunkedNodeGroup& chunkedGroup, PageAllocator& pageAllocator);

    void commit(main::ClientContext* context, catalog::TableCatalogEntry* tableEntry,
        LocalTable* localTable) override;
    bool checkpoint(main::ClientContext* context, catalog::TableCatalogEntry* tableEntry,
        PageAllocator& pageAllocator) override;
    void rollbackCheckpoint() override;
    void reclaimStorage(PageAllocator& pageAllocator) const override;

    void rollbackPKIndexInsert(main::ClientContext* context, common::row_idx_t startRow,
        common::row_idx_t numRows_, common::node_group_idx_t nodeGroupIdx_);
    void rollbackGroupCollectionInsert(common::row_idx_t numRows_);
    // Prévient chaque index chargé que les lignes à partir de ce décalage sont retirées.
    void rollbackIndexInsert(common::offset_t firstRolledBackOffset);

    common::node_group_idx_t getNumCommittedNodeGroups() const {
        return nodeGroups->getNumNodeGroups();
    }

    common::node_group_idx_t getNumNodeGroups() const { return nodeGroups->getNumNodeGroups(); }
    common::offset_t getNumTuplesInNodeGroup(common::node_group_idx_t nodeGroupIdx) const {
        return nodeGroups->getNodeGroup(nodeGroupIdx)->getNumRows();
    }
    NodeGroup* getNodeGroup(common::node_group_idx_t nodeGroupIdx) const {
        return nodeGroups->getNodeGroup(nodeGroupIdx);
    }
    NodeGroup* getNodeGroupNoLock(common::node_group_idx_t nodeGroupIdx) const {
        return nodeGroups->getNodeGroupNoLock(nodeGroupIdx);
    }

    TableStats getStats(const transaction::Transaction* transaction) const;
    // NOLINTNEXTLINE(readability-make-member-function-const): Semantically non-const.
    void mergeStats(const std::vector<common::column_id_t>& columnIDs, const TableStats& stats) {
        nodeGroups->mergeStats(columnIDs, stats);
    }
    // CALL analyze : les statistiques rebâties depuis les lignes vivantes remplacent celles de
    // la table, et le point de reprise suivant les écrit.
    void replaceStats(TableStats stats) {
        nodeGroups->replaceStats(std::move(stats));
        setHasChanges();
    }

    void serialize(common::Serializer& serializer) const override;
    void deserialize(main::ClientContext* context, StorageManager* storageManager,
        common::Deserializer& deSer) override;

    // Le numéro de colonne de la clé et ceux des index ne sont pas une vérité gardée : ils se
    // recalculent ici depuis le catalogue (par le nom de la clé, par les propriétés de l'entrée
    // de chaque index), à la renumérotation du point de reprise (vacuumColumnIDs) et à la
    // lecture de la table. Une base écrite avant ce correctif, après un DROP, gardait sur
    // disque des numéros d'index périmés : elle se répare à l'ouverture.
    // Le catalogue est celui de la base de cette table : à la lecture d'une base attachée, ce
    // n'est pas celui du contexte (StorageManager::deserialize le passe).
    void renumberColumns(const catalog::Catalog& catalog,
        const catalog::NodeTableCatalogEntry& tableEntry);

private:
    // Un index dans lequel une écriture peut passer. Un index non chargé ne l'est pas :
    // pendant le rejeu du journal il est détaché (la table reçoit la ligne, l'index sera à
    // rebâtir) ; hors rejeu, l'écriture est refusée par INDEX_NOT_LOADED_FOR_WRITE.
    bool isWritableIndex(IndexHolder& indexHolder, const transaction::Transaction* transaction);

private:
    void validatePkNotExists(const transaction::Transaction* transaction,
        common::ValueVector* pkVector) const;

    visible_func getVisibleFunc(const transaction::Transaction* transaction) const;
    // La visibilité du contrôle d'unicité de la clé primaire (marche A3′) : sous les verrous,
    // le dernier état validé ; sinon l'instantané. La même à l'insertion et à la validation.
    visible_func getUniquenessVisibleFunc(const transaction::Transaction* transaction) const;
    common::DataChunk constructDataChunkForColumns(
        const std::vector<common::column_id_t>& columnIDs) const;
    void scanIndexColumns(main::ClientContext* context, IndexScanHelper& scanHelper,
        const NodeGroupCollection& nodeGroups_, const LocalNodeTable* localTable = nullptr) const;

private:
    std::vector<std::unique_ptr<Column>> columns;
    std::unique_ptr<NodeGroupCollection> nodeGroups;
    common::column_id_t pkColumnID;
    std::vector<IndexHolder> indexes;
    NodeTableVersionRecordHandler versionRecordHandler;
};

} // namespace storage
} // namespace rag3db
