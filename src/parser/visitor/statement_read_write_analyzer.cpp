#include "parser/visitor/statement_read_write_analyzer.h"

#include "catalog/catalog.h"
#include "catalog/catalog_entry/function_catalog_entry.h"
#include "function/table/table_function.h"
#include "main/client_context.h"
#include "parser/expression/parsed_function_expression.h"
#include "parser/standalone_call_function.h"
#include "transaction/transaction.h"
#include "main/db_config.h"
#include "parser/expression/parsed_expression_visitor.h"
#include "parser/query/reading_clause/reading_clause.h"
#include "parser/query/return_with_clause/with_clause.h"

namespace rag3db {
namespace parser {

void StatementReadWriteAnalyzer::visitExtension(const Statement& /*statement*/) {
    // We allow LOAD EXTENSION to run in read-only mode.
    if (context->getDBConfig()->readOnly) {
        readOnly = true;
    } else {
        readOnly = false;
    }
}

void StatementReadWriteAnalyzer::visitStandaloneCallFunction(const Statement& statement) {
    readOnly = false;
    const auto& call = statement.constCast<StandaloneCallFunction>();
    const auto* expr = call.getFunctionExpression();
    if (!expr || expr->getExpressionType() != common::ExpressionType::FUNCTION) {
        return;
    }
    const auto funcName = expr->constCast<ParsedFunctionExpression>().getFunctionName();
    auto catalog = catalog::Catalog::Get(*context);
    auto transaction = &transaction::DUMMY_TRANSACTION;
    if (!catalog->containsFunction(transaction, funcName)) {
        return;
    }
    const auto entry = catalog->getFunctionEntry(transaction, funcName);
    if (entry->getType() != catalog::CatalogEntryType::STANDALONE_TABLE_FUNCTION_ENTRY) {
        return;
    }
    const auto& funcSet = entry->constPtrCast<catalog::FunctionCatalogEntry>()->getFunctionSet();
    // Un graphe projeté (PROJECT_GRAPH, PROJECT_GRAPH_CYPHER, DROP_PROJECTED_GRAPH) vit dans la
    // connexion : l'appel passe dans une transaction READ ONLY, comme une lecture.
    readOnly = !funcSet.empty() &&
               funcSet[0]->constPtrCast<function::TableFunction>()->connectionScopedOnly;
}

void StatementReadWriteAnalyzer::visitReadingClause(const ReadingClause* readingClause) {
    if (readingClause->hasWherePredicate()) {
        if (!isExprReadOnly(readingClause->getWherePredicate())) {
            readOnly = false;
        }
    }
}

void StatementReadWriteAnalyzer::visitWithClause(const WithClause* withClause) {
    for (auto& expr : withClause->getProjectionBody()->getProjectionExpressions()) {
        if (!isExprReadOnly(expr.get())) {
            readOnly = false;
            return;
        }
    }
}

void StatementReadWriteAnalyzer::visitReturnClause(const ReturnClause* returnClause) {
    for (auto& expr : returnClause->getProjectionBody()->getProjectionExpressions()) {
        if (!isExprReadOnly(expr.get())) {
            readOnly = false;
            return;
        }
    }
}

bool StatementReadWriteAnalyzer::isExprReadOnly(const ParsedExpression* expr) {
    auto analyzer = ReadWriteExprAnalyzer(context);
    analyzer.visit(expr);
    return analyzer.isReadOnly();
}

} // namespace parser
} // namespace rag3db
