#pragma once

#include "function/function.h"

namespace rag3db {
namespace function {

// CALL analyze('Table') : rebâtit les statistiques d'une table de nœuds.
struct AnalyzeFunction {
    static constexpr const char* name = "ANALYZE";

    static function_set getFunctionSet();
};

// CALL acquire_locks('Table', [clés]) : l'annonce des verrous en tête de transaction (marche V2).
struct AcquireLocksFunction {
    static constexpr const char* name = "ACQUIRE_LOCKS";
    static function_set getFunctionSet();
};

struct ClearWarningsFunction {
    static constexpr const char* name = "CLEAR_WARNINGS";

    static function_set getFunctionSet();
};

struct ProjectGraphNativeFunction {
    static constexpr const char* name = "PROJECT_GRAPH";

    static function_set getFunctionSet();
};

struct ProjectGraphCypherFunction {
    static constexpr const char* name = "PROJECT_GRAPH_CYPHER";

    static function_set getFunctionSet();
};

struct DropProjectedGraphFunction {
    static constexpr const char* name = "DROP_PROJECTED_GRAPH";

    static function_set getFunctionSet();
};

} // namespace function
} // namespace rag3db
