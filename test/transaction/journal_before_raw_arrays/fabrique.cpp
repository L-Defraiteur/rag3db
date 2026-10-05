// Fabrique une base et son journal non vide, écrits par le moteur d'AVANT la forme compacte des
// tableaux au journal (bibliothèque à 1177f5794), puis meurt base ouverte.
// Contenu : table Doc(id INT64 clé, name STRING, v FLOAT[4], w DOUBLE[2], n INT32[3]) ;
// insertions ordinaires, COPY journalisé, SET d'un vecteur, un vecteur NULL, un élément NULL.
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <unistd.h>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    std::string path = argv[1];
    std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
    SystemConfig config;
    auto db = new Database(path, config);
    auto conn = new Connection(db);
    auto run = [&](const std::string& q) {
        auto r = conn->query(q);
        if (!r->isSuccess()) { std::cout << "ERREUR " << q.substr(0, 90) << " : " << r->getErrorMessage() << "\n"; _exit(3); }
    };
    run("CALL auto_checkpoint=false;");
    run("CALL force_checkpoint_on_copy=false;");
    run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING, v FLOAT[4], w DOUBLE[2], n INT32[3]);");
    run("CHECKPOINT;");
    for (int i = 0; i < 10; i++) {
        std::string s = std::to_string(i);
        run("CREATE (:Doc {id: " + s + ", name: 'created " + s + "', v: [" + s + ".5, 0.25, -1.0, " + s + ".125], w: [" + s + ".000001, -2.5], n: [" + s + ", -7, 100000]});");
    }
    run("CREATE (:Doc {id: 100, name: 'null vector'});");
    run("CREATE (:Doc {id: 101, name: 'null element', v: [1.5, NULL, 3.5, 4.5], w: [NULL, 1.0], n: [1, 2, NULL]});");
    std::string csv = path + ".csv";
    { std::ofstream f(csv); for (int i = 1000; i < 1050; i++) f << i << ",copied " << i << ",\"[" << i << ".5,0.125,-3.0,7.75]\",\"[" << i << ".25,9.5]\",\"[" << i << ",2,3]\"\n"; }
    run("COPY Doc FROM '" + csv + "' (header=false);");
    run("MATCH (d:Doc {id: 3}) SET d.v = [9.5, 8.25, 7.125, 6.0];");
    run("MATCH (d:Doc {id: 1001}) SET d.w = [0.1, 0.2];");
    run("MATCH (d:Doc {id: 4}) SET d.v = NULL;");
    std::filesystem::remove(csv);
    std::cout << "journal " << std::filesystem::file_size(path + ".wal") << " octets, base " << std::filesystem::file_size(path) << " octets\n";
    _exit(0);
}
