#include "common/exception/binder.h"
#include "common/string_format.h"
#include "common/string_utils.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/standalone_call_function.h"
#include "function/table/table_function.h"
#include "graph/graph_entry_set.h"
#include "processor/execution_context.h"

using namespace rag3db::common;

namespace rag3db {
namespace function {

struct DropProjectedGraphBindData final : TableFuncBindData {
    std::string graphName;
    // skip_if_not_exists := true : un graphe absent n'est pas une erreur. Le produit le passe
    // avant chaque recherche dense filtrée ; l'option était ignorée sans un mot, et le DROP d'un
    // graphe absent échouait, ce qui condamne une transaction READ ONLY entière.
    bool skipIfNotExists;

    DropProjectedGraphBindData(std::string graphName, bool skipIfNotExists)
        : TableFuncBindData{0}, graphName{std::move(graphName)}, skipIfNotExists{skipIfNotExists} {}

    std::unique_ptr<TableFuncBindData> copy() const override {
        return std::make_unique<DropProjectedGraphBindData>(graphName, skipIfNotExists);
    }
};

static offset_t tableFunc(const TableFuncInput& input, TableFuncOutput&) {
    const auto bindData = ku_dynamic_cast<DropProjectedGraphBindData*>(input.bindData);
    auto graphEntrySet = graph::GraphEntrySet::Get(*input.context->clientContext);
    if (bindData->skipIfNotExists && !graphEntrySet->hasGraph(bindData->graphName)) {
        return 0;
    }
    graphEntrySet->validateGraphExist(bindData->graphName);
    graphEntrySet->dropGraph(bindData->graphName);
    return 0;
}

static std::unique_ptr<TableFuncBindData> bindFunc(const main::ClientContext*,
    const TableFuncBindInput* input) {
    auto graphName = input->getLiteralVal<std::string>(0 /* maxOffset */);
    bool skipIfNotExists = false;
    for (auto& [name, value] : input->optionalParams) {
        if (StringUtils::getLower(name) == "skip_if_not_exists") {
            value.validateType(LogicalTypeID::BOOL);
            skipIfNotExists = value.getValue<bool>();
        } else {
            throw BinderException{stringFormat("Unrecognized optional parameter {} in {}.", name,
                DropProjectedGraphFunction::name)};
        }
    }
    return std::make_unique<DropProjectedGraphBindData>(graphName, skipIfNotExists);
}

function_set DropProjectedGraphFunction::getFunctionSet() {
    function_set functionSet;
    auto func = std::make_unique<TableFunction>(name, std::vector{LogicalTypeID::STRING});
    func->bindFunc = bindFunc;
    func->tableFunc = tableFunc;
    func->initSharedStateFunc = TableFunction::initEmptySharedState;
    func->initLocalStateFunc = TableFunction::initEmptyLocalState;
    func->canParallelFunc = []() { return false; };
    // Un graphe projeté est un objet de la connexion, pas une écriture de la base.
    func->connectionScopedOnly = true;
    functionSet.push_back(std::move(func));
    return functionSet;
}

} // namespace function
} // namespace rag3db
