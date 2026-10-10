// Fabrique une petite base avec le moteur d'AVANT la version 40 du stockage (son en-tête n'a pas
// d'étendue) : une table, un COPY, un point de reprise ; fermée proprement.
#include <filesystem>
#include <fstream>
#include <iostream>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int, char** argv) {
    std::string path = argv[1];
    std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
    { std::ofstream f(path + ".csv"); for (int i = 0; i < 500; i++) f << i << ",name " << i << "\n"; }
    { SystemConfig config; Database db(path, config); Connection c(&db);
      auto run = [&](const std::string& q) { auto r = c.query(q); if (!r->isSuccess()) { std::cout << r->getErrorMessage() << "\n"; return; } };
      run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);");
      run("UNWIND range(0, 499) AS i CREATE (:Doc {id: i, name: 'name ' + CAST(i AS STRING)});");
      run("CREATE (:Doc {id: 1000, name: 'created'});");
      run("CHECKPOINT;"); }
    std::filesystem::remove(path + ".csv");
    std::cout << "fichier " << std::filesystem::file_size(path) << " octets, journal " << (std::filesystem::exists(path + ".wal") ? std::filesystem::file_size(path + ".wal") : 0) << "\n";
    return 0;
}
