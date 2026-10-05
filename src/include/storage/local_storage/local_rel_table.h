#pragma once

#include <map>
#include <unordered_map>

#include "common/enums/rel_direction.h"
#include "storage/local_storage/local_table.h"
#include "storage/table/csr_node_group.h"

namespace rag3db {
namespace storage {
class MemoryManager;

static constexpr common::column_id_t LOCAL_BOUND_NODE_ID_COLUMN_ID = 0;
static constexpr common::column_id_t LOCAL_NBR_NODE_ID_COLUMN_ID = 1;
static constexpr common::column_id_t LOCAL_REL_ID_COLUMN_ID = 2;

class RelTable;
struct TableScanState;
struct RelTableUpdateState;

struct DirectedCSRIndex {
    using index_t = std::map<common::offset_t, row_idx_vec_t>;

    explicit DirectedCSRIndex(common::RelDataDirection direction) : direction(direction) {}

    bool isEmpty() const { return index.empty(); }
    void clear() { index.clear(); }

    common::RelDataDirection direction;
    index_t index;
};

// Where the nodes a transaction created ended up. While the transaction runs, its new nodes of a
// table are numbered from the size the table had when it first wrote to it; another transaction
// that commits in between takes those offsets, and this one lands further. `numRows` offsets
// starting at `oldStartOffset` have become the offsets starting at `newStartOffset`.
struct LocalNodeOffsetMap {
    common::offset_t oldStartOffset;
    common::offset_t newStartOffset;
    common::row_idx_t numRows;
};

using local_node_offset_map_t = std::unordered_map<common::table_id_t, LocalNodeOffsetMap>;

class LocalWAL;

class LocalRelTable final : public LocalTable {
public:
    LocalRelTable(const catalog::TableCatalogEntry* tableEntry, const Table& table,
        MemoryManager& mm);
    DELETE_COPY_AND_MOVE(LocalRelTable);

    bool insert(transaction::Transaction* transaction, TableInsertState& state) override;
    bool update(transaction::Transaction* transaction, TableUpdateState& state) override;
    bool delete_(transaction::Transaction* transaction, TableDeleteState& state) override;
    bool addColumn(TableAddColumnState& addColumnState) override;

    bool checkIfNodeHasRels(common::ValueVector* srcNodeIDVector,
        common::RelDataDirection direction) const;

    common::TableType getTableType() const override { return common::TableType::REL; }

    static void initializeScan(TableScanState& state);
    bool scan(const transaction::Transaction* transaction, TableScanState& state) const;

    void clear(MemoryManager&) override {
        localNodeGroup.reset();
        for (auto& index : directedIndices) {
            index.clear();
        }
    }
    bool isEmpty() const {
        KU_ASSERT(directedIndices.size() >= 1);
        RUNTIME_CHECK(for (const auto& index
                           : directedIndices) {
            KU_ASSERT(index.index.empty() == directedIndices[0].index.empty());
        });
        return directedIndices[0].isEmpty();
    }

    common::column_id_t getNumColumns() const { return localNodeGroup->getDataTypes().size(); }
    common::row_idx_t getNumTotalRows() override { return localNodeGroup->getNumRows(); }

    // At commit, once the nodes of the transaction have their final offsets: rewrites the
    // endpoints of the local relationships that point to those nodes, and rebuilds the indexes.
    // Only the first `numRows` rows are looked at: those that existed before the nodes were
    // committed. Rows added during the commit (the edges an index creates for the new nodes)
    // already carry final offsets, which may fall in a range that is being remapped.
    void remapNodeOffsets(const local_node_offset_map_t& offsetMap, common::row_idx_t numRows);
    // Logs the relationships that are still there, with their final endpoints. Called at commit,
    // after remapNodeOffsets: logging at insertion time recorded offsets that a replay would
    // attach to another transaction's nodes.
    void logInsertionsToWAL(LocalWAL& wal, MemoryManager& mm) const;
    // The rows that were not deleted since, in insertion order.
    row_idx_vec_t getActiveRows() const;
    std::pair<ChunkedNodeGroup*, common::row_idx_t> getChunkedGroupAndRow(
        common::row_idx_t rowIdx) const;

    DirectedCSRIndex::index_t& getCSRIndex(common::RelDataDirection direction) {
        const auto directionIdx = common::RelDirectionUtils::relDirectionToKeyIdx(direction);
        KU_ASSERT(directionIdx < directedIndices.size());
        return directedIndices[directionIdx].index;
    }
    NodeGroup& getLocalNodeGroup() const { return *localNodeGroup; }

    // La colonne de ce groupe local : les deux nœuds en tête, puis les propriétés rangées par
    // position (celle du catalogue que voyait la transaction à sa création), pas par numéro de
    // colonne — ils diffèrent après ALTER TABLE … DROP.
    std::vector<common::column_id_t> rewriteLocalColumnIDs(common::RelDataDirection direction,
        const std::vector<common::column_id_t>& columnIDs) const;
    common::column_id_t rewriteLocalColumnID(common::RelDataDirection direction,
        common::column_id_t columnID) const;
    // Pour chaque propriété de ce groupe local, dans son ordre : son numéro de colonne.
    std::vector<common::column_id_t> getCommittedPropertyColumnIDs() const;

private:
    common::row_idx_t findMatchingRow(const transaction::Transaction* transaction,
        const std::vector<row_idx_vec_t*>& rowIndicesToCheck, common::offset_t relOffset) const;

private:
    // We don't duplicate local rel tuples. Tuples are stored same as node tuples.
    // Chunks stored in local rel table are organized as follows:
    // [srcNodeID, dstNodeID, relID, property1, property2, ...]
    // All local rel tuples are stored in a single node group, and they are indexed by src/dst
    // NodeID.
    std::vector<DirectedCSRIndex> directedIndices;
    std::unique_ptr<NodeGroup> localNodeGroup;
    // Indexé par numéro de colonne : la position de la propriété (getPropertyPosition).
    std::vector<common::idx_t> positionOfColumn;
};

} // namespace storage
} // namespace rag3db
