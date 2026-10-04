// Étape 1 du chargement journalisé : ce que coûtent, à la taille du premier index de ce dépôt,
// l'écriture au journal des lignes sous la forme d'insertion (TABLE_INSERTION_RECORD) et leur
// rejeu. Mesuré par le chemin qui écrit déjà ces enregistrements — une insertion ordinaire —,
// puisqu'un COPY journalisé écrirait les mêmes et que le rejeu serait le même.
//   mesure ecrire <base>   : crée, insère, valide, et sort sans fermer (journal non vide)
//   mesure rouvrir <base>  : rouvre (rejeu), compte
#include <chrono>
#include <cstdlib>
#include <filesystem>
#include <iostream>
#include <string>
#include <unistd.h>
#include "rag3db.hpp"
using namespace rag3db::main;
using clk = std::chrono::steady_clock;
static double since(clk::time_point t) { return std::chrono::duration<double>(clk::now() - t).count(); }
static uint64_t walSize(const std::string& path) {
    std::error_code ec;
    auto n = std::filesystem::file_size(path + ".wal", ec);
    return ec ? 0 : n;
}
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    std::string mode = argv[1], path = argv[2];
    const long scopes = argc > 3 ? atol(argv[3]) : 79000;
    const long rels = argc > 4 ? atol(argv[4]) : 250000;
    SystemConfig config;
    config.autoCheckpoint = false;
    config.forceCheckpointOnClose = false;
    config.maxNumThreads = 8;
    if (mode == "rouvrir") {
        std::cout << "journal avant reouverture : " << walSize(path) / 1048576.0 << " Mio\n";
        auto t = clk::now();
        Database db(path, config);
        std::cout << "reouverture (rejeu) : " << since(t) << " s\n";
        Connection conn(&db);
        for (auto q : {"MATCH (n:Scope) RETURN count(*);", "MATCH (n:Chunk) RETURN count(*);",
                 "MATCH ()-[r:R]->() RETURN count(*);", "MATCH (n:Chunk {id: 4242}) RETURN n.content;"}) {
            auto r = conn.query(q);
            std::cout << "  " << q << " -> " << (r->isSuccess() && r->hasNext() ? r->getNext()->toString().substr(0, 60) : r->getErrorMessage() + "\n");
        }
        _exit(0);
    }
    std::filesystem::remove(path);
    std::filesystem::remove(path + ".wal");
    Database db(path, config);
    Connection conn(&db);
    auto run = [&](const std::string& what, const std::string& q) {
        auto t = clk::now();
        auto r = conn.query(q);
        if (!r->isSuccess()) { std::cout << "ERREUR " << what << " : " << r->getErrorMessage() << "\n"; _exit(1); }
        std::cout << what << " : " << since(t) << " s ; journal " << walSize(path) / 1048576.0 << " Mio\n";
    };
    run("schema", "CREATE NODE TABLE Scope(id INT64 PRIMARY KEY, name STRING, body STRING);");
    run("schema", "CREATE NODE TABLE Chunk(id INT64 PRIMARY KEY, content STRING, vec FLOAT[768]);");
    run("schema", "CREATE REL TABLE R(FROM Scope TO Scope, kind STRING);");
    const auto n = std::to_string(scopes - 1);
    run("BEGIN", "BEGIN TRANSACTION;");
    run("scopes, execution", "UNWIND range(0, " + n + ") AS i CREATE (:Scope {id: i, name: 'scope_' + CAST(i AS STRING), "
        "body: repeat('fn body line of code; ', 20) + CAST(i AS STRING)});");
    run("scopes, COMMIT", "COMMIT;");
    run("BEGIN", "BEGIN TRANSACTION;");
    run("morceaux et vecteurs, execution", "UNWIND range(0, " + n + ") AS i CREATE (:Chunk {id: i, content: "
        "repeat('fn body line of code; ', 20) + CAST(i AS STRING), vec: CAST(list_transform(range(1, 768), x -> CAST(x + i AS FLOAT)) AS FLOAT[768])});");
    run("morceaux et vecteurs, COMMIT", "COMMIT;");
    run("BEGIN", "BEGIN TRANSACTION;");
    run("relations, execution", "UNWIND range(0, " + std::to_string(rels - 1) + ") AS i WITH i % " + std::to_string(scopes) +
        " AS ka, (i * 7 + 1) % " + std::to_string(scopes) + " AS kb MATCH (a:Scope {id: ka}) WITH a, kb MATCH (b:Scope {id: kb}) "
        "CREATE (a)-[:R {kind: 'uses'}]->(b);");
    run("relations, COMMIT", "COMMIT;");
    std::cout << "fichier de la base : " << std::filesystem::file_size(path) / 1048576.0 << " Mio (aucun point de reprise)\n";
    _exit(0); // sans fermer : le journal reste, comme après un arrêt brutal
}
