#include <iostream>
#include <string>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    SystemConfig config;
    Database db(argv[1], config);
    Connection conn(&db);
    for (int i = 2; i < argc; i++) {
        auto r = conn.query(argv[i]);
        std::cout << "> " << std::string(argv[i]).substr(0, 150) << "\n";
        if (!r->isSuccess()) { std::cout << "  ERREUR " << r->getErrorMessage() << "\n"; continue; }
        uint64_t n = 0;
        while (r->hasNext()) { auto row = r->getNext(); if (n < 12) std::cout << "  " << row->toString().substr(0, 220); n++; }
        std::cout << "  (" << n << " lignes)\n";
    }
}
