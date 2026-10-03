#include "binder/expression_binder.h"

#include "binder/binder.h"
#include "binder/expression/expression_util.h"
#include "binder/expression/literal_expression.h"
#include "binder/expression/parameter_expression.h"
#include "binder/expression/scalar_function_expression.h"
#include "binder/expression_visitor.h"
#include "common/exception/binder.h"
#include "common/exception/not_implemented.h"
#include "common/string_format.h"
#include "common/types/value/nested.h"
#include "expression_evaluator/expression_evaluator_utils.h"
#include "function/cast/vector_cast_functions.h"
#include "function/struct/vector_struct_functions.h"
#include "parser/expression/parsed_expression_visitor.h"
#include "parser/expression/parsed_parameter_expression.h"

using namespace rag3db::common;
using namespace rag3db::function;
using namespace rag3db::parser;

namespace rag3db {
namespace binder {

std::shared_ptr<Expression> ExpressionBinder::bindExpression(
    const ParsedExpression& parsedExpression) {
    // Normally u can only reference an existing expression through alias which is a parsed
    // VARIABLE expression.
    // An exception is order by binding, e.g. RETURN a, COUNT(*) ORDER BY COUNT(*)
    // the later COUNT(*) should reference the one in projection list. So we need to explicitly
    // check scope when binding order by list.
    if (config.bindOrderByAfterAggregate && binder->scope.contains(parsedExpression.toString())) {
        return binder->scope.getExpression(parsedExpression.toString());
    }
    auto collector = ParsedParamExprCollector();
    collector.visit(&parsedExpression);
    if (collector.hasParamExprs()) {
        bool allParamExist = true;
        for (auto& parsedExpr : collector.getParamExprs()) {
            auto name = parsedExpr->constCast<ParsedParameterExpression>().getParameterName();
            if (!knownParameters.contains(name)) {
                unknownParameters.insert(name);
                allParamExist = false;
            }
        }
        if (!allParamExist) {
            auto expr = std::make_shared<ParameterExpression>(binder->getUniqueExpressionName(""),
                Value::createNullValue());
            if (parsedExpression.hasAlias()) {
                expr->setAlias(parsedExpression.getAlias());
            }
            return expr;
        }
    }
    std::shared_ptr<Expression> expression;
    auto expressionType = parsedExpression.getExpressionType();
    if (ExpressionTypeUtil::isBoolean(expressionType)) {
        expression = bindBooleanExpression(parsedExpression);
    } else if (ExpressionTypeUtil::isComparison(expressionType)) {
        expression = bindComparisonExpression(parsedExpression);
    } else if (ExpressionTypeUtil::isNullOperator(expressionType)) {
        expression = bindNullOperatorExpression(parsedExpression);
    } else if (ExpressionType::FUNCTION == expressionType) {
        expression = bindFunctionExpression(parsedExpression);
    } else if (ExpressionType::PROPERTY == expressionType) {
        expression = bindPropertyExpression(parsedExpression);
    } else if (ExpressionType::PARAMETER == expressionType) {
        expression = bindParameterExpression(parsedExpression);
    } else if (ExpressionType::LITERAL == expressionType) {
        expression = bindLiteralExpression(parsedExpression);
    } else if (ExpressionType::VARIABLE == expressionType) {
        expression = bindVariableExpression(parsedExpression);
    } else if (ExpressionType::SUBQUERY == expressionType) {
        expression = bindSubqueryExpression(parsedExpression);
    } else if (ExpressionType::CASE_ELSE == expressionType) {
        expression = bindCaseExpression(parsedExpression);
    } else if (ExpressionType::LAMBDA == expressionType) {
        expression = bindLambdaExpression(parsedExpression);
    } else {
        throw NotImplementedException(
            "bindExpression(" + ExpressionTypeUtil::toString(expressionType) + ").");
    }
    if (ConstantExpressionVisitor::needFold(*expression)) {
        return foldExpression(expression);
    }
    return expression;
}

std::shared_ptr<Expression> ExpressionBinder::foldExpression(
    const std::shared_ptr<Expression>& expression) const {
    auto value =
        evaluator::ExpressionEvaluatorUtils::evaluateConstantExpression(expression, context);
    auto result = createLiteralExpression(value);
    // Fold result should preserve the alias original expression. E.g.
    // RETURN 2, 1 + 1 AS x
    // Once folded, 1 + 1 will become 2 and have the same identifier as the first RETURN element.
    // We preserve alias (x) to avoid such conflict.
    if (expression->hasAlias()) {
        result->setAlias(expression->getAlias());
    } else {
        result->setAlias(expression->toString());
    }
    return result;
}

static std::string unsupportedImplicitCastException(const Expression& expression,
    const std::string& targetTypeStr) {
    return stringFormat(
        "Expression {} has data type {} but expected {}. Implicit cast is not supported.",
        expression.toString(), expression.dataType.toString(), targetTypeStr);
}

std::shared_ptr<Expression> ExpressionBinder::implicitCastIfNecessary(
    const std::shared_ptr<Expression>& expression, const LogicalType& targetType) {
    auto& type = expression->dataType;
    if (type == targetType || targetType.containsAny()) { // No need to cast.
        return expression;
    }
    if (auto structLiteral = castStructLiteralFieldByField(expression, targetType)) {
        return structLiteral;
    }
    if (!type.isInternalType() || !targetType.isInternalType()) {
        return implicitCast(expression, targetType);
    }
    if (ExpressionUtil::canCastStatically(*expression, targetType)) {
        expression->cast(targetType);
        return expression;
    }
    return implicitCast(expression, targetType);
}

// Gives a struct value the type `targetType` by retyping its NULL fields, at any depth. Returns
// nullptr if anything else would have to change: that is a real cast, left to the usual rules.
static std::unique_ptr<Value> retypeNullFields(const Value& value, const LogicalType& targetType) {
    if (value.isNull()) {
        return std::make_unique<Value>(Value::createNullValue(targetType));
    }
    if (value.getDataType() == targetType) {
        return std::make_unique<Value>(value);
    }
    if (value.getDataType().getLogicalTypeID() != LogicalTypeID::STRUCT ||
        targetType.getLogicalTypeID() != LogicalTypeID::STRUCT) {
        return nullptr;
    }
    const auto& fields = StructType::getFields(value.getDataType());
    const auto& targetFields = StructType::getFields(targetType);
    if (fields.size() != targetFields.size() ||
        NestedVal::getChildrenSize(&value) != fields.size()) {
        return nullptr;
    }
    std::vector<std::unique_ptr<Value>> children;
    for (auto i = 0u; i < fields.size(); i++) {
        if (fields[i].getName() != targetFields[i].getName()) {
            return nullptr;
        }
        auto child =
            retypeNullFields(*NestedVal::getChildVal(&value, i), targetFields[i].getType());
        if (!child) {
            return nullptr;
        }
        children.push_back(std::move(child));
    }
    return std::make_unique<Value>(targetType.copy(), std::move(children));
}

// A struct literal that must become another struct type is rebuilt with each field given its
// target type, instead of being cast as a whole. The difference is a NULL field: the literal gave
// it the type STRING, for want of anything better, and a list of such literals then decides the
// real type of the field from its other elements. There is no implicit cast from STRING to that
// type, so casting the struct as a whole was refused; a NULL field alone takes any type.
// The literal arrives here either folded into a value (all its fields are constants) or still
// as a call to STRUCT_PACK.
std::shared_ptr<Expression> ExpressionBinder::castStructLiteralFieldByField(
    const std::shared_ptr<Expression>& expression, const LogicalType& targetType) {
    if (expression->dataType.getLogicalTypeID() != LogicalTypeID::STRUCT ||
        targetType.getLogicalTypeID() != LogicalTypeID::STRUCT) {
        return nullptr;
    }
    if (expression->expressionType == ExpressionType::LITERAL) {
        const auto retyped =
            retypeNullFields(expression->constCast<LiteralExpression>().getValue(), targetType);
        if (!retyped) {
            return nullptr;
        }
        auto result = createLiteralExpression(*retyped);
        result->setAlias(expression->hasAlias() ? expression->getAlias() : expression->toString());
        return result;
    }
    if (expression->expressionType != ExpressionType::FUNCTION) {
        return nullptr;
    }
    const auto& functionExpression = expression->constCast<ScalarFunctionExpression>();
    if (functionExpression.getFunction().name != StructPackFunctions::name) {
        return nullptr;
    }
    const auto fieldNames = StructType::getFieldNames(expression->dataType);
    const auto& targetFields = StructType::getFields(targetType);
    const auto& children = expression->getChildren();
    if (targetFields.size() != fieldNames.size() || children.size() != fieldNames.size()) {
        return nullptr;
    }
    expression_vector castChildren;
    for (auto i = 0u; i < fieldNames.size(); i++) {
        if (targetFields[i].getName() != fieldNames[i]) {
            return nullptr;
        }
        castChildren.push_back(implicitCastIfNecessary(children[i], targetFields[i].getType()));
    }
    auto result = bindScalarFunctionExpression(castChildren, StructPackFunctions::name, fieldNames);
    if (expression->hasAlias()) {
        result->setAlias(expression->getAlias());
    }
    return result;
}

std::shared_ptr<Expression> ExpressionBinder::implicitCast(
    const std::shared_ptr<Expression>& expression, const LogicalType& targetType) {
    if (CastFunction::hasImplicitCast(expression->dataType, targetType)) {
        return forceCast(expression, targetType);
    } else {
        throw BinderException(unsupportedImplicitCastException(*expression, targetType.toString()));
    }
}

// cast without implicit checking.
std::shared_ptr<Expression> ExpressionBinder::forceCast(
    const std::shared_ptr<Expression>& expression, const LogicalType& targetType) {
    auto functionName = "CAST";
    auto children =
        expression_vector{expression, createLiteralExpression(Value(targetType.toString()))};
    return bindScalarFunctionExpression(children, functionName);
}

std::string ExpressionBinder::getUniqueName(const std::string& name) const {
    return binder->getUniqueExpressionName(name);
}

void ExpressionBinder::addParameter(const std::string& name, std::shared_ptr<Value> value) {
    KU_ASSERT(!knownParameters.contains(name));
    knownParameters[name] = value;
}

} // namespace binder
} // namespace rag3db
