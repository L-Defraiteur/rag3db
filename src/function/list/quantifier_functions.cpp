#include "common/types/types.h"
#include "common/vector/value_vector.h"
#include "expression_evaluator/lambda_evaluator.h"
#include "expression_evaluator/list_slice_info.h"
#include "function/function.h"
#include "function/list/vector_list_functions.h"

using namespace rag3db::common;

namespace rag3db {
namespace function {

void execQuantifierFunc(quantifier_handler handler,
    const std::vector<std::shared_ptr<common::ValueVector>>& input,
    const std::vector<common::SelectionVector*>& inputSelVectors, common::ValueVector& result,
    common::SelectionVector* resultSelVector, void* bindData) {
    auto data = reinterpret_cast<evaluator::ListLambdaBindData*>(bindData);
    auto* slice = data->sliceInfo;
    auto& inputVector = *input[0];
    auto& filterVector = *input[1];
    auto saved = slice->overrideAndSaveParamStates(data->lambdaParamEvaluators);
    if (slice->getSliceSize() > 0 || data->lambdaParamEvaluators.empty()) {
        data->rootEvaluator->evaluate();
        for (sel_t i = 0; i < slice->getSliceSize(); ++i) {
            const auto [listPos, dataPos] = slice->getPos(i);
            (void)dataPos;
            const auto filterPos = filterVector.state->getSelVector()[
                filterVector.state->isFlat() ? 0 : i];
            if (!filterVector.isNull(filterPos) && filterVector.getValue<bool>(filterPos)) {
                ++data->quantifierCounts[listPos];
            }
        }
    }
    if (slice->done()) {
        auto& selection = *inputSelVectors[0];
        for (sel_t i = 0; i < selection.getSelSize(); ++i) {
            const auto pos = selection[i];
            const auto outPos = (*resultSelVector)[i];
            result.setNull(outPos, inputVector.isNull(pos));
            if (!inputVector.isNull(pos)) {
                const auto entry = inputVector.getValue<list_entry_t>(pos);
                result.setValue(outPos, handler(data->quantifierCounts[pos], entry.size));
            }
        }
    }
    slice->restoreParamStates(data->lambdaParamEvaluators, std::move(saved));
}

std::unique_ptr<FunctionBindData> bindQuantifierFunc(const ScalarBindFuncInput& input) {
    std::vector<common::LogicalType> paramTypes;
    paramTypes.push_back(input.arguments[0]->getDataType().copy());
    paramTypes.push_back(input.arguments[1]->getDataType().copy());
    return std::make_unique<FunctionBindData>(std::move(paramTypes), common::LogicalType::BOOL());
}

} // namespace function
} // namespace rag3db
