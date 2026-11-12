// Contrôle d'une base : chaque table de relations, lue par ses deux rangements, rend-elle les
// mêmes relations, avec les mêmes bouts et les mêmes propriétés ?
//
// Une relation est rangée deux fois, une par sens. Deux défauts ont déjà fait diverger les
// rangements. Jusqu'au correctif de la relecture partielle des chaînes (octobre 2026), un point
// de reprise pouvait échanger les chaînes de relations d'un seul des deux rangements. Et dans les
// tables LOCKS et QUERIES de la base du dépôt de code (relevé du 25 octobre 2026, ticket
// 2026-10-25-une-relation-locks-trouvee-d-un-seul-cote-fait-echouer-le-drain-de-la-montre), des
// listes CSR rendaient des lignes vides (relation 0 vers le nœud 0), des relations d'un seul côté,
// et des relations rattachées au mauvais nœud.
//
// **Chaque sens est lu nœud par nœud**, par une requête ancrée sur un bout : le sens avant depuis
// chaque source, le sens arrière depuis chaque destination. Deux formes sans ancre,
// `(a)-[r]->(b)` et `(b)<-[r]-(a)`, ne suffisent pas : le planificateur les joue dans le même
// sens. Sur la base abîmée du 25 octobre, elles rendaient les mêmes identifiants de relation ;
// seul le compte des lignes, doublons compris, trahissait l'écart, sans désigner une relation.
//
// Les deux lectures sont comparées relation par relation : présente d'un seul côté, vue deux fois
// dans un sens, bouts différents, propriété différente. Un écart prouve que la table est atteinte,
// sans dire lequel des deux rangements est juste. Une table sans écart n'est pas prouvée saine pour
// autant : une relation perdue des deux côtés ne se voit pas. Les listes de chaînes des tables de
// nœuds ne sont pas couvertes : un nœud n'a qu'un rangement. Une base atteinte se réindexe.
//
// La base est ouverte en lecture seule ; aucun autre processus ne doit l'avoir ouverte en
// écriture. Travailler sur une copie (reflink), jamais sur une base en service. Bâtir, contre une
// bibliothèque du moteur et son en-tête unique :
//   g++ -std=c++20 -O2 -I <build>/src check_rel_directions.cpp -L <build>/src -lrag3db
// Usage : check_rel_directions <base> [table…]   — toutes les tables de relations sans argument ;
// code de sortie 1 si un écart est trouvé, 2 sur une erreur.
#include <iostream>
#include <map>
#include <set>
#include <string>
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

// Une relation lue dans un sens : ses bouts, puis ses propriétés.
using Reading = std::vector<std::string>;

// Le relevé d'une table ; vrai si elle diverge.
bool check(Connection& conn, const std::string& name) {
    std::vector<std::string> properties;
    std::string projection = "id(a), id(b)";
    for (auto& property : rows(conn, "CALL table_info('" + name + "') RETURN name;")) {
        properties.push_back(property[0]);
        projection += ", r.`" + property[0] + "`";
    }
    // Les bouts : chaque source et chaque destination, avec son étiquette.
    std::set<std::pair<std::string, std::string>> sources, destinations;
    for (auto& row : rows(conn, "MATCH (a)-[r:`" + name +
                                    "`]->(b) RETURN label(a), "
                                    "offset(id(a)), label(b), offset(id(b));")) {
        sources.insert({row[0], row[1]});
        destinations.insert({row[2], row[3]});
    }
    std::multimap<std::string, Reading> forward, backward;
    for (auto& [label, offset] : sources) {
        for (auto& row : rows(conn, "MATCH (a:`" + label + "`)-[r:`" + name +
                                        "`]->(b) WHERE offset(id(a)) = " + offset +
                                        " RETURN id(r), " + projection + ";")) {
            forward.emplace(row[0], Reading(row.begin() + 1, row.end()));
        }
    }
    for (auto& [label, offset] : destinations) {
        for (auto& row : rows(conn, "MATCH (a)-[r:`" + name + "`]->(b:`" + label +
                                        "`) WHERE offset(id(b)) = " + offset + " RETURN id(r), " +
                                        projection + ";")) {
            backward.emplace(row[0], Reading(row.begin() + 1, row.end()));
        }
    }
    uint64_t oneSide = 0, twice = 0, ends = 0;
    std::vector<uint64_t> gaps(properties.size(), 0);
    std::vector<std::string> examples;
    auto example = [&](const std::string& what, const std::string& rel, const Reading& reading) {
        if (examples.size() < 10) {
            examples.push_back(
                what + " : relation " + rel + ", " + reading[0] + " -> " + reading[1]);
        }
    };
    for (auto& [rel, reading] : forward) {
        if (forward.count(rel) > 1) {
            twice++;
            example("vue deux fois au sens avant", rel, reading);
            continue;
        }
        const auto other = backward.find(rel);
        if (other == backward.end()) {
            oneSide++;
            example("au seul sens avant", rel, reading);
            continue;
        }
        if (backward.count(rel) > 1) {
            continue; // compté par le sens arrière
        }
        if (other->second[0] != reading[0] || other->second[1] != reading[1]) {
            ends++;
            example("bouts différents (sens avant)", rel, reading);
            continue;
        }
        for (auto i = 0u; i < properties.size(); i++) {
            gaps[i] += other->second[i + 2] != reading[i + 2];
        }
    }
    for (auto& [rel, reading] : backward) {
        if (backward.count(rel) > 1) {
            twice++;
            example("vue deux fois au sens arrière", rel, reading);
        } else if (forward.count(rel) == 0) {
            oneSide++;
            example("au seul sens arrière", rel, reading);
        }
    }
    bool gap = oneSide > 0 || twice > 0 || ends > 0;
    for (auto g : gaps) {
        gap = gap || g > 0;
    }
    std::cout << name << " : " << sources.size() << " sources, " << destinations.size()
              << " destinations ; sens avant " << forward.size() << ", sens arrière "
              << backward.size() << (gap ? " — ÉCART" : " — égales") << "\n";
    if (oneSide > 0) {
        std::cout << "    " << oneSide << " relations d'un seul côté\n";
    }
    if (twice > 0) {
        std::cout << "    " << twice << " relations vues deux fois dans un sens\n";
    }
    if (ends > 0) {
        std::cout << "    " << ends << " relations aux bouts différents selon le sens\n";
    }
    for (auto i = 0u; i < properties.size(); i++) {
        if (gaps[i] > 0) {
            std::cout << "    " << properties[i] << " : " << gaps[i]
                      << " relations rendent deux valeurs\n";
        }
    }
    for (auto& line : examples) {
        std::cout << "      " << line << "\n";
    }
    return gap;
}

} // namespace

int main(int argc, char** argv) {
    if (argc < 2) {
        std::cerr << "usage : check_rel_directions <base> [table…]\n";
        return 2;
    }
    SystemConfig config;
    config.readOnly = true;
    Database db(argv[1], config);
    Connection conn(&db);
    std::vector<std::string> tables(argv + 2, argv + argc);
    if (tables.empty()) {
        for (auto& row : rows(conn, "CALL show_tables() WHERE type = 'REL' RETURN name;")) {
            tables.push_back(row[0]);
        }
    }
    bool anyGap = false;
    for (auto& name : tables) {
        anyGap = check(conn, name) || anyGap;
    }
    std::cout << (anyGap ? "BASE ATTEINTE : au moins une table de relations diverge.\n" :
                           "Aucun écart entre les deux rangements.\n");
    return anyGap ? 1 : 0;
}
