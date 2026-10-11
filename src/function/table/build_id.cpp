#include "binder/binder.h"
#include "common/build_id.h"
#include "function/table/bind_data.h"
#include "function/table/bind_input.h"
#include "function/table/simple_table_function.h"

using namespace rag3db::common;
using namespace rag3db::main;

namespace rag3db {
namespace function {

// L'identifiant de bâti du moteur (cmake/build_id.cmake), celui que LOAD EXTENSION compare à celui
// de chaque extension : « rag3db-build-<commit>[-dirty-<diff>]-<options> ». Pour qu'un hôte (le
// démon de rag3weaver) dise de quel bâti il sert, sans lire le binaire.
static offset_t internalTableFunc(const TableFuncMorsel& /*morsel*/,
    const TableFuncInput& /*input*/, DataChunk& output) {
    auto& outputVector = output.getValueVectorMutable(0);
    auto pos = output.state->getSelVector()[0];
    outputVector.setValue(pos, std::string(RAG3DB_BUILD_ID));
    return 1;
}

static std::unique_ptr<TableFuncBindData> bindFunc(const ClientContext*,
    const TableFuncBindInput* input) {
    std::vector<std::string> returnColumnNames;
    std::vector<LogicalType> returnTypes;
    returnColumnNames.emplace_back("build_id");
    returnTypes.emplace_back(LogicalType::STRING());
    returnColumnNames =
        TableFunction::extractYieldVariables(returnColumnNames, input->yieldVariables);
    auto columns = input->binder->createVariables(returnColumnNames, returnTypes);
    return std::make_unique<TableFuncBindData>(std::move(columns), 1 /* one row result */);
}

function_set BuildIdFunction::getFunctionSet() {
    function_set functionSet;
    auto function = std::make_unique<TableFunction>(name, std::vector<LogicalTypeID>{});
    function->tableFunc = SimpleTableFunc::getTableFunc(internalTableFunc);
    function->bindFunc = bindFunc;
    function->initSharedStateFunc = SimpleTableFunc::initSharedState;
    function->initLocalStateFunc = TableFunction::initEmptyLocalState;
    functionSet.push_back(std::move(function));
    return functionSet;
}

} // namespace function
} // namespace rag3db
