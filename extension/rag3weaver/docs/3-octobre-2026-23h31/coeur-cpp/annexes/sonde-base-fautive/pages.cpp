// Toutes les plages de pages des colonnes de toutes les tables : table, colonne, début, nombre.
#include <iostream>
#include <string>
#include <vector>
#include "rag3db.hpp"
using namespace rag3db::main;
int main(int argc, char** argv) {
    SystemConfig config;
    Database db(argv[1], config);
    Connection conn(&db);
    std::vector<std::string> tables;
    auto r = conn.query("CALL show_tables() RETURN name;");
    while (r->hasNext()) tables.push_back(r->getNext()->getValue(0)->getValue<std::string>());
    for (auto& t : tables) {
        auto s = conn.query("CALL storage_info('" + t + "') RETURN column_name, start_page_idx, num_pages, num_values, compression, node_group_id;");
        if (!s->isSuccess()) { std::cout << "ERREUR\t" << t << "\t" << s->getErrorMessage() << "\n"; continue; }
        while (s->hasNext()) {
            auto row = s->getNext();
            std::cout << t;
            for (int i = 0; i < 6; i++) std::cout << "\t" << row->getValue(i)->toString();
            std::cout << "\n";
        }
    }
}
