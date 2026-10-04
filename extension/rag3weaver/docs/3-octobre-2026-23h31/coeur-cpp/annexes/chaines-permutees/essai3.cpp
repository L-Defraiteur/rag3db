// La séquence de la base fautive : des nœuds neufs créés par MERGE (validés, sans point de
// reprise), puis un COPY de 2 170 relations dont 140 entre ces nœuds neufs (res='fichier'),
// puis un petit lot UNWIND … MERGE … SET res='nom' sur ces 140 paires.
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    std::string path = argv[1], mode = argc > 2 ? argv[2] : "base";
    std::string csv = path + ".rel.csv";
    std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
    {   // 2 030 relations entre nœuds anciens, puis 140 entre nœuds neufs (plusieurs par nœud d'arrivée).
        std::ofstream f(csv);
        for (int i = 0; i < 2030; i++) f << i % 2400 << "," << (i * 7 + 3) % 2400 << ",fichier," << i << "\n";
        for (int i = 0; i < 140; i++) f << 2400 + i << "," << 2400 + (i * 3 + 1) % 60 << ",fichier," << 5000 + i << "\n";
    }
    SystemConfig config;
    Database db(path, config);
    Connection conn(&db);
    auto run = [&](const std::string& q, bool quiet = false) {
        auto r = conn.query(q);
        if (!quiet) std::cout << "> " << q.substr(0, 140) << "\n";
        if (!r->isSuccess()) { std::cout << "   ERREUR " << q.substr(0, 60) << " : " << r->getErrorMessage() << "\n"; return; }
        int n = 0; while (!quiet && r->hasNext() && n < 6) { std::cout << "   " << r->getNext()->toString(); n++; }
    };
    run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);", true);
    run("CREATE REL TABLE M(FROM Doc TO Doc, res STRING, line INT64);", true);
    run("UNWIND range(0, 2399) AS i CREATE (:Doc {id: i, name: 'ancien'});", true);
    if (mode != "sans-anciennes") {
        run("UNWIND range(0, 1999) AS i MATCH (a:Doc {id: i}) WITH a, i MATCH (b:Doc {id: (i * 11) % 2400}) CREATE (a)-[:M {res: 'vieux', line: -1}]->(b);", true);
    }
    run("CHECKPOINT;", true);
    // Les nœuds neufs, par MERGE, en plusieurs instructions validées, sans point de reprise.
    for (int k = 2400; k < 2600; k += 50)
        run("UNWIND range(" + std::to_string(k) + ", " + std::to_string(k + 49) + ") AS i MERGE (n:Doc {id: i}) ON CREATE SET n.name = 'neuf';", true);
    if (mode == "sans-copy" || mode == "minimal") {
        run("UNWIND range(0, 2029) AS i MATCH (a:Doc {id: i % 2400}) WITH a, i MATCH (b:Doc {id: (i * 7 + 3) % 2400}) CREATE (a)-[:M {res: 'fichier', line: i}]->(b);", true);
        run("UNWIND range(0, 139) AS i MATCH (a:Doc {id: 2400 + i}) WITH a, i MATCH (b:Doc {id: 2400 + (i * 3 + 1) % 60}) CREATE (a)-[:M {res: 'fichier', line: 5000 + i}]->(b);", true);
        run("CHECKPOINT;");
    } else run("COPY M FROM '" + csv + "';");
    auto checks = [&] {
        run("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE r.line >= 0 RETURN r.res, count(*) ORDER BY r.res;");
        run("MATCH (b:Doc)<-[r:M]-(a:Doc) WHERE r.line >= 0 RETURN r.res, count(*) ORDER BY r.res;");
    };
    checks();
    std::string pair = "MATCH (a:Doc {id: 2400 + i}) WITH a, i MATCH (b:Doc {id: 2400 + (i * 3 + 1) % 60})";
    if (mode == "sans-set") {} else if (mode == "petits") for (int k = 0; k < 140; k += 20) run("UNWIND range(" + std::to_string(k) + ", " + std::to_string(k + 19) + ") AS i " + pair + " MERGE (a)-[r:M]->(b) SET r.res = 'nom';", true);
    else if (mode == "minimal") run("MATCH (a:Doc {id: 2400})-[r:M]->(b:Doc) SET r.res = 'nom';");
    else run("UNWIND range(0, 139) AS i " + pair + " MERGE (a)-[r:M]->(b) SET r.res = 'nom';");
    checks();
    run("CHECKPOINT;", true);
    checks();
    run("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE r.line >= 0 AND r.res = 'vieux' RETURN a.id, b.id, r.line, offset(id(r)) ORDER BY a.id LIMIT 6;");
    run("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE r.line >= 0 AND r.res = 'vieux' RETURN min(a.id), max(a.id), min(r.line), max(r.line), count(*);");
    run("MATCH (a:Doc)-[r:M]->(b:Doc) RETURN count(r), count(DISTINCT id(r));");
    run("MATCH (b:Doc)<-[r:M]-(a:Doc) RETURN count(r), count(DISTINCT id(r));");
    run("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE r.line < 0 RETURN r.res, count(*) ORDER BY r.res;");
    run("MATCH (b:Doc)<-[r:M]-(a:Doc) WHERE r.line < 0 RETURN r.res, count(*) ORDER BY r.res;");
    std::filesystem::remove(csv);
    return 0;
}
