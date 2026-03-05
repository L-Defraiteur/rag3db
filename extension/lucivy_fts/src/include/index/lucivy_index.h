#pragma once

#include "common/serializer/buffer_reader.h"
#include "storage/index/index.h"
#include "bridge.rs.h"

namespace rag3db {
namespace lucivy_fts_extension {

struct LucivyStorageInfo final : storage::IndexStorageInfo {
    common::offset_t numCheckpointedNodes;

    explicit LucivyStorageInfo(common::offset_t numCheckpointedNodes)
        : numCheckpointedNodes{numCheckpointedNodes} {}

    std::shared_ptr<common::BufferWriter> serialize() const override;
    static std::unique_ptr<storage::IndexStorageInfo> deserialize(
        std::unique_ptr<common::BufferReader> reader);
};

class LucivyIndex final : public storage::Index {
public:
    LucivyIndex(storage::IndexInfo indexInfo,
        std::unique_ptr<storage::IndexStorageInfo> storageInfo,
        rust::Box<::LucivyHandle> handle);

    static storage::IndexType getIndexType();

    static std::unique_ptr<storage::Index> load(main::ClientContext* context,
        storage::StorageManager* storageManager, storage::IndexInfo indexInfo,
        std::span<uint8_t> storageInfoBuffer);

    // ── CRUD hooks (called automatically by NodeTable) ──

    std::unique_ptr<InsertState> initInsertState(main::ClientContext* context,
        storage::visible_func isVisible) override;

    void insert(transaction::Transaction* transaction, const common::ValueVector& nodeIDVector,
        const std::vector<common::ValueVector*>& propertyVectors,
        InsertState& insertState) override;

    std::unique_ptr<DeleteState> initDeleteState(const transaction::Transaction* transaction,
        storage::MemoryManager* mm, storage::visible_func isVisible) override;

    void delete_(transaction::Transaction* transaction, const common::ValueVector& nodeIDVector,
        DeleteState& deleteState) override;

    std::unique_ptr<UpdateState> initUpdateState(main::ClientContext* context,
        common::column_id_t columnID, storage::visible_func isVisible) override;

    void update(transaction::Transaction* transaction,
        const common::ValueVector& nodeIDVector, common::ValueVector& propertyVector,
        UpdateState& updateState) override;

    bool needCommitInsert() const override { return false; }

    // ── Persistence ──

    void checkpointInMemory() override;
    void checkpoint(main::ClientContext* context, storage::PageAllocator& pageAllocator) override;
    void rollbackCheckpoint() override;
    void finalize(main::ClientContext* context) override;

    // ── Accessor for QUERY ──

    const ::LucivyHandle& getHandle() const { return *handle_; }

    // Flush pending writes to the Lucivy reader (commit + reload).
    // Called lazily before queries, or manually if needed.
    void flushIfDirty();

private:
    rust::Box<::LucivyHandle> handle_;
    // Cached field_ids from get_field_ids() for the indexed columns.
    std::vector<uint32_t> fieldIds_;
    // Field types parallel to fieldIds_: "text", "u64", "i64", "f64".
    std::vector<std::string> fieldTypes_;
    bool hasMixedFields_ = false;
    bool dirty_ = false;
};

} // namespace lucivy_fts_extension
} // namespace rag3db
