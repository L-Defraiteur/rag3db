#pragma once

#include "planner/operator/logical_operator.h"

namespace rag3db {
namespace planner {

class LogicalDummyScan final : public LogicalOperator {
public:
    // Une ligne, connue : la source des plans sans lecture (UNWIND d'un paramètre, CREATE…), que
    // le processeur mappe en appel de fonction de table — sans estimation, ses tâches gardaient
    // tous les fils (page coeur-cpp/12 §8).
    explicit LogicalDummyScan() : LogicalOperator{LogicalOperatorType::DUMMY_SCAN} {
        setCardinality(1);
    }

    void computeFactorizedSchema() override;
    void computeFlatSchema() override;

    inline std::string getExpressionsForPrinting() const override { return std::string(); }

    static std::shared_ptr<binder::Expression> getDummyExpression();

    inline std::unique_ptr<LogicalOperator> copy() override {
        return std::make_unique<LogicalDummyScan>();
    }
};

} // namespace planner
} // namespace rag3db
