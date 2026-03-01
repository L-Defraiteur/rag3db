#include "function/geo_within_bbox.h"

#include "common/types/types.h"
#include "common/vector/value_vector.h"
#include "function/scalar_function.h"

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// geo_within_bbox(lat, lon, min_lat, min_lon, max_lat, max_lon) -> BOOL
static void geoWithinBboxExecFunc(
    const std::vector<std::shared_ptr<ValueVector>>& parameters,
    const std::vector<SelectionVector*>& parameterSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void* /*dataPtr*/) {
    for (auto i = 0u; i < resultSelVector->getSelSize(); ++i) {
        auto resultPos = (*resultSelVector)[i];

        auto p0 = (*parameterSelVectors[0])[parameters[0]->state->isFlat() ? 0 : i];
        auto p1 = (*parameterSelVectors[1])[parameters[1]->state->isFlat() ? 0 : i];
        auto p2 = (*parameterSelVectors[2])[parameters[2]->state->isFlat() ? 0 : i];
        auto p3 = (*parameterSelVectors[3])[parameters[3]->state->isFlat() ? 0 : i];
        auto p4 = (*parameterSelVectors[4])[parameters[4]->state->isFlat() ? 0 : i];
        auto p5 = (*parameterSelVectors[5])[parameters[5]->state->isFlat() ? 0 : i];

        bool isNull = parameters[0]->isNull(p0) || parameters[1]->isNull(p1) ||
                      parameters[2]->isNull(p2) || parameters[3]->isNull(p3) ||
                      parameters[4]->isNull(p4) || parameters[5]->isNull(p5);
        result.setNull(resultPos, isNull);
        if (!isNull) {
            double lat = parameters[0]->getValue<double>(p0);
            double lon = parameters[1]->getValue<double>(p1);
            double minLat = parameters[2]->getValue<double>(p2);
            double minLon = parameters[3]->getValue<double>(p3);
            double maxLat = parameters[4]->getValue<double>(p4);
            double maxLon = parameters[5]->getValue<double>(p5);
            result.setValue<bool>(resultPos,
                lat >= minLat && lat <= maxLat && lon >= minLon && lon <= maxLon);
        }
    }
}

function_set GeoWithinBboxFunction::getFunctionSet() {
    function_set result;
    result.push_back(std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::DOUBLE, LogicalTypeID::DOUBLE,
            LogicalTypeID::DOUBLE, LogicalTypeID::DOUBLE, LogicalTypeID::DOUBLE,
            LogicalTypeID::DOUBLE},
        LogicalTypeID::BOOL, geoWithinBboxExecFunc));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
