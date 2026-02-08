#pragma once

#include "binder/expression/expression.h"
#include "common/types/value/value.h"

namespace rag3db {
namespace evaluator {

struct ExpressionEvaluatorUtils {
    static RAG3DB_API common::Value evaluateConstantExpression(
        std::shared_ptr<binder::Expression> expression, main::ClientContext* clientContext);
};

} // namespace evaluator
} // namespace rag3db
