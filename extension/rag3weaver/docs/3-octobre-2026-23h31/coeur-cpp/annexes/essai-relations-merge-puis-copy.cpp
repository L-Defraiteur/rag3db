// Dans une transaction : un petit lot de relations par UNWIND … MERGE, puis un gros par COPY
// dans la même table de relations. Que lit-on avant et après la validation ?
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    std::string path = argv[1], csv = path + ".rel.csv", ncsv = path + ".n.csv";
    const bool nodesByCopyInTx = argc > 2;
    std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
    { std::ofstream f(ncsv); for (int i = 0; i < 1000; i++) f << i << ",n" << i << "\n"; }
    // Le gros lot : i -> (i + 1) pour i dans [100, 900), poids i.
    { std::ofstream f(csv); for (int i = 100; i < 900; i++) f << i << "," << i + 1 << "," << i << "\n"; }
    SystemConfig config;
    Database db(path, config);
    Connection conn(&db);
    auto run = [&](const std::string& q) {
        auto r = conn.query(q);
        std::cout << "> " << q.substr(0, 110) << "\n";
        if (!r->isSuccess()) { std::cout << "   ERREUR " << r->getErrorMessage() << "\n"; return; }
        int n = 0; while (r->hasNext() && n < 8) { std::cout << "   " << r->getNext()->toString(); n++; }
    };
    run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);");
    run("CREATE REL TABLE M(FROM Doc TO Doc, w INT64);");
    if (!nodesByCopyInTx) run("COPY Doc FROM '" + ncsv + "';");
    run("BEGIN TRANSACTION;");
    if (nodesByCopyInTx) run("COPY Doc FROM '" + ncsv + "';");
    // Le petit lot : i -> (i + 1) pour i dans [0, 20), poids 10000 + i.
    run("UNWIND range(0, 19) AS i MATCH (a:Doc {id: i}) WITH a, i MATCH (b:Doc {id: i + 1}) MERGE (a)-[r:M]->(b) ON CREATE SET r.w = 10000 + i;");
    run("MATCH (a:Doc)-[r:M]->(b:Doc) RETURN count(r), min(r.w), max(r.w);");
    run("COPY M FROM '" + csv + "';");
    auto checks = [&] {
        run("MATCH (a:Doc)-[r:M]->(b:Doc) RETURN count(r), min(r.w), max(r.w);");
        run("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE b.id <> a.id + 1 RETURN count(r);");
        run("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE (a.id < 20 AND r.w <> 10000 + a.id) OR (a.id >= 100 AND r.w <> a.id) RETURN count(r);");
        run("MATCH (b:Doc)<-[r:M]-(a:Doc) RETURN count(r);");
        run("MATCH (a:Doc {id: 5})-[r:M]->(b:Doc) RETURN a.id, r.w, b.id;");
        run("MATCH (a:Doc {id: 500})-[r:M]->(b:Doc) RETURN a.id, r.w, b.id;");
        run("MATCH (b:Doc {id: 6})<-[r:M]-(a:Doc) RETURN a.id, r.w, b.id;");
    };
    std::cout << "--- dans la transaction\n";
    checks();
    run("MATCH (a:Doc {id: 5})-[r:M]->(b:Doc) SET r.w = 77777;");
    run("MATCH (a:Doc {id: 500})-[r:M]->(b:Doc) SET r.w = 88888;");
    run("COMMIT;");
    std::cout << "--- après validation\n";
    run("MATCH (a:Doc)-[r:M]->(b:Doc) RETURN count(r);");
    run("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE b.id <> a.id + 1 RETURN count(r);");
    run("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE r.w IN [77777, 88888] RETURN a.id, r.w, b.id ORDER BY a.id;");
    run("MATCH (b:Doc)<-[r:M]-(a:Doc) RETURN count(r);");
    std::filesystem::remove(csv); std::filesystem::remove(ncsv);
    return 0;
}
