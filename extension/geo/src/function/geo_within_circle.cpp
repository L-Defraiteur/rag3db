#include "function/geo_within_circle.h"

#include "common/vector/value_vector.h"
#include "function/scalar_function.h"
#include "math/geometry.h"

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// GEO_WITHIN_CIRCLE(lat, lon, center_lat, center_lon, radius_m) → BOOL
static void execFunc(const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void*) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resPos = (*resultSelVector)[i];
        auto p0 = (*parameterSelVectors[0])[parameters[0]->state->isFlat() ? 0 : i];
        auto p1 = (*parameterSelVectors[1])[parameters[1]->state->isFlat() ? 0 : i];
        auto p2 = (*parameterSelVectors[2])[parameters[2]->state->isFlat() ? 0 : i];
        auto p3 = (*parameterSelVectors[3])[parameters[3]->state->isFlat() ? 0 : i];
        auto p4 = (*parameterSelVectors[4])[parameters[4]->state->isFlat() ? 0 : i];

        bool isNull = parameters[0]->isNull(p0) || parameters[1]->isNull(p1) ||
                      parameters[2]->isNull(p2) || parameters[3]->isNull(p3) ||
                      parameters[4]->isNull(p4);
        result.setNull(resPos, isNull);
        if (isNull) continue;

        double lat = parameters[0]->getValue<double>(p0);
        double lon = parameters[1]->getValue<double>(p1);
        double cLat = parameters[2]->getValue<double>(p2);
        double cLon = parameters[3]->getValue<double>(p3);
        double radius = parameters[4]->getValue<double>(p4);

        result.setValue<bool>(resPos, math::haversine(lat, lon, cLat, cLon) <= radius);
    }
}

function_set GeoWithinCircleFunction::getFunctionSet() {
    function_set result;
    result.push_back(std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::DOUBLE, LogicalTypeID::DOUBLE,
            LogicalTypeID::DOUBLE, LogicalTypeID::DOUBLE, LogicalTypeID::DOUBLE},
        LogicalTypeID::BOOL, execFunc));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
