#pragma once

#include <utility>

#include "main/client_context.h"
#include "transaction/transaction_manager.h"

namespace rag3db {
namespace testing {

// The test-only way to make a database use another Checkpointer: it is the class
// TransactionManager befriends. Shared by the tests that need a checkpoint to stop or fail at a
// chosen point.
class FlakyCheckpointer {
public:
    explicit FlakyCheckpointer(transaction::TransactionManager::init_checkpointer_func_t initFunc)
        : initFunc(std::move(initFunc)) {}

    void setCheckpointer(main::ClientContext& context) const {
        transaction::TransactionManager::Get(context)->initCheckpointerFunc = initFunc;
    }

private:
    transaction::TransactionManager::init_checkpointer_func_t initFunc;
};

} // namespace testing
} // namespace rag3db
