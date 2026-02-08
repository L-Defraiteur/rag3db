#pragma once

#include "binder/bound_statement_visitor.h"
#include "binder/query/query_graph_label_analyzer.h"

namespace rag3db {
namespace binder {

class MatchClausePatternLabelRewriter final : public BoundStatementVisitor {
public:
    explicit MatchClausePatternLabelRewriter(const main::ClientContext& clientContext)
        : analyzer{clientContext, false /* throwOnViolate */} {}

    void visitMatchUnsafe(BoundReadingClause& readingClause) override;

private:
    QueryGraphLabelAnalyzer analyzer;
};

} // namespace binder
} // namespace rag3db
