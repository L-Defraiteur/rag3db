#include "function/geo_contains.h"

#include "common/types/types.h"
#include "common/vector/value_vector.h"
#include "function/scalar_function.h"
#include "math/geometry.h"

namespace rag3db {
namespace geo_extension {

using namespace function;
using namespace common;

// geo_contains(lat DOUBLE, lon DOUBLE, ring_lats LIST(DOUBLE), ring_lons LIST(DOUBLE)) -> BOOL
static void geoContainsExecFunc(
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
        if (isNull) continue;

        double lat = parameters[0]->getValue<double>(p0);
        double lon = parameters[1]->getValue<double>(p1);

        auto latList = parameters[2]->getValue<list_entry_t>(p2);
        auto lonList = parameters[3]->getValue<list_entry_t>(p3);
        auto* latData = ListVector::getDataVector(parameters[2].get());
        auto* lonData = ListVector::getDataVector(parameters[3].get());

        uint32_t ringSize = std::min(latList.size, lonList.size);
        if (ringSize < 3) {
            result.setValue<bool>(resultPos, false);
            continue;
        }

        // Extract ring coordinates into contiguous arrays.
        std::vector<double> ringLats(ringSize);
        std::vector<double> ringLons(ringSize);
        for (uint32_t k = 0; k < ringSize; k++) {
            ringLats[k] = latData->getValue<double>(latList.offset + k);
            ringLons[k] = lonData->getValue<double>(lonList.offset + k);
        }

        result.setValue<bool>(resultPos,
            math::pointInPolygon(lat, lon, ringLats.data(), ringLons.data(), ringSize));
    }
}

static std::unique_ptr<FunctionBindData> geoContainsBindFunc(const ScalarBindFuncInput& input) {
    auto resultType = LogicalType::BOOL();
    return FunctionBindData::getSimpleBindData(input.arguments, std::move(resultType));
}

function_set GeoContainsFunction::getFunctionSet() {
    function_set result;
    auto func = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::DOUBLE, LogicalTypeID::DOUBLE,
            LogicalTypeID::LIST, LogicalTypeID::LIST},
        LogicalTypeID::BOOL, geoContainsExecFunc);
    func->bindFunc = geoContainsBindFunc;
    result.push_back(std::move(func));
    return result;
}

} // namespace geo_extension
} // namespace rag3db
