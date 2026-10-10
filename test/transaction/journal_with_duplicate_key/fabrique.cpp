// Fabrique une base et son journal non vide où DEUX transactions ont validé la même clé
// primaire, puis meurt base ouverte. Écrite par le moteur d'AVANT le verrou de clé (marche A3′,
// bibliothèque à ff9bad960), sous le mode multi-écrivains (debug_enable_multi_writes), qui
// laissait naître ce doublon : deux connexions, chacune sa transaction, chacune CREATE
// (:Item {id: 7}), chacune COMMIT. Depuis A3′ le second CREATE attend le premier et reçoit
// l'erreur de clé en double : ce journal ne peut plus se fabriquer, il est gardé ici pour le
// témoin LockBench.RecoveryOfAJournalWithADuplicateKeyKeepsTheDatabaseOpen.
// Bâti : g++ -std=c++20 fabrique.cpp -I<dossier de rag3db.hpp> -L<dossier de librag3db.so> -lrag3db
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
    run(first, "CHECKPOINT;");
    run(first, "BEGIN TRANSACTION;");
    run(second, "BEGIN TRANSACTION;");
    run(first, "CREATE (:Item {id: 7, v: 0});");
    run(second, "CREATE (:Item {id: 7, v: 1});");
    run(first, "COMMIT;");
    run(second, "COMMIT;");
    std::cout << "journal " << std::filesystem::file_size(path + ".wal") << " octets, base " << std::filesystem::file_size(path) << " octets\n";
    kill(getpid(), SIGKILL);
}
