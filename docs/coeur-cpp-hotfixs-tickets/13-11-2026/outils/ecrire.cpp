#include <cstdio>
#include <iostream>
#include <string>
#include "rag3db.hpp"
int main(int argc, char** argv) {
    rag3db::main::SystemConfig config; config.readOnly = false;
    rag3db::main::Database db(argv[1], config); rag3db::main::Connection c(&db);
    for (int i = 2; i < argc; i++) {
        auto r = c.query(argv[i]); std::cout << "> " << argv[i] << "\n";
        if (!r->isSuccess()) { std::cout << "ERREUR " << r->getErrorMessage() << "\n"; return 1; }
        while (r->hasNext()) std::cout << r->getNext()->toString() << "\n";
    }
}
