#include "processor/operator/multiplicity_reducer.h"

namespace rag3db {
namespace processor {

bool MultiplicityReducer::getNextTuplesInternal(ExecutionContext* context) {
    if (numRepeat == 0) {
        restoreMultiplicity();
        if (!children[0]->getNextTuple(context)) {
            return false;
        }
        saveMultiplicity();
        resultSet->multiplicity = 1;
    }
    numRepeat++;
    if (numRepeat == prevMultiplicity) {
        numRepeat = 0;
    }
    return true;
}

} // namespace processor
} // namespace rag3db
