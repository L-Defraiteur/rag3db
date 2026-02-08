#include "function/function.h"

#include "binder/expression/expression_util.h"

using namespace rag3db::binder;
using namespace rag3db::common;

namespace rag3db {
namespace function {

std::unique_ptr<FunctionBindData> FunctionBindData::getSimpleBindData(
    const expression_vector& params, const LogicalType& resultType) {
    auto paramTypes = ExpressionUtil::getDataTypes(params);
    return std::make_unique<FunctionBindData>(std::move(paramTypes), resultType.copy());
}

} // namespace function
} // namespace rag3db
