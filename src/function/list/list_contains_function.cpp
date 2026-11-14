#include <string>
#include <string_view>
#include <type_traits>
#include <unordered_set>

#include "binder/expression/expression_util.h"
#include "common/exception/binder.h"
#include "common/type_utils.h"
#include "function/list/functions/list_position_function.h"
#include "function/list/vector_list_functions.h"
#include "function/scalar_function.h"

using namespace rag3db::common;
using namespace rag3db::binder;

namespace rag3db {
namespace function {

struct ListContains {
    template<typename T>
    static void operation(common::list_entry_t& list, T& element, uint8_t& result,
        common::ValueVector& listVector, common::ValueVector& elementVector,
        common::ValueVector& resultVector) {
        int64_t pos = 0;
        ListPosition::operation(list, element, pos, listVector, elementVector, resultVector);
        result = (pos != 0);
    }
};

// **Un IN sur une liste constante ne parcourt plus la liste à chaque ligne** (ticket
// docs/everything-declarative/tickets/2026-11-14-un-in-sur-la-cle-primaire-balaie-toute-la-table.md).
// `n.k IN $l` se planifie en balayage puis LIST_CONTAINS($l, n.k) : avec un parcours de la liste par
// ligne, N clés sur N lignes coûtaient N². Quand la liste est un littéral ou un paramètre, elle ne
// change pas pendant la requête : l'évaluateur (qui a sa propre copie de cette donnée liée) en bâtit
// un ensemble haché au premier appel, puis chaque ligne y est cherchée en O(1). Les entiers et les
// chaînes seulement, dont l'égalité est celle des octets ; les autres types, et les petites
// listes, gardent le parcours.
struct ListContainsBindData final : FunctionBindData {
    static constexpr uint64_t MIN_LIST_SIZE_FOR_SET = 16;

    bool constantList;
    // L'ensemble bâti, et la liste d'où il vient : un autre vecteur ou une autre entrée le rebâtit.
    bool built = false;
    const ValueVector* builtFrom = nullptr;
    uint64_t builtOffset = 0;
    uint64_t builtSize = 0;
    std::unordered_set<int64_t> integers;
    struct StringHash {
        using is_transparent = void;
        size_t operator()(std::string_view view) const {
            return std::hash<std::string_view>{}(view);
        }
    };
    std::unordered_set<std::string, StringHash, std::equal_to<>> strings;

    ListContainsBindData(std::vector<LogicalType> paramTypes, bool constantList)
        : FunctionBindData{std::move(paramTypes), LogicalType::BOOL()},
          constantList{constantList} {}

    std::unique_ptr<FunctionBindData> copy() const override {
        // L'ensemble ne se copie pas : chaque évaluateur bâtit le sien.
        return std::make_unique<ListContainsBindData>(LogicalType::copy(paramTypes), constantList);
    }

    template<typename T>
    static constexpr bool hashable() {
        return std::is_same_v<T, ku_string_t> ||
               (std::is_integral_v<T> && sizeof(T) <= sizeof(int64_t));
    }

    template<typename T>
    void build(const list_entry_t& list, ValueVector& listVector) {
        integers.clear();
        strings.clear();
        auto values = reinterpret_cast<T*>(ListVector::getListValues(&listVector, list));
        for (auto i = 0u; i < list.size; i++) {
            // Comme ListPosition : la valeur brute de chaque élément, sans regarder son masque nul.
            if constexpr (std::is_same_v<T, ku_string_t>) {
                strings.emplace(values[i].getAsStringView());
            } else {
                integers.insert(static_cast<int64_t>(values[i]));
            }
        }
        built = true;
        builtFrom = &listVector;
        builtOffset = list.offset;
        builtSize = list.size;
    }

    template<typename T>
    bool contains(const T& element) const {
        if constexpr (std::is_same_v<T, ku_string_t>) {
            return strings.find(element.getAsStringView()) != strings.end();
        } else {
            return integers.contains(static_cast<int64_t>(element));
        }
    }
};

struct ListContainsWithSet {
    template<typename T>
    static void operation(list_entry_t& list, T& element, uint8_t& result, ValueVector& listVector,
        ValueVector& elementVector, ValueVector& resultVector, void* dataPtr) {
        auto* bindData = static_cast<ListContainsBindData*>(dataPtr);
        if constexpr (ListContainsBindData::hashable<T>()) {
            if (bindData && bindData->constantList &&
                list.size >= ListContainsBindData::MIN_LIST_SIZE_FOR_SET &&
                ListType::getChildType(listVector.dataType) == elementVector.dataType) {
                if (!bindData->built || bindData->builtFrom != &listVector ||
                    bindData->builtOffset != list.offset || bindData->builtSize != list.size) {
                    bindData->build<T>(list, listVector);
                }
                result = bindData->contains(element);
                return;
            }
        }
        ListContains::operation(list, element, result, listVector, elementVector, resultVector);
    }
};

struct ListContainsWrapper {
    template<typename LEFT_TYPE, typename RIGHT_TYPE, typename RESULT_TYPE, typename OP>
    static void operation(LEFT_TYPE& left, RIGHT_TYPE& right, RESULT_TYPE& result,
        ValueVector* leftValueVector, ValueVector* rightValueVector,
        ValueVector* resultValueVector, uint64_t /*resultPos*/, void* dataPtr) {
        OP::operation(left, right, result, *leftValueVector, *rightValueVector, *resultValueVector,
            dataPtr);
    }
};

template<typename T>
static void execListContains(const std::vector<std::shared_ptr<ValueVector>>& params,
    const std::vector<SelectionVector*>& paramSelVectors, ValueVector& result,
    SelectionVector* resultSelVector, void* dataPtr) {
    KU_ASSERT(params.size() == 2);
    BinaryFunctionExecutor::executeSwitch<list_entry_t, T, uint8_t, ListContainsWithSet,
        ListContainsWrapper>(*params[0], paramSelVectors[0], *params[1], paramSelVectors[1],
        result, resultSelVector, dataPtr);
}

static std::unique_ptr<FunctionBindData> bindFunc(const ScalarBindFuncInput& input) {
    auto scalarFunction = input.definition->ptrCast<ScalarFunction>();
    // for list_contains(list, input), we expect input and list child have the same type, if list
    // is empty, we use in the input type. Otherwise, we use list child type because casting list
    // is more expensive.
    std::vector<LogicalType> paramTypes;
    LogicalType childType;
    auto listExpr = input.arguments[0];
    auto elementExpr = input.arguments[1];
    if (ExpressionUtil::isEmptyList(*listExpr)) {
        childType = elementExpr->getDataType().copy();
    } else {
        auto& listChildType = ListType::getChildType(listExpr->getDataType());
        auto& elementType = elementExpr->getDataType();
        if (!LogicalTypeUtils::tryGetMaxLogicalType(listChildType, elementType, childType)) {
            throw BinderException(
                stringFormat("Cannot compare {} and {} in list_contains function.",
                    listChildType.toString(), elementType.toString()));
        }
    }
    if (childType.getLogicalTypeID() == LogicalTypeID::ANY) {
        childType = LogicalType::STRING();
    }
    auto listType = LogicalType::LIST(childType.copy());
    paramTypes.push_back(listType.copy());
    paramTypes.push_back(childType.copy());
    TypeUtils::visit(childType.getPhysicalType(),
        [&scalarFunction]<typename T>(T) { scalarFunction->execFunc = execListContains<T>; });
    // Un littéral ou un paramètre : la liste est la même pour toute la requête.
    const auto constantList = listExpr->expressionType == ExpressionType::LITERAL ||
                              listExpr->expressionType == ExpressionType::PARAMETER;
    return std::make_unique<ListContainsBindData>(std::move(paramTypes), constantList);
}

function_set ListContainsFunction::getFunctionSet() {
    function_set result;
    auto function = std::make_unique<ScalarFunction>(name,
        std::vector<LogicalTypeID>{LogicalTypeID::LIST, LogicalTypeID::ANY}, LogicalTypeID::BOOL);
    function->bindFunc = bindFunc;
    result.push_back(std::move(function));
    return result;
}

} // namespace function
} // namespace rag3db
