#include "parser/expression/parsed_expression_visitor.h"

#include "catalog/catalog.h"
#include "catalog/catalog_entry/function_catalog_entry.h"
#include "common/exception/not_implemented.h"
#include "parser/expression/parsed_case_expression.h"
#include "parser/expression/parsed_function_expression.h"
#include "parser/expression/parsed_lambda_expression.h"
#include "parser/expression/parsed_subquery_expression.h"
#include "transaction/transaction.h"

using namespace rag3db::common;
using namespace rag3db::catalog;

namespace rag3db {
namespace parser {

void ParsedExpressionVisitor::visit(const ParsedExpression* expr) {
    visitChildren(*expr);
    visitSwitch(expr);
}

void ParsedExpressionVisitor::visitUnsafe(ParsedExpression* expr) {
    visitChildrenUnsafe(*expr);
    visitSwitchUnsafe(expr);
}

void ParsedExpressionVisitor::visitSwitch(const ParsedExpression* expr) {
    switch (expr->getExpressionType()) {
    case ExpressionType::OR:
    case ExpressionType::XOR:
    case ExpressionType::AND:
    case ExpressionType::NOT:
    case ExpressionType::EQUALS:
    case ExpressionType::NOT_EQUALS:
    case ExpressionType::GREATER_THAN:
    case ExpressionType::GREATER_THAN_EQUALS:
    case ExpressionType::LESS_THAN:
    case ExpressionType::LESS_THAN_EQUALS:
    case ExpressionType::IS_NULL:
    case ExpressionType::IS_NOT_NULL:
    case ExpressionType::FUNCTION: {
        visitFunctionExpr(expr);
    } break;
    case ExpressionType::AGGREGATE_FUNCTION: {
        visitAggFunctionExpr(expr);
    } break;
    case ExpressionType::PROPERTY: {
        visitPropertyExpr(expr);
    } break;
    case ExpressionType::LITERAL: {
        visitLiteralExpr(expr);
    } break;
    case ExpressionType::VARIABLE: {
        visitVariableExpr(expr);
    } break;
    case ExpressionType::PATH: {
        visitPathExpr(expr);
    } break;
    case ExpressionType::PATTERN: {
        visitNodeRelExpr(expr);
    } break;
    case ExpressionType::PARAMETER: {
        visitParamExpr(expr);
    } break;
    case ExpressionType::SUBQUERY: {
        visitSubqueryExpr(expr);
    } break;
    case ExpressionType::CASE_ELSE: {
        visitCaseExpr(expr);
    } break;
    case ExpressionType::GRAPH: {
        visitGraphExpr(expr);
    } break;
    case ExpressionType::LAMBDA: {
        visitLambdaExpr(expr);
    } break;
    case ExpressionType::STAR: {
        visitStar(expr);
    } break;
        // LCOV_EXCL_START
    default:
        throw NotImplementedException("ExpressionVisitor::visitSwitch");
        // LCOV_EXCL_STOP
    }
}

void ParsedExpressionVisitor::visitChildren(const ParsedExpression& expr) {
    switch (expr.getExpressionType()) {
    case ExpressionType::CASE_ELSE: {
        visitCaseChildren(expr);
    } break;
    case ExpressionType::LAMBDA: {
        auto& lambda = expr.constCast<ParsedLambdaExpression>();
        visit(lambda.getFunctionExpr());
    } break;
    case ExpressionType::SUBQUERY: {
        visitSubqueryChildren(expr);
    } break;
    default: {
        for (auto i = 0u; i < expr.getNumChildren(); ++i) {
            visit(expr.getChild(i));
        }
    }
    }
}

// Les expressions d'une sous-requête (EXISTS, COUNT) ne sont pas des enfants au sens de
// getChild : elles vivent dans ses motifs (propriétés, WHERE et projections d'une relation
// récursive) et dans sa clause WHERE. Un visiteur qui ne les parcourt pas ne voit pas un paramètre
// écrit dedans, et la préparation ne l'enregistre pas.
void ParsedExpressionVisitor::visitSubqueryChildren(const ParsedExpression& expr) {
    auto& subquery = expr.constCast<ParsedSubqueryExpression>();
    for (auto& element : subquery.getPatternElements()) {
        visitNodePattern(*element.getFirstNodePattern());
        for (auto i = 0u; i < element.getNumPatternElementChains(); ++i) {
            auto chain = element.getPatternElementChain(i);
            visitRelPattern(*chain->getRelPattern());
            visitNodePattern(*chain->getNodePattern());
        }
    }
    if (subquery.hasWhereClause()) {
        visit(subquery.getWhereClause());
    }
}

void ParsedExpressionVisitor::visitNodePattern(const NodePattern& pattern) {
    for (auto& [_, value] : pattern.getPropertyKeyVals()) {
        visit(value.get());
    }
}

void ParsedExpressionVisitor::visitRelPattern(const RelPattern& pattern) {
    visitNodePattern(pattern);
    auto info = pattern.getRecursiveInfo();
    if (info->whereExpression != nullptr) {
        visit(info->whereExpression.get());
    }
    for (auto& projected : info->relProjectionList) {
        visit(projected.get());
    }
    for (auto& projected : info->nodeProjectionList) {
        visit(projected.get());
    }
}

void ParsedExpressionVisitor::visitChildrenUnsafe(ParsedExpression& expr) {
    switch (expr.getExpressionType()) {
    case ExpressionType::CASE_ELSE: {
        visitCaseChildrenUnsafe(expr);
    } break;
    default: {
        for (auto i = 0u; i < expr.getNumChildren(); ++i) {
            visitUnsafe(expr.getChild(i));
        }
    }
    }
}

void ParsedExpressionVisitor::visitCaseChildren(const ParsedExpression& expr) {
    auto& caseExpr = expr.constCast<ParsedCaseExpression>();
    if (caseExpr.hasCaseExpression()) {
        visit(caseExpr.getCaseExpression());
    }
    for (auto i = 0u; i < caseExpr.getNumCaseAlternative(); ++i) {
        auto alternative = caseExpr.getCaseAlternative(i);
        visit(alternative->whenExpression.get());
        visit(alternative->thenExpression.get());
    }
    if (caseExpr.hasElseExpression()) {
        visit(caseExpr.getElseExpression());
    }
}

void ParsedExpressionVisitor::visitCaseChildrenUnsafe(ParsedExpression& expr) {
    auto& caseExpr = expr.cast<ParsedCaseExpression>();
    if (caseExpr.hasCaseExpression()) {
        visitUnsafe(caseExpr.getCaseExpression());
    }
    for (auto i = 0u; i < caseExpr.getNumCaseAlternative(); ++i) {
        auto alternative = caseExpr.getCaseAlternative(i);
        visitUnsafe(alternative->whenExpression.get());
        visitUnsafe(alternative->thenExpression.get());
    }
    if (caseExpr.hasElseExpression()) {
        visitUnsafe(caseExpr.getElseExpression());
    }
}

void ReadWriteExprAnalyzer::visitFunctionExpr(const ParsedExpression* expr) {
    if (expr->getExpressionType() != ExpressionType::FUNCTION) {
        // Can be AND/OR/... which guarantees to be readonly.
        return;
    }
    auto funcName = expr->constCast<ParsedFunctionExpression>().getFunctionName();
    auto catalog = Catalog::Get(*context);
    // Assume user cannot add function with sideeffect, i.e. all non-readonly function is
    // registered when database starts.
    auto transaction = &transaction::DUMMY_TRANSACTION;
    if (!catalog->containsFunction(transaction, funcName)) {
        return;
    }
    auto entry = catalog->getFunctionEntry(transaction, funcName);
    if (entry->getType() != CatalogEntryType::SCALAR_FUNCTION_ENTRY) {
        // Can be macro function which guarantees to be readonly.
        return;
    }
    auto& funcSet = entry->constPtrCast<FunctionCatalogEntry>()->getFunctionSet();
    KU_ASSERT(!funcSet.empty());
    if (!funcSet[0]->isReadOnly) {
        readOnly = false;
    }
}

std::unique_ptr<ParsedExpression> MacroParameterReplacer::replace(
    std::unique_ptr<ParsedExpression> input) {
    if (nameToExpr.contains(input->getRawName())) {
        return nameToExpr.at(input->getRawName())->copy();
    }
    visitUnsafe(input.get());
    return input;
}

void MacroParameterReplacer::visitSwitchUnsafe(ParsedExpression* expr) {
    switch (expr->getExpressionType()) {
    case ExpressionType::CASE_ELSE: {
        auto& caseExpr = expr->cast<ParsedCaseExpression>();
        if (caseExpr.hasCaseExpression()) {
            auto replace = getReplace(caseExpr.getCaseExpression()->getRawName());
            if (replace) {
                caseExpr.setCaseExpression(std::move(replace));
            }
        }
        for (auto i = 0u; i < caseExpr.getNumCaseAlternative(); i++) {
            auto caseAlternative = caseExpr.getCaseAlternativeUnsafe(i);
            auto whenReplace = getReplace(caseAlternative->whenExpression->getRawName());
            auto thenReplace = getReplace(caseAlternative->thenExpression->getRawName());
            if (whenReplace) {
                caseAlternative->whenExpression = std::move(whenReplace);
            }
            if (thenReplace) {
                caseAlternative->thenExpression = std::move(thenReplace);
            }
        }
        if (caseExpr.hasElseExpression()) {
            auto replace = getReplace(caseExpr.getElseExpression()->getRawName());
            if (replace) {
                caseExpr.setElseExpression(std::move(replace));
            }
        }
    } break;
    default: {
        for (auto i = 0u; i < expr->getNumChildren(); ++i) {
            auto child = expr->getChild(i);
            auto replace = getReplace(child->getRawName());
            if (replace) {
                expr->setChild(i, std::move(replace));
            }
        }
    }
    }
}

std::unique_ptr<ParsedExpression> MacroParameterReplacer::getReplace(const std::string& name) {
    if (nameToExpr.contains(name)) {
        return nameToExpr.at(name)->copy();
    }
    return nullptr;
}

} // namespace parser
} // namespace rag3db
