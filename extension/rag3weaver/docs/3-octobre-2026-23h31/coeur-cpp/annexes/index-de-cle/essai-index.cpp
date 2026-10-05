// Le point de reprise sans fin après un gros COPY annulé, sur le moteur d'AVANT le correctif.
//   essai-index <base> <lignes> full   : COPY annulé, COPY validé, deux CREATE, CHECKPOINT (ne finit pas si fautif)
//   essai-index <base> <lignes> make   : s'arrête après le COPY validé — laisse sur disque un index
//                                        de clé plus grand que son compte (une base « déjà abîmée »)
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <unistd.h>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    std::string path = argv[1]; long n = std::stol(argv[2]); std::string mode = argv[3];
    std::string csv = path + ".csv";
    std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
    { std::ofstream f(csv); for (long i = 0; i < n; i++) f << i << ",ligne " << i << "\n"; }
    SystemConfig config;
    Database db(path, config);
    Connection conn(&db);
    auto run = [&](const std::string& q) {
        auto r = conn.query(q);
        if (!r->isSuccess()) { std::cout << "ERREUR " << q.substr(0, 60) << " : " << r->getErrorMessage() << "\n"; _exit(3); }
        return r;
    };
    run("CALL auto_checkpoint=false;");
    run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);");
    run("BEGIN TRANSACTION;");
    run("COPY Doc FROM '" + csv + "';");
    run("ROLLBACK;");
    run("COPY Doc FROM '" + csv + "';");
    if (mode == "make") { std::cout << "base laissée après le COPY validé\n"; _exit(0); }
    run("CREATE (:Doc {id: 500000000, name: 'a'});");
    run("CREATE (:Doc {id: 500000001, name: 'b'});");
    run("CHECKPOINT;");
    std::cout << "CHECKPOINT fini : " << run("MATCH (d:Doc) RETURN count(*);")->getNext()->toString();
    _exit(0);
}
