#include "planner/operator/logical_unwind.h"

#include "binder/expression/literal_expression.h"
#include "binder/expression/parameter_expression.h"
#include "binder/expression/scalar_function_expression.h"
#include "function/list/vector_list_functions.h"

#include "planner/operator/factorization/flatten_resolver.h"

using namespace rag3db::binder;
using namespace rag3db::common;

namespace rag3db {
namespace planner {

f_group_pos_set LogicalUnwind::getGroupsPosToFlatten() {
    auto childSchema = children[0]->getSchema();
    return FlattenAll::getGroupsPosToFlatten(inExpr, *childSchema);
}

void LogicalUnwind::computeFactorizedSchema() {
    copyChildSchema(0);
    auto groupPos = schema->createGroup();
    schema->insertToGroupAndScope(outExpr, groupPos);
    if (hasIDExpr()) {
        schema->insertToGroupAndScope(idExpr, groupPos);
    }
}

void LogicalUnwind::computeFlatSchema() {
    copyChildSchema(0);
    schema->insertToGroupAndScope(outExpr, 0);
    if (hasIDExpr()) {
        schema->insertToGroupAndScope(idExpr, 0);
    }
}

std::optional<uint64_t> LogicalUnwind::knownListSize() const {
    using namespace binder;
    switch (inExpr->expressionType) {
    case common::ExpressionType::LITERAL:
        return inExpr->constCast<LiteralExpression>().getValue().getChildrenSize();
    case common::ExpressionType::PARAMETER:
        return inExpr->constCast<ParameterExpression>().getValue().getChildrenSize();
    case common::ExpressionType::FUNCTION:
        // `[a, b, c]` écrit dans la requête : un appel de LIST_CREATION, un enfant par élément.
        if (inExpr->constCast<ScalarFunctionExpression>().getFunction().name ==
            function::ListCreationFunction::name) {
            return inExpr->getNumChildren();
        }
        return std::nullopt;
    default:
        return std::nullopt;
    }
}

} // namespace planner
} // namespace rag3db
