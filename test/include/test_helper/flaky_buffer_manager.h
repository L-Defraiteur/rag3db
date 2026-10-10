#pragma once

#include <atomic>
#include <exception>
#include <functional>
#include <string>

#include "main/client_context.h"
#include "storage/buffer_manager/buffer_manager.h"
#include "transaction/transaction.h"
#include "transaction/transaction_manager.h"

namespace rag3db {
namespace testing {

// A buffer manager that refuses one reservation out of `failureFrequency`, in the phases it is
// allowed to fail in. Shared by the copy tests and the checkpoint recovery tests.
class FlakyBufferManager : public storage::BufferManager {
public:
    FlakyBufferManager(const std::string& databasePath, const std::string& spillToDiskPath,
        uint64_t bufferPoolSize, uint64_t maxDBSize, common::VirtualFileSystem* vfs, bool readOnly,
        std::atomic<uint64_t>& failureFrequency, bool canFailDuringExecute,
        bool canFailDuringCheckpoint, bool canFailDuringCommit)
        : storage::BufferManager(databasePath, spillToDiskPath, bufferPoolSize, maxDBSize, vfs,
              readOnly),
          failureFrequency(failureFrequency), canFailDuringCheckpoint(canFailDuringCheckpoint),
          canFailDuringExecute(canFailDuringExecute), canFailDuringCommit(canFailDuringCommit) {}

    bool reserve(uint64_t sizeToReserve) override {
        // we currently can't handle exceptions thrown during rollback
        const bool inRollback = std::current_exception().operator bool();

        const bool inDBInit = ctx == nullptr;

        const bool inCheckpoint =
            ctx && !transaction::TransactionManager::Get(*ctx)->hasActiveWriteTransactionNoLock();
        const bool inCommit =
            !inCheckpoint && ctx && transaction::Transaction::Get(*ctx) &&
            transaction::Transaction::Get(*ctx)->getCommitTS() != common::INVALID_TRANSACTION;
        const bool inExecute = (!inCommit && !inCheckpoint);
        // Pour viser les réservations d'une page de 4 Kio, dont celles du journal local d'une
        // transaction (InMemFileWriter) : les autres ne sont ni comptées ni refusées.
        if (onlyFailPageReservations && sizeToReserve != common::RAG3DB_PAGE_SIZE) {
            return storage::BufferManager::reserve(sizeToReserve);
        }
        numReservations++;
        reserveCount = (reserveCount + 1) % failureFrequency;
        if (!inRollback && !inDBInit && (canFailDuringCommit || !inCommit) &&
            (canFailDuringCheckpoint || !inCheckpoint) && (canFailDuringExecute || !inExecute) &&
            reserveCount == 0) {
            failureFrequency = failureFrequency * 2;
            if (onRefusal) {
                onRefusal();
            }
            return false;
        }
        return storage::BufferManager::reserve(sizeToReserve);
    }

    void setClientContext(main::ClientContext* newCtx) { ctx = newCtx; }

    std::atomic<uint64_t>& failureFrequency;
    main::ClientContext* ctx{nullptr};
    bool canFailDuringCheckpoint;
    bool canFailDuringExecute;
    bool canFailDuringCommit;
    std::atomic<uint64_t> reserveCount = 0;
    bool onlyFailPageReservations = false;
    // Les réservations vues (celles qui avancent reserveCount), pour qu'un test règle sa
    // fréquence de refus sur la taille réelle d'une requête.
    std::atomic<uint64_t> numReservations = 0;
    // Appelé à chaque refus, dans le fil qui réserve : un test y relève où en est la requête.
    std::function<void()> onRefusal;
};

} // namespace testing
} // namespace rag3db
