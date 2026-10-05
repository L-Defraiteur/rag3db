// Ce qu'un COPY journalisé écrit au journal : octets de journal par ligne, selon la forme de la
// ligne (une chaîne de L caractères, un vecteur FLOAT[D] ou non).
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <unistd.h>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    std::string path = argv[1]; long rows = std::stol(argv[2]); long textLen = std::stol(argv[3]); long dim = std::stol(argv[4]);
    std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
    SystemConfig config;
    auto db = std::make_unique<Database>(path, config);
    auto conn = std::make_unique<Connection>(db.get());
    auto run = [&](const std::string& q) {
        auto r = conn->query(q);
        if (!r->isSuccess()) { std::cout << "ERREUR " << q.substr(0, 70) << " : " << r->getErrorMessage() << "\n"; _exit(3); }
        return r;
    };
    run("CALL auto_checkpoint=false;");
    run("CALL force_checkpoint_on_copy=false;");
    run(std::string("CREATE NODE TABLE T(id STRING PRIMARY KEY, body STRING") + (dim ? ", v FLOAT[" + std::to_string(dim) + "]" : "") + ");");
    std::string csv = path + ".csv";
    unsigned long csvBytes = 0;
    {
        std::ofstream f(csv);
        for (long i = 0; i < rows; i++) {
            char id[64]; snprintf(id, sizeof id, "3f2b8c1e-0000-4000-8000-%012ld", i);
            f << id << "," << std::string(textLen, 'a' + (i % 26));
            if (dim) { f << ",\"["; for (long d = 0; d < dim; d++) { f << (d ? "," : "") << "0." << (1000 + (i * 31 + d * 7) % 9000); } f << "]\""; }
            f << "\n";
        }
    }
    csvBytes = std::filesystem::file_size(csv);
    auto walBefore = std::filesystem::exists(path + ".wal") ? std::filesystem::file_size(path + ".wal") : 0;
    run("COPY T FROM '" + csv + "' (header=false);");
    auto wal = std::filesystem::file_size(path + ".wal") - walBefore;
    std::cout << rows << " lignes, texte " << textLen << ", vecteur " << dim << " : CSV " << csvBytes / 1024 << " Kio, journal " << wal / 1024 << " Kio, soit " << wal / rows << " octets par ligne (x" << (double)wal / csvBytes << " du CSV)\n";
    std::filesystem::remove(csv);
    _exit(0);
}
