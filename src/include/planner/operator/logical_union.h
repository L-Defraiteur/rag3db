#pragma once

#include "logical_operator.h"

namespace rag3db {
namespace planner {

class LogicalUnion : public LogicalOperator {
public:
    LogicalUnion(binder::expression_vector expressions,
        const std::vector<std::shared_ptr<LogicalOperator>>& children)
        : LogicalOperator{LogicalOperatorType::UNION_ALL, children},
          expressionsToUnion{std::move(expressions)} {}

    // La liste des projections de chaque branche, telle qu'elle est écrite : le schéma d'une
    // branche dédoublonne ses expressions (RETURN b.age, b.age n'y occupe qu'une place), et
    // ne peut donc pas servir à les retrouver par position.
    void setChildProjections(std::vector<binder::expression_vector> projections) {
        childProjections = std::move(projections);
    }
    const std::vector<binder::expression_vector>& getChildProjections() const {
        return childProjections;
    }

    f_group_pos_set getGroupsPosToFlatten(uint32_t childIdx);

    void computeFactorizedSchema() override;
    void computeFlatSchema() override;

    std::string getExpressionsForPrinting() const override { return std::string{}; }

    binder::expression_vector getExpressionsToUnion() const { return expressionsToUnion; }

    Schema* getSchemaBeforeUnion(uint32_t idx) const { return children[idx]->getSchema(); }

    std::unique_ptr<LogicalOperator> copy() override;

private:
    // If an expression to union has different flat/unflat state in different child, we
    // need to flatten that expression in all the single queries.
    bool requireFlatExpression(uint32_t expressionIdx);

private:
    binder::expression_vector expressionsToUnion;
    std::vector<binder::expression_vector> childProjections;
};

} // namespace planner
} // namespace rag3db
