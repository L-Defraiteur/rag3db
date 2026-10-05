#pragma once

#include "common/copy_constructors.h"
#include "storage/local_storage/local_hash_index.h"
#include "storage/local_storage/local_table.h"
#include "storage/table/node_group_collection.h"

namespace rag3db {
namespace storage {

struct TableScanState;
class MemoryManager;

class LocalNodeTable final : public LocalTable {
public:
    LocalNodeTable(const catalog::TableCatalogEntry* tableEntry, Table& table, MemoryManager& mm);
    DELETE_COPY_AND_MOVE(LocalNodeTable);

    bool insert(transaction::Transaction* transaction, TableInsertState& insertState) override;
    bool update(transaction::Transaction* transaction, TableUpdateState& updateState) override;
    bool delete_(transaction::Transaction* transaction, TableDeleteState& deleteState) override;
    bool addColumn(TableAddColumnState& addColumnState) override;

    common::offset_t validateUniquenessConstraint(const transaction::Transaction* transaction,
        const common::ValueVector& pkVector) const;

    common::TableType getTableType() const override { return common::TableType::NODE; }

    void clear(MemoryManager& mm) override;

    common::row_idx_t getNumTotalRows() override { return nodeGroups.getNumTotalRows(); }
    common::node_group_idx_t getNumNodeGroups() const { return nodeGroups.getNumNodeGroups(); }

    NodeGroup* getNodeGroup(common::node_group_idx_t nodeGroupIdx) const {
        return nodeGroups.getNodeGroup(nodeGroupIdx);
    }
    NodeGroupCollection& getNodeGroups() { return nodeGroups; }

    bool lookupPK(const transaction::Transaction* transaction, const common::ValueVector* keyVector,
        common::sel_t pos, common::offset_t& result) const;

    TableStats getStats() const { return nodeGroups.getStats(); }
    common::offset_t getStartOffset() const { return startOffset; }
    // Called at commit once the rows are in the table. From then on no offset designates a row of
    // this local table: the transaction tells its own rows from the committed ones by comparing
    // an offset to the start offset, which was only ever the size of the table when the
    // transaction first wrote to it. The rows may have landed further, and what follows in the
    // commit (an index inserting the new rows) reads them by their final offsets.
    void rowsAreCommitted() { startOffset = common::INVALID_OFFSET; }
    // Vide — après que ses lignes ont été versées dans la table pour un COPY, ou créée avant
    // un COPY de la transaction : ses prochaines lignes commencent où la table finit
    // maintenant, pas où elle finissait à sa création.
    void restartAt(common::offset_t tableEnd) {
        KU_ASSERT(nodeGroups.getNumTotalRows() == 0);
        startOffset = tableEnd;
    }

    static std::vector<common::LogicalType> getNodeTableColumnTypes(
        const catalog::TableCatalogEntry& table);

    // Les groupes de cette table sont rangés par position de propriété (celle du catalogue que
    // voyait la transaction à sa création) ; le stockage validé et les index, par numéro de
    // colonne. La colonne columnID dans ces groupes ; les numéros spéciaux (INVALID_COLUMN_ID,
    // ROW_IDX_COLUMN_ID) passent tels quels.
    common::column_id_t getLocalColumnID(common::column_id_t columnID) const;
    std::vector<common::column_id_t> getLocalColumnIDs(
        const std::vector<common::column_id_t>& columnIDs) const;
    // L'inverse, pour chaque position de ces groupes : son numéro de colonne.
    std::vector<common::column_id_t> getCommittedColumnIDs() const;

private:
    void initLocalHashIndex(MemoryManager& mm);
    bool isVisible(const transaction::Transaction* transaction, common::offset_t offset) const;

private:
    // This is equivalent to the num of committed nodes in the table.
    common::offset_t startOffset;
    PageCursor overflowCursor;
    std::unique_ptr<OverflowFile> overflowFile;
    OverflowFileHandle* overflowFileHandle;
    std::unique_ptr<LocalHashIndex> hashIndex;
    NodeGroupCollection nodeGroups;
    // Indexé par numéro de colonne : la position de la propriété (getPropertyPosition).
    std::vector<common::idx_t> positionOfColumn;
    // Le type de la clé, pris au catalogue : le numéro de colonne de la clé gardé par la table
    // ne suit pas la renumérotation d'un point de reprise.
    common::PhysicalTypeID pkPhysicalType;
};

} // namespace storage
} // namespace rag3db
