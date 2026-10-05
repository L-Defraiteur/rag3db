// Ce qu'un COPY de relations journalisé écrit au journal : octets par relation.
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <unistd.h>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    std::string path = argv[1]; long rels = std::stol(argv[2]); int props = std::stoi(argv[3]); std::string keyType = argv[4];
    std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
    SystemConfig config;
    auto db = std::make_unique<Database>(path, config);
    auto conn = std::make_unique<Connection>(db.get());
    auto run = [&](const std::string& q) {
        auto r = conn->query(q);
        if (!r->isSuccess()) { std::cout << "ERREUR " << q.substr(0, 70) << " : " << r->getErrorMessage() << "\n"; _exit(3); }
    };
    auto key = [&](long i) { char b[64]; snprintf(b, sizeof b, "3f2b8c1e-0000-4000-8000-%012ld", i); return keyType == "STRING" ? std::string(b) : std::to_string(i); };
    run("CALL auto_checkpoint=false;");
    run("CALL force_checkpoint_on_copy=false;");
    run("CREATE NODE TABLE T(id " + keyType + " PRIMARY KEY);");
    std::string relSchema = "CREATE REL TABLE R(FROM T TO T";
    for (int p = 0; p < props; p++) relSchema += ", p" + std::to_string(p) + " STRING";
    run(relSchema + ");");
    long nodes = 10000;
    std::string csv = path + ".csv";
    { std::ofstream f(csv); for (long i = 0; i < nodes; i++) f << key(i) << "\n"; }
    run("COPY T FROM '" + csv + "' (header=false);");
    run("CHECKPOINT;");
    { std::ofstream f(csv); for (long i = 0; i < rels; i++) { f << key(i % nodes) << "," << key((i * 7 + 3) % nodes); for (int p = 0; p < props; p++) f << ",propriete-" << p << "-" << i; f << "\n"; } }
    auto csvBytes = std::filesystem::file_size(csv);
    run("COPY R FROM '" + csv + "' (header=false);");
    auto wal = std::filesystem::file_size(path + ".wal");
    std::cout << rels << " relations, " << props << " propriétés, clé " << keyType << " : CSV " << csvBytes / 1024 << " Kio, journal " << wal / 1024 << " Kio, soit " << wal / rels << " octets par relation\n";
    std::filesystem::remove(csv);
    _exit(0);
}
