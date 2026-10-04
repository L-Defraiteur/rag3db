// Rouvre une base laissée par une passe fautive, dans un processus neuf, et relit tout.
#include <iostream>
#include <string>
#include <vector>

#include "rag3db.hpp"

using namespace rag3db::main;

int main(int argc, char** argv) {
    std::string path = argv[1];
    std::string ext = argv[2];
    SystemConfig config;
    Database db(path, config);
    Connection conn(&db);
    auto run = [&](const std::string& q, bool print) {
        auto r = conn.query(q);
        if (!r->isSuccess()) {
            std::cout << "ERREUR  " << q.substr(0, 110) << "\n        " << r->getErrorMessage() << "\n";
            return std::string();
        }
        std::string first;
        uint64_t n = 0;
        while (r->hasNext()) {
            auto row = r->getNext();
            if (n == 0) first = row->toString();
            if (print) std::cout << "   " << row->toString();
            n++;
        }
        if (!print) std::cout << "ok " << n << " lignes  " << q.substr(0, 110) << "\n";
        return first;
    };
    run("LOAD EXTENSION '" + ext + "';", false);
    std::vector<std::string> tables;
    {
        auto r = conn.query("CALL show_tables() RETURN name, type;");
        while (r->hasNext()) {
            auto row = r->getNext();
            auto name = row->getValue(0)->getValue<std::string>();
            auto type = row->getValue(1)->getValue<std::string>();
            if (type == "NODE") tables.push_back(name);
            else if (type == "REL") tables.push_back("-" + name);
        }
    }
    for (auto& t : tables) {
        if (t[0] == '-') {
            run("MATCH ()-[r:`" + t.substr(1) + "`]->() RETURN r;", false);
        } else {
            run("MATCH (n:`" + t + "`) RETURN n;", false);
        }
    }
    std::cout << "index :\n";
    run("CALL SHOW_INDEXES() RETURN *;", true);
    std::cout << "\nScope_Chunk : count, max offset\n";
    run("MATCH (n:Scope_Chunk) RETURN count(*), max(offset(id(n)));", true);
    std::cout << "\n";
    return 0;
}
