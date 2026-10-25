#pragma once

#include <cstdint>
#include <optional>
#include <shared_mutex>

#include "common/uniq_lock.h"
#include "storage/enums/residency_state.h"
#include "storage/table/chunked_node_group.h"
#include "storage/table/group_collection.h"
#include "storage/table/version_record_handler.h"

namespace rag3db {
namespace transaction {
class Transaction;
} // namespace transaction

namespace storage {
class MemoryManager;

class ColumnStats;
struct TableAddColumnState;
class NodeGroup;

// La liste des blocs d'un groupe, en propriété partagée : celle qu'un balayage tient, celle que
// le point de reprise prend sous le verrou puis lit sans lui.
using ChunkedGroups = std::vector<std::shared_ptr<ChunkedNodeGroup>>;

struct NodeGroupScanState {
    // Index of committed but not yet checkpointed chunked group to scan.
    common::idx_t chunkedGroupIdx = 0;
    common::row_idx_t nextRowToScan = 0;
    // State of each chunk in the checkpointed chunked group.
    std::vector<ChunkState> chunkStates;
    // Les blocs du groupe au moment où le balayage a commencé, tenus en propriété partagée :
    // un point de reprise qui échange la liste des blocs (le bloc neuf de V2-1b) n'enlève rien
    // sous les pieds d'un lecteur, et chunkStates reste accordé au bloc sur disque qu'il
    // décrit. Un bloc ajouté depuis (une insertion locale de la même transaction, ou une
    // validation d'une autre, dont les lignes lui sont de toute façon invisibles) s'y ajoute à la
    // demande (NodeGroup::extendScanGroups) ; une liste échangée par un point de reprise, jamais :
    // le lecteur finit sur la sienne.
    ChunkedGroups groups;

    explicit NodeGroupScanState() {}
    explicit NodeGroupScanState(common::idx_t numChunks) { chunkStates.resize(numChunks); }

    virtual ~NodeGroupScanState() = default;
    DELETE_COPY_DEFAULT_MOVE(NodeGroupScanState);

    template<class TARGET>
    TARGET& cast() {
        return common::ku_dynamic_cast<TARGET&>(*this);
    }
    template<class TARGET>
    const TARGET& constCast() {
        return common::ku_dynamic_cast<const TARGET&>(*this);
    }
};

struct NodeGroupCheckpointState {
    std::vector<common::column_id_t> columnIDs;
    std::vector<Column*> columns;
    PageAllocator& pageAllocator;
    MemoryManager* mm;
    // Le drapeau du point de reprise (StorageManager::markCheckpointChangedMemory), posé juste
    // avant le premier échange de groupes.
    bool* changedMemory = nullptr;
    // V2-2 : l'horodatage T de ce point de reprise et le plus vieux instantané ouvert F. Si
    // F < T, l'ancienne génération des blocs est gardée pour les transactions d'instantané < T.
    common::transaction_t checkpointTS = 0;
    common::transaction_t oldestReaderTS = 0;
    bool keepsPreviousGeneration() const { return oldestReaderTS < checkpointTS; }

    void markChangedMemory() const {
        if (changedMemory) {
            *changedMemory = true;
        }
    }

    NodeGroupCheckpointState(std::vector<common::column_id_t> columnIDs,
        std::vector<Column*> columns, PageAllocator& pageAllocator, MemoryManager* mm)
        : columnIDs{std::move(columnIDs)}, columns{std::move(columns)},
          pageAllocator{pageAllocator}, mm{mm} {}
    virtual ~NodeGroupCheckpointState() = default;

    template<typename T>
    const T& cast() const {
        return common::ku_dynamic_cast<const T&>(*this);
    }
    template<typename T>
    T& cast() {
        return common::ku_dynamic_cast<T&>(*this);
    }
};

struct NodeGroupScanResult {

    common::row_idx_t startRow = common::INVALID_ROW_IDX;
    common::row_idx_t numRows = 0;

    constexpr NodeGroupScanResult() noexcept = default;
    constexpr NodeGroupScanResult(common::row_idx_t startRow, common::row_idx_t numRows) noexcept
        : startRow{startRow}, numRows{numRows} {}

    bool operator==(const NodeGroupScanResult& other) const {
        return startRow == other.startRow && numRows == other.numRows;
    }
};

static auto NODE_GROUP_SCAN_EMPTY_RESULT = NodeGroupScanResult{};

struct TableScanState;
class NodeGroup {
public:
    NodeGroup(MemoryManager& mm, const common::node_group_idx_t nodeGroupIdx,
        const bool enableCompression, std::vector<common::LogicalType> dataTypes,
        common::row_idx_t capacity = common::StorageConfig::NODE_GROUP_SIZE,
        NodeGroupDataFormat format = NodeGroupDataFormat::REGULAR)
        : mm{mm}, nodeGroupIdx{nodeGroupIdx}, format{format}, enableCompression{enableCompression},
          numRows{0}, nextRowToAppend{0}, capacity{capacity}, dataTypes{std::move(dataTypes)} {}
    NodeGroup(MemoryManager& mm, const common::node_group_idx_t nodeGroupIdx,
        const bool enableCompression, std::unique_ptr<ChunkedNodeGroup> chunkedNodeGroup,
        common::row_idx_t capacity = common::StorageConfig::NODE_GROUP_SIZE,
        NodeGroupDataFormat format = NodeGroupDataFormat::REGULAR)
        : mm{mm}, nodeGroupIdx{nodeGroupIdx}, format{format}, enableCompression{enableCompression},
          numRows{chunkedNodeGroup->getStartRowIdx() + chunkedNodeGroup->getNumRows()},
          nextRowToAppend{numRows}, capacity{capacity} {
        for (auto i = 0u; i < chunkedNodeGroup->getNumColumns(); i++) {
            dataTypes.push_back(chunkedNodeGroup->hasColumnChunk(i) ?
                                    chunkedNodeGroup->getColumnChunk(i).getDataType().copy() :
                                    common::LogicalType::ANY());
        }
        const auto lock = chunkedGroups.lock();
        chunkedGroups.appendGroup(lock, std::move(chunkedNodeGroup));
    }
    NodeGroup(MemoryManager& mm, const common::node_group_idx_t nodeGroupIdx,
        const bool enableCompression, common::row_idx_t capacity, NodeGroupDataFormat format)
        : mm{mm}, nodeGroupIdx{nodeGroupIdx}, format{format}, enableCompression{enableCompression},
          numRows{0}, nextRowToAppend{0}, capacity{capacity} {}
    virtual ~NodeGroup() = default;

    virtual bool isEmpty() const { return numRows.load() == 0; }
    virtual common::row_idx_t getNumRows() const { return numRows.load(); }
    // Les lignes validées et non supprimées, d'après les informations de version de ses blocs
    // (persistées avec eux) : sans relire les colonnes. Un groupe vidé au point de reprise
    // (ChunkedNodeGroup::flushEmpty) n'a plus de bloc porteur de lignes et compte 0.
    common::row_idx_t getNumLiveRows() const;
    void moveNextRowToAppend(common::row_idx_t numRowsToAppend) {
        nextRowToAppend += numRowsToAppend;
    }
    common::row_idx_t getNumRowsLeftToAppend() const { return capacity - nextRowToAppend; }
    bool isFull() const { return numRows.load() == capacity; }
    const std::vector<common::LogicalType>& getDataTypes() const { return dataTypes; }
    NodeGroupDataFormat getFormat() const { return format; }
    common::row_idx_t append(const transaction::Transaction* transaction,
        const std::vector<common::column_id_t>& columnIDs, ChunkedNodeGroup& chunkedGroup,
        common::row_idx_t startRowIdx, common::row_idx_t numRowsToAppend);
    common::row_idx_t append(const transaction::Transaction* transaction,
        const std::vector<common::column_id_t>& columnIDs, InMemChunkedNodeGroup& chunkedGroup,
        common::row_idx_t startRowIdx, common::row_idx_t numRowsToAppend);
    common::row_idx_t append(const transaction::Transaction* transaction,
        const std::vector<common::column_id_t>& columnIDs,
        std::span<const ColumnChunkData*> chunkedGroup, common::row_idx_t startRowIdx,
        common::row_idx_t numRowsToAppend);
    common::row_idx_t append(const transaction::Transaction* transaction,
        const std::vector<common::column_id_t>& columnIDs,
        std::span<const ColumnChunk*> chunkedGroup, common::row_idx_t startRowIdx,
        common::row_idx_t numRowsToAppend);
    void append(const transaction::Transaction* transaction,
        const std::vector<common::ValueVector*>& vectors, common::row_idx_t startRowIdx,
        common::row_idx_t numRowsToAppend);

    void merge(transaction::Transaction* transaction,
        std::unique_ptr<ChunkedNodeGroup> chunkedGroup);

    virtual void initializeScanState(const transaction::Transaction* transaction,
        TableScanState& state) const;
    void initializeScanState(const transaction::Transaction* transaction,
        const common::UniqLock& lock, TableScanState& state) const;
    virtual NodeGroupScanResult scan(const transaction::Transaction* transaction,
        TableScanState& state) const;

    virtual NodeGroupScanResult scan(transaction::Transaction* transaction, TableScanState& state,
        common::offset_t startOffsetInGroup, common::offset_t numRowsToScan) const;

    bool lookup(const transaction::Transaction* transaction, const TableScanState& state,
        common::sel_t posInSel = 0) const;
    // La lecture d'une ligne sans le verrou du groupe, pour qui lit beaucoup et en parallèle
    // (la recherche HNSW lit un vecteur par nœud visité) : elle ne tient que la garde
    // partagée contre l'ajout en mémoire.
    bool lookupSharedLock(const transaction::Transaction* transaction,
        const TableScanState& state, common::sel_t posInSel = 0) const;
    // TODO(Guodong): These should be merged together with `lookup`.
    bool lookupMultiple(const common::UniqLock& lock, const transaction::Transaction* transaction,
        const TableScanState& state) const;
    bool lookupMultiple(const transaction::Transaction* transaction,
        const TableScanState& state) const;

    void update(const transaction::Transaction* transaction, common::row_idx_t rowIdxInGroup,
        common::column_id_t columnID, const common::ValueVector& propertyVector);
    bool delete_(const transaction::Transaction* transaction, common::row_idx_t rowIdxInGroup);

    bool hasDeletions(const transaction::Transaction* transaction) const;
    virtual void addColumn(TableAddColumnState& addColumnState, PageAllocator* pageAllocator,
        ColumnStats* newColumnStats);
    virtual void removeLastColumn(PageAllocator* pageAllocator);

    void applyFuncToChunkedGroups(version_record_handler_op_t func, common::row_idx_t startRow,
        common::row_idx_t numRows, common::transaction_t commitTS) const;
    virtual void rollbackInsert(common::row_idx_t startRow);
    // V2-2 : retire les générations précédentes que plus aucune transaction ne précède.
    virtual void retireGenerationsBefore(common::transaction_t oldestReaderTS);
    void reclaimStorage(PageAllocator& pageAllocator) const;
    void retireStorage(PageAllocator& pageAllocator, const ChunkedGroups& groups) const;
    virtual void reclaimStorage(PageAllocator& pageAllocator, const common::UniqLock& lock) const;

    virtual void checkpoint(MemoryManager& memoryManager, NodeGroupCheckpointState& state);

    uint64_t getEstimatedMemoryUsage() const;

    virtual void serialize(common::Serializer& serializer);
    static std::unique_ptr<NodeGroup> deserialize(MemoryManager& mm, common::Deserializer& deSer,
        const std::vector<common::LogicalType>& columnTypes);

    common::node_group_idx_t getNumChunkedGroups() const {
        const auto lock = chunkedGroups.lock();
        return chunkedGroups.getNumGroups(lock);
    }
    ChunkedNodeGroup* getChunkedNodeGroup(common::node_group_idx_t groupIdx) const {
        const auto lock = chunkedGroups.lock();
        return chunkedGroups.getGroup(lock, groupIdx);
    }

    template<class TARGET>
    TARGET& cast() {
        return common::ku_dynamic_cast<TARGET&>(*this);
    }
    template<class TARGET>
    const TARGET& cast() const {
        return common::ku_dynamic_cast<const TARGET&>(*this);
    }

    bool isVisible(const transaction::Transaction* transaction,
        common::row_idx_t rowIdxInGroup) const;
    bool isVisible(common::transaction_t startTS, common::transaction_t transactionID,
        common::row_idx_t rowIdxInGroup) const;
    bool wasWrittenByCommitAfter(common::transaction_t startTS,
        common::transaction_t transactionID, common::row_idx_t rowIdxInGroup) const;
    // Une mise à jour validée après startTS de cette colonne, à cette ligne (la chaîne des
    // versions de mise à jour, en mémoire jusqu'au point de reprise).
    bool wasColumnUpdatedByCommitAfter(common::column_id_t columnID,
        common::transaction_t startTS, common::row_idx_t rowIdxInGroup) const;
    bool isVisibleNoLock(const transaction::Transaction* transaction,
        common::row_idx_t rowIdxInGroup) const;
    bool isDeleted(const transaction::Transaction* transaction,
        common::offset_t offsetInGroup) const;
    bool isInserted(const transaction::Transaction* transaction,
        common::offset_t offsetInGroup) const;

    common::node_group_idx_t getNodeGroupIdx() const { return nodeGroupIdx; }

protected:
    static constexpr auto INVALID_CHUNKED_GROUP_IDX = UINT32_MAX;
    static constexpr auto INVALID_START_ROW_IDX = UINT64_MAX;

protected:
    void checkpointDataTypesNoLock(const NodeGroupCheckpointState& state);
    // Les blocs tenus par un balayage (NodeGroupScanState::groups) : l'instantané de la liste
    // s'il n'en a pas encore, sinon les blocs ajoutés depuis, si la liste vivante commence
    // encore par le même bloc. Après un échange au point de reprise, le lecteur garde la sienne.
    void extendScanGroups(const common::UniqLock& lock, NodeGroupScanState& scanState) const;
    // V2-2 : une génération précédente de la liste des blocs, remplacée par le point de reprise
    // d'horodatage checkpointTS et gardée pour les transactions d'instantané antérieur : elles y
    // lisent ce qu'elles voyaient, versions comprises. Retirée au point de reprise suivant quand
    // plus aucune n'est ouverte.
    struct PreviousGeneration {
        common::transaction_t checkpointTS;
        ChunkedGroups groups;
    };
    // La liste des blocs que lit une transaction d'instantané startTS : la plus ancienne
    // génération gardée dont le point de reprise lui est postérieur, sinon la liste vivante.
    const ChunkedGroups& groupsFor(const common::UniqLock& lock,
        common::transaction_t startTS) const;
    // Garde l'ancienne liste (si l'état le demande) et retire les générations que plus
    // personne ne lit. Sous le verrou.
    void retireAndKeepGeneration(const common::UniqLock& lock, ChunkedGroups oldGroups,
        const NodeGroupCheckpointState& state);
    // Le bloc de ce rang parmi ceux que le balayage tient, en l'étendant s'il le faut ; nul si
    // la liste n'a pas ce rang.
    const ChunkedNodeGroup* scanGroupAt(NodeGroupScanState& scanState,
        common::idx_t chunkedGroupIdx) const;
    // Le rang du bloc qui porte une ligne, et la ligne dans ce bloc : le premier bloc porte ses
    // propres lignes, les suivants en font CHUNKED_NODE_GROUP_CAPACITY chacun.
    static std::pair<common::idx_t, common::row_idx_t> findChunkedGroupIdx(
        const std::vector<std::shared_ptr<ChunkedNodeGroup>>& groups, common::row_idx_t rowIdx);

private:
    std::pair<common::idx_t, common::row_idx_t> findChunkedGroupIdxFromRowIdxNoLock(
        common::row_idx_t rowIdx) const;
    ChunkedNodeGroup* findChunkedGroupFromRowIdx(const common::UniqLock& lock,
        common::row_idx_t rowIdx) const;
    // La ligne cherchée, lue dans les blocs que le balayage tient ; rien si aucun ne la porte.
    std::optional<bool> lookupInScanGroups(const transaction::Transaction* transaction,
        const TableScanState& state, common::sel_t posInSel) const;
    // Le même bloc en propriété partagée, pour l'utiliser hors du verrou de la collection.
    std::shared_ptr<ChunkedNodeGroup> findChunkedGroupSharedFromRowIdx(const common::UniqLock& lock,
        common::row_idx_t rowIdx) const;
    ChunkedNodeGroup* findChunkedGroupFromRowIdxNoLock(common::row_idx_t rowIdx) const;

    // Le point de reprise bâtit le bloc neuf sur la liste des blocs prise sous le verrou puis
    // rendue (V2-1c) : ces fonctions lisent `groups`, jamais chunkedGroups.
    std::unique_ptr<ChunkedNodeGroup> checkpointInMemOnly(MemoryManager& memoryManager,
        const ChunkedGroups& groups, const NodeGroupCheckpointState& state) const;
    std::unique_ptr<ChunkedNodeGroup> checkpointInMemAndOnDisk(MemoryManager& memoryManager,
        const ChunkedGroups& groups, NodeGroupCheckpointState& state) const;
    std::unique_ptr<VersionInfo> checkpointVersionInfo(const ChunkedGroups& groups,
        const transaction::Transaction* transaction) const;
    // L'état de balayage posé sur les blocs qu'il tient déjà (NodeGroupScanState::groups).
    void initializeScanStateFromGroups(TableScanState& state) const;

    template<ResidencyState SCAN_RESIDENCY_STATE>
    common::row_idx_t getNumResidentRows(const ChunkedGroups& groups) const;
    template<ResidencyState SCAN_RESIDENCY_STATE>
    std::unique_ptr<InMemChunkedNodeGroup> scanAllInsertedAndVersions(MemoryManager& memoryManager,
        const ChunkedGroups& groups, const std::vector<common::column_id_t>& columnIDs,
        const std::vector<const Column*>& columns) const;

    virtual NodeGroupScanResult scanInternal(const common::UniqLock& lock,
        transaction::Transaction* transaction, TableScanState& state,
        common::offset_t startOffsetInGroup, common::offset_t numRowsToScan) const;

    common::row_idx_t getStartRowIdxInGroupNoLock() const;
    common::row_idx_t getStartRowIdxInGroup(const common::UniqLock& lock) const;

    void scanCommittedUpdatesForColumn(std::vector<ChunkCheckpointState>& chunkCheckpointStates,
        MemoryManager& memoryManager, const ChunkedGroups& groups, common::column_id_t columnID,
        const Column* column) const;

protected:
    MemoryManager& mm;
    common::node_group_idx_t nodeGroupIdx;
    NodeGroupDataFormat format;
    bool enableCompression;
    std::atomic<common::row_idx_t> numRows;
    // `nextRowToAppend` is a cursor to allow us to pre-reserve a set of rows to append before
    // acutally appending data. This is an optimization to reduce lock-contention when appending in
    // parallel.
    // TODO(Guodong): Remove this field.
    common::row_idx_t nextRowToAppend;
    common::row_idx_t capacity;
    std::vector<common::LogicalType> dataTypes;
    GroupCollection<ChunkedNodeGroup> chunkedGroups;
    // Les générations précédentes, de la plus ancienne à la plus récente (V2-2). Sous le
    // verrou de chunkedGroups.
    std::vector<PreviousGeneration> previousGenerations;
    // La garde des lectures qui ne prennent pas le verrou de chunkedGroups (lookupSharedLock,
    // isVisibleNoLock). L'ajout en mémoire la prend en exclusif : il allonge la liste des
    // blocs et réalloue le tampon d'une colonne à longueur variable, qu'un lecteur ne doit
    // pas avoir sous la main à ce moment-là. Toujours prise après le verrou de
    // chunkedGroups, jamais l'inverse.
    mutable std::shared_mutex inMemAppendMtx;
};

} // namespace storage
} // namespace rag3db
