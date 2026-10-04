// Dans une transaction : des nœuds créés par UNWIND … MERGE (certains existaient déjà), puis
// plus de 200 relations par COPY vers ces nœuds, les deux bouts pouvant être tout juste créés.
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    std::string path = argv[1], csv = path + ".rel.csv", ncsv = path + ".n.csv";
    std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
    // 300 nœuds déjà en base (id 0 à 299), validés par un COPY.
    { std::ofstream f(ncsv); for (int i = 0; i < 300; i++) f << i << ",ancien " << i << "\n"; }
    // 600 relations i -> i + 1 pour i dans [100, 700) : anciens->anciens, ancien->neuf, neufs->neufs.
    { std::ofstream f(csv); for (int i = 100; i < 700; i++) f << i << "," << i + 1 << "," << i << "\n"; }
    SystemConfig config;
    Database db(path, config);
    Connection conn(&db);
    auto run = [&](const std::string& q) {
        auto r = conn.query(q);
        std::cout << "> " << q.substr(0, 120) << "\n";
        if (!r->isSuccess()) { std::cout << "   ERREUR " << r->getErrorMessage() << "\n"; return; }
        int n = 0; while (r->hasNext() && n < 8) { std::cout << "   " << r->getNext()->toString(); n++; }
    };
    run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);");
    run("CREATE REL TABLE M(FROM Doc TO Doc, w INT64);");
    run("COPY Doc FROM '" + ncsv + "';");
    run("BEGIN TRANSACTION;");
    // Les nœuds 250 à 799 par MERGE : 250 à 299 existent, 300 à 799 sont créés dans la transaction.
    run("UNWIND range(250, 799) AS i MERGE (n:Doc {id: i}) ON CREATE SET n.name = 'neuf ' + CAST(i AS STRING);");
    run("COPY M FROM '" + csv + "';");
    auto checks = [&] {
        run("MATCH (n:Doc) RETURN count(*);");
        run("MATCH (a:Doc)-[r:M]->(b:Doc) RETURN count(r), min(a.id), max(b.id);");
        run("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE b.id <> a.id + 1 OR r.w <> a.id RETURN count(r);");
        run("MATCH (b:Doc)<-[r:M]-(a:Doc) RETURN count(r);");
        run("MATCH (b:Doc)<-[r:M]-(a:Doc) WHERE b.id <> a.id + 1 RETURN count(r);");
        run("MATCH (a:Doc {id: 150})-[r:M]->(b:Doc) RETURN a.id, a.name, r.w, b.id, b.name;");
        run("MATCH (a:Doc {id: 299})-[r:M]->(b:Doc) RETURN a.id, a.name, r.w, b.id, b.name;");
        run("MATCH (a:Doc {id: 500})-[r:M]->(b:Doc) RETURN a.id, a.name, r.w, b.id, b.name;");
        run("MATCH (b:Doc {id: 500})<-[r:M]-(a:Doc) RETURN a.id, a.name, r.w, b.id, b.name;");
        run("MATCH (a:Doc {id: 699})-[r:M]->(b:Doc) RETURN a.id, a.name, r.w, b.id, b.name;");
    };
    std::cout << "--- dans la transaction\n";
    checks();
    run("COMMIT;");
    std::cout << "--- après validation\n";
    checks();
    std::filesystem::remove(csv); std::filesystem::remove(ncsv);
    return 0;
}
