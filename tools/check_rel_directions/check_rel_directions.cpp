// Contrôle d'une base : chaque table de relations, lue par ses deux rangements, rend-elle les
// mêmes propriétés ?
//
// Une relation est rangée deux fois, une par sens. Jusqu'au correctif de la relecture partielle
// des chaînes (octobre 2026), un point de reprise pouvait échanger les chaînes de relations
// d'un seul des deux rangements : la même relation rend alors deux valeurs selon la forme de
// la requête. Ce programme lit chaque table par les deux formes et compare, relation par
// relation, propriété par propriété.
//
// Ce qu'il dit : un écart prouve que la table est atteinte. Il ne dit pas lequel des deux
// rangements est juste, et une table sans écart n'est pas prouvée saine (les deux rangements
// peuvent avoir été faussés au même endroit). Il ne voit pas non plus les listes de chaînes
// des tables de nœuds, atteintes par le même défaut : un nœud n'a qu'un rangement.
// Une base atteinte se réindexe.
//
// La base est ouverte en lecture seule ; aucun autre processus ne doit l'avoir ouverte en
// écriture. Bâtir, contre une bibliothèque du moteur et son en-tête unique :
//   g++ -std=c++20 -O2 -I <build>/src check_rel_directions.cpp -L <build>/src -lrag3db
// Usage : check_rel_directions <base>     — code de sortie 1 si un écart est trouvé.
#include <iostream>
#include <string>
#include <unordered_map>
#include <vector>

#include "rag3db.hpp"

using namespace rag3db::main;

namespace {

std::vector<std::vector<std::string>> rows(Connection& conn, const std::string& query) {
    std::vector<std::vector<std::string>> out;
    auto result = conn.query(query);
    if (!result->isSuccess()) {
        std::cerr << "ERREUR " << query << " : " << result->getErrorMessage() << "\n";
        exit(2);
    }
    while (result->hasNext()) {
        auto tuple = result->getNext();
        std::vector<std::string> row;
        for (auto i = 0u; i < tuple->len(); i++) {
            row.push_back(tuple->getValue(i)->toString());
        }
        out.push_back(std::move(row));
    }
    return out;
}

} // namespace

int main(int argc, char** argv) {
    if (argc < 2) {
        std::cerr << "usage : check_rel_directions <base>\n";
        return 2;
    }
    SystemConfig config;
    config.readOnly = true;
    Database db(argv[1], config);
    Connection conn(&db);

    bool anyGap = false;
    for (auto& table : rows(conn, "CALL show_tables() WHERE type = 'REL' RETURN name;")) {
        const auto& name = table[0];
        std::vector<std::string> properties;
        std::string projection = "id(r)";
        for (auto& property : rows(conn, "CALL table_info('" + name + "') RETURN name;")) {
            properties.push_back(property[0]);
            projection += ", r.`" + property[0] + "`";
        }
        std::unordered_map<std::string, std::vector<std::string>> forward;
        for (auto& row : rows(conn, "MATCH (a)-[r:`" + name + "`]->(b) RETURN " + projection + ";")) {
            forward.emplace(row[0], std::move(row));
        }
        const auto backward =
            rows(conn, "MATCH (b)<-[r:`" + name + "`]-(a) RETURN " + projection + ";");
        std::vector<uint64_t> gaps(properties.size(), 0);
        uint64_t missing = 0;
        for (auto& row : backward) {
            auto it = forward.find(row[0]);
            if (it == forward.end()) {
                missing++;
                continue;
            }
            for (auto i = 0u; i < properties.size(); i++) {
                gaps[i] += it->second[i + 1] != row[i + 1];
            }
        }
        bool tableGap = missing > 0 || forward.size() != backward.size();
        for (auto gap : gaps) {
            tableGap = tableGap || gap > 0;
        }
        anyGap = anyGap || tableGap;
        std::cout << name << " : " << forward.size() << " relations d'un côté, " << backward.size()
                  << " de l'autre" << (tableGap ? " — ÉCART" : " — égales") << "\n";
        if (missing > 0) {
            std::cout << "    " << missing << " relations d'un rangement absentes de l'autre\n";
        }
        for (auto i = 0u; i < properties.size(); i++) {
            if (gaps[i] > 0) {
                std::cout << "    " << properties[i] << " : " << gaps[i]
                          << " relations rendent deux valeurs\n";
            }
        }
    }
    std::cout << (anyGap ? "BASE ATTEINTE : au moins une table de relations diverge.\n" :
                           "Aucun écart entre les deux rangements.\n");
    return anyGap ? 1 : 0;
}
