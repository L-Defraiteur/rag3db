#include "common/assert.h"
#include "parser/transaction_statement.h"
#include "parser/transformer.h"

using namespace rag3db::transaction;
using namespace rag3db::common;

namespace rag3db {
namespace parser {

std::unique_ptr<Statement> Transformer::transformTransaction(
    CypherParser::KU_TransactionContext& ctx) {
    if (ctx.TRANSACTION()) {
        if (ctx.READ()) {
            return std::make_unique<TransactionStatement>(TransactionAction::BEGIN_READ);
        }
        return std::make_unique<TransactionStatement>(TransactionAction::BEGIN_WRITE);
    }
    if (ctx.COMMIT()) {
        return std::make_unique<TransactionStatement>(TransactionAction::COMMIT);
    }
    if (ctx.ROLLBACK()) {
        return std::make_unique<TransactionStatement>(TransactionAction::ROLLBACK);
    }
    if (ctx.CHECKPOINT()) {
        return std::make_unique<TransactionStatement>(TransactionAction::CHECKPOINT);
    }
    KU_UNREACHABLE;
}

} // namespace parser
} // namespace rag3db
