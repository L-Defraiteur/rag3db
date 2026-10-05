// Les clés réelles d'une base de l'arbre principal, relues dans l'ordre des décalages, puis
// rechargées dans une table neuve du moteur nu par K COPY validés ; chaque clé cherchée ensuite.
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <vector>
#include <unistd.h>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    std::string source = argv[1], path = argv[2]; int copies = std::stoi(argv[3]);
    std::string type = argc > 4 ? argv[4] : "STRING";
    std::string table = argc > 5 ? argv[5] : "Symbol";
    std::vector<std::string> keys;
    {
        SystemConfig config; config.readOnly = true;
        Database db(source, config); Connection conn(&db);
        auto r = conn.query("MATCH (n:" + table + ") RETURN n._uuid ORDER BY offset(id(n));");
        if (!r->isSuccess()) { std::cout << "ERREUR " << r->getErrorMessage() << "\n"; return 2; }
        while (r->hasNext()) keys.push_back(r->getNext()->getValue(0)->toString());
    }
    std::filesystem::remove(path); std::filesystem::remove(path + ".wal");
    SystemConfig config;
    auto db = std::make_unique<Database>(path, config);
    auto conn = std::make_unique<Connection>(db.get());
    auto run = [&](const std::string& q) {
        auto r = conn->query(q);
        if (!r->isSuccess()) { std::cout << "ERREUR " << q.substr(0, 70) << " : " << r->getErrorMessage() << "\n"; _exit(3); }
        return r;
    };
    run("CREATE NODE TABLE Symbol(_uuid " + type + " PRIMARY KEY, name STRING);");
    size_t n = keys.size() / copies;
    for (int c = 0; c < copies; c++) {
        std::string csv = path + ".csv";
        { std::ofstream f(csv); for (size_t i = c * n; i < (c + 1 == copies ? keys.size() : (c + 1) * n); i++) f << keys[i] << ",f" << i << "\n"; }
        run("COPY Symbol FROM '" + csv + "';");
    }
    auto count = [&](const std::string& when) {
        long missed = 0; std::string firsts;
        for (size_t i = 0; i < keys.size(); i++) {
            auto r = conn->query("MATCH (s:Symbol {_uuid: '" + keys[i] + "'}) RETURN count(*);");
            if (!r->isSuccess() || r->getNext()->getValue(0)->getValue<int64_t>() != 1) { if (missed++ < 8) firsts += " " + std::to_string(i) + ":" + keys[i].substr(0, 8); }
        }
        std::cout << when << " : " << run("MATCH (s:Symbol) RETURN count(*);")->getNext()->toString()
                  << "   clés introuvables : " << missed << firsts << "\n";
    };
    std::cout << keys.size() << " clés relues, " << copies << " COPY, clé " << type << "\n";
    count("dans la session");
    conn.reset(); db.reset();
    db = std::make_unique<Database>(path, config); conn = std::make_unique<Connection>(db.get());
    count("après réouverture");
    _exit(0);
}
