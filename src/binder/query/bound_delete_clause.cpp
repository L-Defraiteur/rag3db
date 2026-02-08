#include "binder/query/updating_clause/bound_delete_clause.h"

using namespace rag3db::common;

namespace rag3db {
namespace binder {

bool BoundDeleteClause::hasInfo(const std::function<bool(const BoundDeleteInfo&)>& check) const {
    for (auto& info : infos) {
        if (check(info)) {
            return true;
        }
    }
    return false;
}

std::vector<BoundDeleteInfo> BoundDeleteClause::getInfos(
    const std::function<bool(const BoundDeleteInfo&)>& check) const {
    std::vector<BoundDeleteInfo> result;
    for (auto& info : infos) {
        if (check(info)) {
            result.push_back(info.copy());
        }
    }
    return result;
}

} // namespace binder
} // namespace rag3db
