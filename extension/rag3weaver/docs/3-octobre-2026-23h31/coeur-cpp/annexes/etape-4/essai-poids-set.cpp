// Ce qu'un SET d'un vecteur écrit au journal : octets par mise à jour.
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <unistd.h>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    std::string path = argv[1]; long rows = std::stol(argv[2]); long dim = std::stol(argv[3]);
    std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
    SystemConfig config;
    auto db = std::make_unique<Database>(path, config);
    auto conn = std::make_unique<Connection>(db.get());
    auto run = [&](const std::string& q) {
        auto r = conn->query(q);
        if (!r->isSuccess()) { std::cout << "ERREUR " << q.substr(0, 70) << " : " << r->getErrorMessage() << "\n"; _exit(3); }
    };
    run("CALL auto_checkpoint=false;");
    run("CREATE NODE TABLE T(id INT64 PRIMARY KEY, v FLOAT[" + std::to_string(dim) + "]);");
    run("UNWIND range(0, " + std::to_string(rows - 1) + ") AS i CREATE (:T {id: i});");
    run("CHECKPOINT;");
    auto size = [&] { std::error_code ec; auto s = std::filesystem::file_size(path + ".wal", ec); return ec ? 0ul : s; };
    auto before = size();
    for (long i = 0; i < rows; i++) {
        std::string v = "[";
        for (long d = 0; d < dim; d++) v += (d ? "," : "") + std::string("0.") + std::to_string(1000 + (i * 31 + d * 7) % 9000);
        run("MATCH (t:T {id: " + std::to_string(i) + "}) SET t.v = " + v + "];");
    }
    auto wal = size() - before;
    std::cout << rows << " SET d'un FLOAT[" << dim << "] : journal " << wal / 1024 << " Kio, soit " << wal / rows << " octets par mise à jour\n";
    _exit(0);
}
