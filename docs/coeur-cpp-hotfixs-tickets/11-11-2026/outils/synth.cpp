// Étude du 9 novembre 2026 (hotfixs) : le témoin des deux sens rebâti de zéro. Une table Scope de
// 82 890 nœuds, les 659 paires INHERITS_FROM de la fixture 729 (fichier « id|source|destination »),
// versées selon un mode, un CHECKPOINT, puis la relation 37654 -> 23480 et un CHECKPOINT.
// Usage : synth <dossier neuf> <paires> <mode : copy | lots:<n> | une>
#include "check_rel_directions.cpp"

#include <filesystem>
#include <fstream>
#include <sstream>

static void run(Connection& c, const std::string& q) {
    auto r = c.query(q);
    if (!r->isSuccess()) {
        std::cerr << "ERREUR " << q.substr(0, 200) << " : " << r->getErrorMessage() << "\n";
        exit(3);
    }
}

int main(int argc, char** argv) {
    if (argc < 4) return 2;
    const std::string dir = argv[1], mode = argv[3];
    std::vector<std::pair<uint64_t, uint64_t>> pairs;
    std::ifstream in(argv[2]);
    for (std::string l; std::getline(in, l);) {
        uint64_t id, a, b;
        if (sscanf(l.c_str(), "%lu|%lu|%lu", &id, &a, &b) == 3) pairs.push_back({a, b});
    }
    std::filesystem::create_directories(dir);
    SystemConfig config;
    Database db(dir + "/base.rag3db", config);
    Connection c(&db);
    run(c, "CREATE NODE TABLE Scope(id INT64, PRIMARY KEY(id))");
    run(c, "CREATE REL TABLE INHERITS_FROM(FROM Scope TO Scope, line INT64)");
    run(c, "UNWIND range(0, 82889) AS i CREATE (:Scope {id: i})");
    run(c, "CHECKPOINT");
    if (mode == "copy") {
        std::ofstream csv(dir + "/paires.csv");
        for (auto& [a, b] : pairs) csv << a << "," << b << ",1\n";
        csv.close();
        run(c, "COPY INHERITS_FROM FROM '" + dir + "/paires.csv'");
    } else {
        size_t lot = mode == "une" ? 1 : std::stoul(mode.substr(5));
        for (size_t i = 0; i < pairs.size(); i += lot) {
            std::string list;
            for (size_t j = i; j < std::min(i + lot, pairs.size()); j++) {
                list += (list.empty() ? "" : ", ") + std::string("[") + std::to_string(pairs[j].first) +
                        ", " + std::to_string(pairs[j].second) + "]";
            }
            run(c, "UNWIND [" + list + "] AS p MATCH (a:Scope {id: p[1]}), (b:Scope {id: p[2]}) CREATE (a)-[:INHERITS_FROM {line: 1}]->(b)");
        }
    }
    run(c, "CHECKPOINT");
    Connection reader(&db);
    std::cout << "avant : ";
    bool avant = check(reader, "INHERITS_FROM");
    run(c, "MATCH (a:Scope {id: 37654}), (b:Scope {id: 23480}) MERGE (a)-[r:INHERITS_FROM]->(b) SET r.line = 131");
    run(c, "CHECKPOINT");
    std::cout << "après : ";
    bool apres = check(reader, "INHERITS_FROM");
    std::cout << (apres && !avant ? "ROUGE : le point de reprise croise les deux sens\n" : "vert\n");
    return apres ? 1 : 0;
}
