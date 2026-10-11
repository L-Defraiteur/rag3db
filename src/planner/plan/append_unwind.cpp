#include <algorithm>
#include <optional>

#include "binder/query/reading_clause/bound_unwind_clause.h"
#include "planner/operator/logical_unwind.h"
#include "planner/planner.h"

using namespace rag3db::binder;
using namespace rag3db::common;

namespace rag3db {
namespace planner {

void Planner::appendUnwind(const BoundReadingClause& readingClause, LogicalPlan& plan) {
    auto& unwindClause = ku_dynamic_cast<const BoundUnwindClause&>(readingClause);
    auto unwind = make_shared<LogicalUnwind>(unwindClause.getInExpr(), unwindClause.getOutExpr(),
        unwindClause.getIDExpr(), plan.getLastOperator());
    appendFlattens(unwind->getGroupsPosToFlatten(), plan);
    unwind->setChild(0, plan.getLastOperator());
    // La cardinalité d'un UNWIND : l'enfant fois la longueur de la liste quand elle est connue à la
    // liaison (`UNWIND $uuids` des lectures d'outil : une à quelques dizaines) ; une liste calculée
    // ne dit rien, et la tâche garde alors tous ses fils (page coeur-cpp/12 §8). Le remetteur à jour
    // de l'optimiseur (CardinalityUpdater) refait le même calcul après les réécritures.
    const auto listSize = unwind->knownListSize();
    if (listSize.has_value() && plan.getLastOperator()->hasCardinalityEstimate()) {
        unwind->setCardinality(std::max<cardinality_t>(1, plan.getLastOperator()->getCardinality()) *
                               std::max<uint64_t>(1, *listSize));
    } else {
        unwind->clearCardinalityEstimate();
    }
    unwind->computeFactorizedSchema();
    plan.setLastOperator(unwind);
}

} // namespace planner
} // namespace rag3db
