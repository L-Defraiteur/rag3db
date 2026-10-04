// Les deux tests du banc non commités (ListOfStringsAcrossCheckpoints), en programme à part :
// mode « noeuds » (400 000 lignes à listes alternées par lots de 50 000, une mise à jour sur
// 500, un point de reprise, relecture, réouverture) et mode « relations » (listes réécrites
// en place entre deux points de reprise).
#include <filesystem>
#include <iostream>
#include <string>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    std::string path = argv[1], mode = argv[2];
    uint64_t pool = argc > 3 ? std::stoull(argv[3]) : -1u;
    std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
    SystemConfig config; config.bufferPoolSize = pool;
    auto db = std::make_unique<Database>(path, config);
    auto conn = std::make_unique<Connection>(db.get());
    auto run = [&](const std::string& q) {
        std::cout << "> " << q.substr(0, 110) << "\n";
        auto r = conn->query(q);
        if (!r->isSuccess()) std::cout << "   ERREUR : " << r->getErrorMessage() << "\n";
    };
    auto dump = [&](const std::string& q) {
        std::string out; auto r = conn->query(q);
        if (!r->isSuccess()) { std::cout << "   ERREUR : " << r->getErrorMessage() << "\n"; return out; }
        while (r->hasNext()) out += r->getNext()->toString();
        return out;
    };
    if (mode == "noeuds") {
        run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, tags STRING[]);");
        for (auto first = 0; first < 400000; first += 50000)
            run("UNWIND range(" + std::to_string(first) + ", " + std::to_string(first + 49999) + ") AS i CREATE (:Doc {id: i, tags: CASE WHEN i % 2 = 0 THEN ['vieux', 'vieux'] ELSE ['fichier', 'vieux'] END});");
        run("CHECKPOINT;");
        run("MATCH (d:Doc) WHERE d.id % 500 = 0 SET d.tags = ['nom'];");
        const std::string q = "MATCH (d:Doc) RETURN d.id, d.tags ORDER BY d.id;";
        auto before = dump(q);
        run("CHECKPOINT;");
        std::cout << "après le point de reprise : " << (before == dump(q) ? "égal" : "DIFFÉRENT") << "\n";
        conn.reset(); db.reset();
        db = std::make_unique<Database>(path, config); conn = std::make_unique<Connection>(db.get());
        std::cout << "après la réouverture : " << (before == dump(q) ? "égal" : "DIFFÉRENT") << "\n";
    } else {
        run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, tags STRING[]);");
        run("CREATE REL TABLE M(FROM Doc TO Doc, tags STRING[]);");
        run("UNWIND range(0, 2399) AS i CREATE (:Doc {id: i, tags: ['vieux', 'vieux']});");
        run("UNWIND range(0, 1999) AS i MATCH (a:Doc {id: i}), (b:Doc {id: (i * 11) % 2400}) CREATE (a)-[:M {tags: ['vieux', 'vieux']}]->(b);");
        run("CHECKPOINT;");
        run("MATCH (d:Doc) WHERE d.id % 3 = 0 SET d.tags = ['fichier', 'nom', 'fichier'];");
        run("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE a.id % 3 = 0 SET r.tags = ['fichier', 'nom', 'fichier'];");
        run("CHECKPOINT;");
        run("MATCH (d:Doc) WHERE d.id % 7 = 0 SET d.tags = ['nom'];");
        run("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE a.id % 7 = 0 SET r.tags = ['nom'];");
        const std::string n = "MATCH (d:Doc) RETURN d.id, d.tags ORDER BY d.id;";
        const std::string f = "MATCH (a:Doc)-[r:M]->(b:Doc) RETURN a.id, b.id, r.tags ORDER BY a.id, b.id;";
        const std::string b = "MATCH (b:Doc)<-[r:M]-(a:Doc) RETURN a.id, b.id, r.tags ORDER BY a.id, b.id;";
        auto n0 = dump(n), f0 = dump(f), b0 = dump(b);
        run("CHECKPOINT;");
        std::cout << "nœuds " << (n0 == dump(n) ? "égal" : "DIFFÉRENT") << " ; une forme " << (f0 == dump(f) ? "égal" : "DIFFÉRENT") << " ; l'autre " << (b0 == dump(b) ? "égal" : "DIFFÉRENT") << "\n";
    }
    std::cout << "FIN\n";
    return 0;
}
