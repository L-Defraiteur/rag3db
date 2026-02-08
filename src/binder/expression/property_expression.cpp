#include "binder/expression/property_expression.h"

using namespace rag3db::common;
using namespace rag3db::catalog;

namespace rag3db {
namespace binder {

bool PropertyExpression::isPrimaryKey() const {
    for (auto& [id, info] : infos) {
        if (!info.isPrimaryKey) {
            return false;
        }
    }
    return true;
}

bool PropertyExpression::isPrimaryKey(table_id_t tableID) const {
    if (!infos.contains(tableID)) {
        return false;
    }
    return infos.at(tableID).isPrimaryKey;
}

bool PropertyExpression::hasProperty(table_id_t tableID) const {
    KU_ASSERT(infos.contains(tableID));
    return infos.at(tableID).exists;
}

} // namespace binder
} // namespace rag3db
