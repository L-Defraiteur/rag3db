// Quatrième fixture (relecture d'hotfixs, 11 octobre 2026) : la clé en doublon porte des relations,
// deux dans chaque sens, avant la pierre tombale. Écrite par le moteur d'AVANT le verrou de clé
// (ff9bad960), sous le mode multi-écrivains :
//   A : CREATE 1, 2, 7, COMMIT ; B (ouverte avant le COMMIT de A) : CREATE 7 (le doublon), puis
//   (7b)->(1), (7b)->(2), (1)->(7b), (2)->(7b) — le MATCH de B ne voit que son propre 7 —, COMMIT ;
//   puis mort base ouverte. Au rejeu, la pierre tombale doit retirer les quatre relations, des deux
//   côtés, et laisser 1, 2 et le 7 de A, sans relation.
// Bâti : g++ -std=c++20 fabrique4.cpp -I<dossier de rag3db.hpp> -L<dossier de librag3db.so> -lrag3db
#include <filesystem>
#include <iostream>
#include <string>
#include <signal.h>
#include <unistd.h>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    std::string path = argv[1];
    std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
    SystemConfig config;
    auto db = new Database(path, config);
    auto first = new Connection(db);
    auto second = new Connection(db);
    auto run = [&](Connection* conn, const std::string& q) {
        auto r = conn->query(q);
        if (!r->isSuccess()) { std::cout << "ERREUR " << q << " : " << r->getErrorMessage() << "\n"; _exit(3); }
    };
    run(first, "CALL debug_enable_multi_writes=true;");
    run(first, "CALL auto_checkpoint=false;");
    run(first, "CREATE NODE TABLE Item(id INT64 PRIMARY KEY, v INT64);");
    run(first, "CREATE REL TABLE Link(FROM Item TO Item, w INT64);");
    run(first, "CREATE (:Item {id: 1, v: 0});");
    run(first, "CREATE (:Item {id: 2, v: 0});");
    run(first, "CHECKPOINT;");
    run(first, "BEGIN TRANSACTION;");
    run(second, "BEGIN TRANSACTION;");
    run(first, "CREATE (:Item {id: 7, v: 0});");
    run(second, "CREATE (:Item {id: 7, v: 1});");
    run(second, "MATCH (a:Item {id: 7}), (b:Item {id: 1}) CREATE (a)-[:Link {w: 71}]->(b);");
    run(second, "MATCH (a:Item {id: 7}), (b:Item {id: 2}) CREATE (a)-[:Link {w: 72}]->(b);");
    run(second, "MATCH (a:Item {id: 1}), (b:Item {id: 7}) CREATE (a)-[:Link {w: 17}]->(b);");
    run(second, "MATCH (a:Item {id: 2}), (b:Item {id: 7}) CREATE (a)-[:Link {w: 27}]->(b);");
    run(first, "COMMIT;");
    run(second, "COMMIT;");
    std::cout << "journal " << std::filesystem::file_size(path + ".wal") << " octets, base " << std::filesystem::file_size(path) << " octets\n";
    kill(getpid(), SIGKILL);
}
