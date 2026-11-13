#include "rag3db.hpp"
#include <cstdio>
int main(int, char** argv) {
    rag3db::main::SystemConfig config; rag3db::main::Database db(argv[1], config); rag3db::main::Connection c(&db);
    for (auto q : {"CREATE NODE TABLE Scope(_uuid STRING, key STRING, _snapshot STRING, PRIMARY KEY(_uuid))",
                   "CREATE NODE TABLE Scope_Chunk(_uuid STRING, _parent_uuid STRING, PRIMARY KEY(_uuid))",
                   "CREATE REL TABLE Scope_CHUNKED_FROM(FROM Scope_Chunk TO Scope)",
                   "CREATE (:Scope {_uuid: 'u1', key: 'file#a.rs#f:function', _snapshot: 's1'})",
                   "CREATE (:Scope {_uuid: 'u2', key: 'file#a.rs#f:function', _snapshot: 's2'})",
                   "CREATE (:Scope {_uuid: 'u3', key: 'file#a.rs#g:function', _snapshot: 's1'})",
                   "CREATE (:Scope_Chunk {_uuid: 'k1', _parent_uuid: 'u1'})",
                   "MATCH (k:Scope_Chunk {_uuid: 'k1'}), (s:Scope {_uuid: 'u1'}) CREATE (k)-[:Scope_CHUNKED_FROM]->(s)",
                   "CREATE (:Scope {_uuid: 'u1', key: 'x'})"}) {
        auto r = c.query(q); printf("%s : %s\n", q, r->isSuccess() ? "ok" : r->getErrorMessage().c_str());
    }
}
