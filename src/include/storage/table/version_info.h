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

    // V2-2 : la version brute d'une ligne (INVALID_TRANSACTION : aucune, la ligne est insérée
    // pour tous, ou jamais supprimée), et sa pose directe, sans jugement ni conflit : c'est ce
    // qui porte dans un bloc neuf les versions postérieures au point de fonte.
    common::transaction_t insertionVersionOf(common::row_idx_t rowIdx) const;
    common::transaction_t deletionVersionOf(common::row_idx_t rowIdx) const;
    void setInsertion(common::row_idx_t rowIdx, common::transaction_t version);
    void setDeletion(common::row_idx_t rowIdx, common::transaction_t version);

    void commitInsert(common::row_idx_t startRow, common::row_idx_t numRows,
        common::transaction_t commitTS);
    void rollbackInsert(common::row_idx_t startRow, common::row_idx_t numRows);
    void commitDelete(common::row_idx_t startRow, common::row_idx_t numRows,
        common::transaction_t commitTS);
    void rollbackDelete(common::row_idx_t startRow, common::row_idx_t numRows);

    void serialize(common::Serializer& serializer) const;
    static std::unique_ptr<VersionInfo> deserialize(common::Deserializer& deSer);

private:
    // Return nullptr when vectorIdx is out of range or when the vector is not created.
    VectorVersionInfo* getVectorVersionInfo(common::idx_t vectorIdx) const;
    VectorVersionInfo& getOrCreateVersionInfo(common::idx_t vectorIdx);

    std::vector<std::unique_ptr<VectorVersionInfo>> vectorsInfo;
};

} // namespace storage
} // namespace rag3db
