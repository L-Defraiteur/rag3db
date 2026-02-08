#include "processor/operator/empty_result.h"

namespace rag3db {
namespace processor {

bool EmptyResult::getNextTuplesInternal(ExecutionContext*) {
    return false;
}

} // namespace processor
} // namespace rag3db
