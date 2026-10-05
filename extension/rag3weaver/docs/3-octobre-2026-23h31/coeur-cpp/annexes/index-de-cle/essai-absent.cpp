// Sur une copie d'une base de l'arbre principal : combien de clés de Symbol l'index de clé
// primaire ne retrouve-t-il pas, et où sont-elles ?
#include <iostream>
#include <string>
#include <vector>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    std::cout.setf(std::ios::unitbuf);
    SystemConfig config; config.readOnly = true;
    Database db(argv[1], config);
    Connection conn(&db);
    std::string table = argc > 2 ? argv[2] : "Symbol";
    auto r = conn.query("MATCH (n:" + table + ") RETURN n._uuid, offset(id(n)) ORDER BY offset(id(n));");
    if (!r->isSuccess()) { std::cout << "ERREUR " << r->getErrorMessage() << "\n"; return 2; }
    std::vector<std::pair<std::string, long>> rows;
    while (r->hasNext()) { auto t = r->getNext(); rows.push_back({t->getValue(0)->toString(), t->getValue(1)->getValue<int64_t>()}); }
    long missed = 0; std::string firsts;
    for (auto& [uuid, offset] : rows) {
        auto q = conn.query("MATCH (n:" + table + " {_uuid: '" + uuid + "'}) RETURN count(*);");
        if (!q->isSuccess() || q->getNext()->getValue(0)->getValue<int64_t>() != 1) {
            if (missed++ < 12) firsts += " " + std::to_string(offset) + ":" + uuid.substr(0, 8);
        }
    }
    std::cout << table << " : " << rows.size() << " lignes, " << missed << " clés introuvables par l'index ; premières (décalage:uuid) :" << firsts << "\n";
    return 0;
}
