// Dans une transaction : vingt insertions ordinaires, puis un COPY dans la même table, puis
// des lectures et écritures par clé. Que lit-on, qu'écrit-on ?
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    std::string path = argv[1], csv = path + ".csv";
    std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
    { std::ofstream f(csv); for (int i = 0; i < 100; i++) f << i << ",copie " << i << "\n"; }
    SystemConfig config;
    Database db(path, config);
    Connection conn(&db);
    auto run = [&](const std::string& q) {
        auto r = conn.query(q);
        std::cout << "> " << q.substr(0, 90) << "\n";
        if (!r->isSuccess()) { std::cout << "   ERREUR " << r->getErrorMessage() << "\n"; return; }
        int n = 0; while (r->hasNext() && n < 6) { std::cout << "   " << r->getNext()->toString(); n++; }
    };
    run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);");
    run("CREATE REL TABLE L(FROM Doc TO Doc);");
    run("BEGIN TRANSACTION;");
    run("UNWIND range(1000, 1019) AS i CREATE (:Doc {id: i, name: 'locale ' + CAST(i AS STRING)});");
    run("COPY Doc FROM '" + csv + "';");
    run("MATCH (n:Doc {id: 5}) RETURN n.id, n.name;");
    run("MATCH (n:Doc {id: 1005}) RETURN n.id, n.name;");
    run("MATCH (n:Doc {id: 5}) SET n.name = 'MODIFIEE PAR LA CLE 5';");
    run("MATCH (a:Doc {id: 7}), (b:Doc {id: 8}) CREATE (a)-[:L]->(b);");
    run("COMMIT;");
    run("MATCH (n:Doc) RETURN count(*);");
    run("MATCH (n:Doc) WHERE n.name STARTS WITH 'MODIFIEE' RETURN n.id, n.name;");
    run("MATCH (n:Doc {id: 5}) RETURN n.id, n.name;");
    run("MATCH (a:Doc)-[:L]->(b:Doc) RETURN a.id, b.id;");
    std::filesystem::remove(csv);
    return 0;
}
