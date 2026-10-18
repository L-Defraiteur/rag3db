#pragma once

#include "common/data_chunk/sel_vector.h"
#include "common/types/types.h"

namespace rag3db {
namespace transaction {
class Transaction;
} // namespace transaction

namespace storage {

class ChunkedNodeGroup;
struct VectorVersionInfo;

class RAG3DB_API VersionInfo {
public:
    VersionInfo();
    ~VersionInfo();
    DELETE_BOTH_COPY(VersionInfo);

    void append(common::transaction_t transactionID, common::row_idx_t startRow,
        common::row_idx_t numRows);
    bool delete_(common::transaction_t transactionID, common::row_idx_t rowIdx);

    bool isSelected(common::transaction_t startTS, common::transaction_t transactionID,
        common::row_idx_t rowIdx) const;
    void getSelVectorToScan(common::transaction_t startTS, common::transaction_t transactionID,
        common::SelectionVector& selVector, common::row_idx_t startRow,
        common::row_idx_t numRows) const;

    void clearVectorInfo(common::idx_t vectorIdx);

    bool hasDeletions() const;
    common::row_idx_t getNumDeletions(const transaction::Transaction* transaction,
        common::row_idx_t startRow, common::length_t numRows) const;
    bool hasInsertions() const;
    bool isDeleted(const transaction::Transaction* transaction, common::row_idx_t rowInChunk) const;
    bool isInserted(const transaction::Transaction* transaction,
        common::row_idx_t rowInChunk) const;
    // La même visibilité depuis un instantané choisi (Transaction::LATEST_COMMITTED_TS : le
    // dernier état validé, plus les lignes de la transaction donnée).
    bool isDeleted(common::transaction_t startTS, common::transaction_t transactionID,
        common::row_idx_t rowInChunk) const;
    bool isInserted(common::transaction_t startTS, common::transaction_t transactionID,
        common::row_idx_t rowInChunk) const;

    bool hasDeletions(const transaction::Transaction* transaction) const;

    common::idx_t getNumVectors() const { return vectorsInfo.size(); }

    void commitInsert(common::row_idx_t startRow, common::row_idx_t numRows,
        common::transaction_t commitTS);
    void rollbackInsert(common::row_idx_t startRow, common::row_idx_t numRows);
    void commitDelete(common::row_idx_t startRow, common::row_idx_t numRows,
        common::transaction_t commitTS);
    void rollbackDelete(common::row_idx_t startRow, common::row_idx_t numRows);

    void serialize(common::Serializer& serializer) const;
    static std::unique_ptr<VersionInfo> deserialize(common::Deserializer& deSer);

    // Le résumé du bloc (page coeur-cpp/13 §6) : quand toutes ses insertions sont validées au
    // plus tard à l'instantané et qu'aucune de ses suppressions n'est encore à lire, chacune de
    // ses lignes est visible — une lecture n'a rien à vérifier ligne à ligne (isSelected,
    // getSelVectorToScan, isInserted le rendent en un test). Tenu par append, commitInsert,
    // rollbackInsert, delete_ et rollbackDelete ; une suppression cesse d'être à lire quand la
    // compaction du CSR a retiré sa ligne de toute liste (forgetDeletion : plus personne ne la
    // lit, et les tables de nœuds, lues par décalage, n'oublient jamais). Un résumé chargé du
    // disque ne promet rien.
    bool allRowsVisibleTo(common::transaction_t startTS) const {
        return insertSummaryValid && listedDeletions == 0 && uncommittedInsertRows == 0 &&
               maxInsertCommitTS <= startTS;
    }
    // La ligne supprimée (validée) n'est plus dans aucune liste : sa suppression n'a plus à être
    // lue (CSRNodeGroup::compactDeadRowsNoLock).
    void forgetDeletion() {
        if (listedDeletions > 0) {
            listedDeletions--;
        }
    }

private:
    // Return nullptr when vectorIdx is out of range or when the vector is not created.
    VectorVersionInfo* getVectorVersionInfo(common::idx_t vectorIdx) const;
    VectorVersionInfo& getOrCreateVersionInfo(common::idx_t vectorIdx);

    std::vector<std::unique_ptr<VectorVersionInfo>> vectorsInfo;
    uint64_t uncommittedInsertRows = 0;
    common::transaction_t maxInsertCommitTS = 0;
    // Les suppressions encore à lire : marquées (validées ou non) et non oubliées.
    uint64_t listedDeletions = 0;
    bool insertSummaryValid = true;
};

} // namespace storage
} // namespace rag3db
