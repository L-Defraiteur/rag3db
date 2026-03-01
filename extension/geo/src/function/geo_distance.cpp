#include "function/geo_distance.h"

#include "common/types/types.h"
#include "common/vector/value_vector.h"
#include "function/scalar_function.h"
#include "math/geometry.h"

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// geo_distance(lat1 DOUBLE, lon1 DOUBLE, lat2 DOUBLE, lon2 DOUBLE) -> DOUBLE (meters)
static void geoDistanceExecFunc(
    const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void* /*dataPtr*/) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resultPos = (*resultSelVector)[i];

        auto p0 = (*parameterSelVectors[0])[parameters[0]->state->isFlat() ? 0 : i];
        auto p1 = (*parameterSelVectors[1])[parameters[1]->state->isFlat() ? 0 : i];
        auto p2 = (*parameterSelVectors[2])[parameters[2]->state->isFlat() ? 0 : i];
        auto p3 = (*parameterSelVectors[3])[parameters[3]->state->isFlat() ? 0 : i];

        bool isNull = parameters[0]->isNull(p0) || parameters[1]->isNull(p1) ||
                      parameters[2]->isNull(p2) || parameters[3]->isNull(p3);
        result.setNull(resultPos, isNull);
        if (!isNull) {
            double lat1 = parameters[0]->getValue<double>(p0);
            double lon1 = parameters[1]->getValue<double>(p1);
            double lat2 = parameters[2]->getValue<double>(p2);
            double lon2 = parameters[3]->getValue<double>(p3);
            result.setValue<double>(resultPos, math::haversine(lat1, lon1, lat2, lon2));
        }
    }
}

function_set GeoDistanceFunction::getFunctionSet() {
    function_set result;
    result.push_back(std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::DOUBLE, LogicalTypeID::DOUBLE,
            LogicalTypeID::DOUBLE, LogicalTypeID::DOUBLE},
        LogicalTypeID::DOUBLE, geoDistanceExecFunc));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
