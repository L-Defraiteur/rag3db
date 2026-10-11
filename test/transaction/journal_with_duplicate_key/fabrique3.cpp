// Troisième fixture (relecture d'hotfixs, cas 1) : comme la seconde, puis une transaction D
// supprime la première clé 7 (celle de A, v = 0). À la fin du rejeu, la ligne de B (v = 1) est la
// seule clé 7 vivante : elle doit être gardée, avec son entrée d'index — pas de pierre tombale.
// Écrite par le moteur d'AVANT le verrou de clé (ff9bad960), sous le mode multi-écrivains :
//   A : CREATE 1, CREATE 7, COMMIT ; B (ouverte avant le COMMIT de A) : CREATE 7, CREATE 8, COMMIT ;
//   C : CREATE 9, SET 9.v = 99, SET 1.v = 11, (1)-[:Link {w: 1}]->(9), COMMIT ;
//   D : MATCH (n:Item {id: 7}) WHERE n.v = 0 DELETE n, COMMIT ; puis mort base ouverte.
// Bâti : g++ -std=c++20 fabrique3.cpp -I<dossier de rag3db.hpp> -L<dossier de librag3db.so> -lrag3db
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
    run(first, "CHECKPOINT;");
    run(first, "BEGIN TRANSACTION;");
    run(second, "BEGIN TRANSACTION;");
    run(first, "CREATE (:Item {id: 1, v: 0});");
    run(first, "CREATE (:Item {id: 7, v: 0});");
    run(second, "CREATE (:Item {id: 7, v: 1});");
    run(second, "CREATE (:Item {id: 8, v: 0});");
    run(first, "COMMIT;");
    run(second, "COMMIT;");
    run(first, "BEGIN TRANSACTION;");
    run(first, "CREATE (:Item {id: 9, v: 0});");
    run(first, "MATCH (n:Item {id: 9}) SET n.v = 99;");
    run(first, "MATCH (n:Item {id: 1}) SET n.v = 11;");
    run(first, "MATCH (a:Item {id: 1}), (b:Item {id: 9}) CREATE (a)-[:Link {w: 1}]->(b);");
    run(first, "COMMIT;");
    // D : la clé 7 de A (v = 0) est supprimée ; celle de B (v = 1) doit survivre au rejeu.
    run(first, "BEGIN TRANSACTION;");
    run(first, "MATCH (n:Item {id: 7}) WHERE n.v = 0 DELETE n;");
    run(first, "COMMIT;");
    std::cout << "journal " << std::filesystem::file_size(path + ".wal") << " octets, base " << std::filesystem::file_size(path) << " octets\n";
    kill(getpid(), SIGKILL);
}
