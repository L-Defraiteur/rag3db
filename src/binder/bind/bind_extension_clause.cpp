#include "binder/binder.h"
#include "extension/binder_extension.h"

using namespace rag3db::common;
using namespace rag3db::parser;

namespace rag3db {
namespace binder {

std::unique_ptr<BoundStatement> Binder::bindExtensionClause(const parser::Statement& statement) {
    for (auto& binderExtension : binderExtensions) {
        auto boundStatement = binderExtension->bind(statement);
        if (boundStatement) {
            return boundStatement;
        }
    }
    KU_UNREACHABLE;
}

} // namespace binder
} // namespace rag3db
