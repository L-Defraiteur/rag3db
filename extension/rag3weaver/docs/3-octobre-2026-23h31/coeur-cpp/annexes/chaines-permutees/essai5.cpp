// L'étendue du défaut : la même recette (deux générations, une mise à jour, un point de
// reprise) par type de colonne, sur une table de relations et sur une table de nœuds.
// Chaque cas compare toutes les lignes avant et après le dernier point de reprise.
#include <filesystem>
#include <iostream>
#include <string>
#include <vector>
#include "rag3db.hpp"
using namespace rag3db::main;
struct Case { std::string name, type, oldV, newV, setV; bool rel; };
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    std::string base = argv[1];
    std::vector<Case> cases = {
        {"relation STRING[] à un élément", "STRING[]", "['vieux']", "['fichier']", "['nom']", true},
        {"relation STRING[] à deux éléments", "STRING[]", "['vieux','vieux']", "['fichier','vieux']", "['nom','fichier']", true},
        {"noeud STRING[] à un élément", "STRING[]", "['vieux']", "['fichier']", "['nom']", false},
        {"noeud STRING[] alterné", "STRING[]", "CASE WHEN i % 2 = 0 THEN ['b','a'] ELSE ['a','c','b'] END", "CASE WHEN i % 3 = 0 THEN ['c','a'] ELSE ['b'] END", "['a','b','c','a']", false},
    };
    for (auto& c : cases) {
        std::string path = base + ".cas";
        std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
        SystemConfig config;
        Database db(path, config);
        Connection conn(&db);
        bool failed = false;
        auto run = [&](const std::string& q) {
            auto r = conn.query(q);
            if (!r->isSuccess()) { std::cout << "   ERREUR " << q.substr(0, 70) << " : " << r->getErrorMessage() << "\n"; failed = true; }
        };
        auto dump = [&](const std::string& q) {
            std::vector<std::string> out; auto r = conn.query(q);
            if (!r->isSuccess()) { std::cout << "   ERREUR " << r->getErrorMessage() << "\n"; failed = true; return out; }
            while (r->hasNext()) out.push_back(r->getNext()->toString());
            return out;
        };
        std::string fwd, bwd;
        if (c.rel) {
            run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY);");
            run("CREATE REL TABLE M(FROM Doc TO Doc, res " + c.type + ", line INT64);");
            run("UNWIND range(0, 2399) AS i CREATE (:Doc {id: i});");
            run("UNWIND range(0, 1999) AS i MATCH (a:Doc {id: i}) WITH a, i MATCH (b:Doc {id: (i * 11) % 2400}) CREATE (a)-[:M {res: " + c.oldV + ", line: -1}]->(b);");
            run("CHECKPOINT;");
            run("UNWIND range(2400, 2599) AS i CREATE (:Doc {id: i});");
            run("UNWIND range(0, 2029) AS i MATCH (a:Doc {id: i % 2400}) WITH a, i MATCH (b:Doc {id: (i * 7 + 3) % 2400}) CREATE (a)-[:M {res: " + c.newV + ", line: i}]->(b);");
            run("UNWIND range(0, 139) AS i MATCH (a:Doc {id: 2400 + i}) WITH a, i MATCH (b:Doc {id: 2400 + (i * 3 + 1) % 60}) CREATE (a)-[:M {res: " + c.newV + ", line: 5000 + i}]->(b);");
            run("CHECKPOINT;");
            run("MATCH (a:Doc {id: 2400})-[r:M]->(b:Doc) SET r.res = " + c.setV + ";");
            fwd = "MATCH (a:Doc)-[r:M]->(b:Doc) RETURN a.id, b.id, r.line, r.res ORDER BY a.id, b.id, r.line;";
            bwd = "MATCH (b:Doc)<-[r:M]-(a:Doc) RETURN a.id, b.id, r.line, r.res ORDER BY a.id, b.id, r.line;";
        } else {
            // Nœuds : deux générations, puis des mises à jour éparses (une ligne sur 500).
            run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, res " + c.type + ");");
            run("UNWIND range(0, 199999) AS i CREATE (:Doc {id: i, res: " + c.oldV + "});");
            run("CHECKPOINT;");
            run("UNWIND range(200000, 399999) AS i CREATE (:Doc {id: i, res: " + c.newV + "});");
            run("CHECKPOINT;");
            run("MATCH (n:Doc) WHERE n.id % 500 = 7 SET n.res = " + c.setV + ";");
            fwd = bwd = "MATCH (n:Doc) RETURN n.id, n.res ORDER BY n.id;";
        }
        auto f0 = dump(fwd), b0 = dump(bwd);
        run("CHECKPOINT;");
        auto f1 = dump(fwd), b1 = dump(bwd);
        auto diff = [](const std::vector<std::string>& x, const std::vector<std::string>& y) {
            size_t n = x.size() == y.size() ? 0 : 1000000;
            for (size_t i = 0; i < std::min(x.size(), y.size()); i++) n += x[i] != y[i];
            return n;
        };
        std::cout << c.name << " : " << f0.size() << " lignes ; avant le point de reprise, écart entre sens " << diff(f0, b0)
                  << " ; après, sens direct changé " << diff(f0, f1) << ", sens inverse changé " << diff(b0, b1)
                  << (failed ? "  [ERREURS]" : "") << "\n";
    }
    std::filesystem::remove(base + ".cas"); std::filesystem::remove(base + ".cas.wal");
    return 0;
}
