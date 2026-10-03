#pragma once

#include <mutex>

#include "transaction.h"

namespace rag3db {

namespace main {
class ClientContext;
}

namespace transaction {

/**
 * If the connection is in AUTO_COMMIT mode, any query over the connection will be wrapped around
 * a transaction and committed (even if the query is READ_ONLY).
 * If the connection is in MANUAL transaction mode, which happens only if an application
 * manually begins a transaction (see below), then an application has to manually commit or
 * rollback the transaction by calling commit() or rollback().
 *
 * AUTO_COMMIT is the default mode when a Connection is created. If an application calls
 * begin[ReadOnly/Write]Transaction at any point, the mode switches to MANUAL. This creates
 * an "active transaction" in the connection. When a connection is in MANUAL mode and the
 * active transaction is rolled back or committed, then the active transaction is removed (so
 * the connection no longer has an active transaction), and the mode automatically switches
 * back to AUTO_COMMIT.
 * Note: When a Connection object is deconstructed, if the connection has an active (manual)
 * transaction, then the active transaction is rolled back.
 */
enum class TransactionMode : uint8_t { AUTO = 0, MANUAL = 1 };

class RAG3DB_API TransactionContext {
public:
    explicit TransactionContext(main::ClientContext& clientContext);
    ~TransactionContext();

    bool isAutoTransaction() const { return mode == TransactionMode::AUTO; }

    void beginReadTransaction();
    void beginWriteTransaction();
    void beginAutoTransaction(bool readOnlyStatement);
    void beginRecoveryTransaction();
    void validateManualTransaction(bool readOnlyStatement) const;

    void commit();
    void rollback();
    // Rolls back after a statement failed. If the transaction was opened by BEGIN, the connection
    // then refuses every statement until ROLLBACK (see MANUAL_TRANSACTION_FAILED).
    void rollbackAfterStatementFailure();

    // The start of the error every statement gets between the failure of a statement inside a
    // BEGIN ... COMMIT block and the ROLLBACK that closes the block. Without this state the
    // statements that follow the failure ran one by one in auto-commit, and a client that did not
    // read every result lost atomicity without knowing. PostgreSQL does the same ("current
    // transaction is aborted, commands ignored until end of transaction block").
    static constexpr const char* MANUAL_TRANSACTION_FAILED =
        "The transaction was rolled back because a statement in it failed";
    bool isManualTransactionFailed() const { return manualTransactionFailed; }
    // Closes the failed block: what ROLLBACK does, and COMMIT too, since there is nothing left
    // to commit.
    void closeFailedManualTransaction() { manualTransactionFailed = false; }
    // Throws MANUAL_TRANSACTION_FAILED if the connection is in a failed block.
    void throwIfManualTransactionFailed() const;

    TransactionMode getTransactionMode() const { return mode; }
    bool hasActiveTransaction() const { return activeTransaction != nullptr; }
    Transaction* getActiveTransaction() const { return activeTransaction; }

    void clearTransaction();

    static TransactionContext* Get(const main::ClientContext& context);

private:
    void beginTransactionInternal(TransactionType transactionType);

private:
    std::mutex mtx;
    main::ClientContext& clientContext;
    TransactionMode mode;
    Transaction* activeTransaction;
    bool manualTransactionFailed = false;
};

} // namespace transaction
} // namespace rag3db
