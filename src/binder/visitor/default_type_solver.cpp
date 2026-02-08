#include "binder/visitor/default_type_solver.h"

using namespace rag3db::common;

namespace rag3db {
namespace binder {

static void resolveAnyType(Expression& expr) {
    if (expr.getDataType().getLogicalTypeID() != LogicalTypeID::ANY) {
        return;
    }
    expr.cast(LogicalType::STRING());
}

void DefaultTypeSolver::visitProjectionBody(const BoundProjectionBody& projectionBody) {
    for (auto& expr : projectionBody.getProjectionExpressions()) {
        resolveAnyType(*expr);
    }
    for (auto& expr : projectionBody.getOrderByExpressions()) {
        resolveAnyType(*expr);
    }
}

} // namespace binder
} // namespace rag3db
