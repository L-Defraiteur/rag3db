#include "processor/operator/persistent/set.h"

#include "binder/expression/expression_util.h"

namespace rag3db {
namespace processor {

void SetNodeProperty::initLocalStateInternal(ResultSet* resultSet, ExecutionContext* context) {
    for (auto& executor : executors) {
        executor->init(resultSet, context);
    }
}

bool SetNodeProperty::getNextTuplesInternal(ExecutionContext* context) {
    if (!children[0]->getNextTuple(context)) {
        // La fin de l'instruction, une fois : ici et non dans finalizeInternal, que la requête
        // appelle sur l'opérateur d'origine et non sur la copie qui tient les états (comme
        // DeleteNode).
        if (!executorsFinalized) {
            executorsFinalized = true;
            for (auto& executor : executors) {
                executor->finalize(context);
            }
        }
        return false;
    }
    for (auto& executor : executors) {
        executor->set(context);
    }
    return true;
}

void SetRelProperty::initLocalStateInternal(ResultSet* resultSet, ExecutionContext* context) {
    for (auto& executor : executors) {
        executor->init(resultSet, context);
    }
}

bool SetRelProperty::getNextTuplesInternal(ExecutionContext* context) {
    if (!children[0]->getNextTuple(context)) {
        return false;
    }
    for (auto& executor : executors) {
        executor->set(context);
    }
    return true;
}

std::string SetPropertyPrintInfo::toString() const {
    std::string result = "Properties: ";
    result += binder::ExpressionUtil::toString(expressions);
    return result;
}
} // namespace processor
} // namespace rag3db
