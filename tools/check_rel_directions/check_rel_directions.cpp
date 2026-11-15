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
// **Les bouts se parcourent par tranches ancrées** (14 novembre 2026, relevé d'hotfixs) : l'outil
// tirait ses sources et ses destinations d'une seule lecture non ancrée, `MATCH (a)-[r:T]->(b)`,
// que le planificateur joue dans un seul sens ; une source perdue par ce sens n'était jamais
// ancrée, et sa relation n'était relue par aucun sens, même par celui qui l'avait encore (LOCKS de
// code-principal, lockIndexForWrite → acquireLock à la ligne 1173). Chaque nœud de chaque étiquette
// de bout (`show_connection`) est désormais ancré, par tranches de 500 offsets : le sens avant
// depuis les étiquettes sources, le sens arrière depuis les étiquettes destinations.
// `CHECK_REL_PLANS=1` imprime, pour chaque étiquette et chaque sens, le plan d'une tranche
// (EXPLAIN) : il prouve le sens joué.
//
// **Quatre comptes par table** : les relations lues au sens avant, au sens arrière (identifiants
// distincts), leur union, et celles perdues d'un seul sens. « égales » juge la cohérence des deux
// rangements, pas leur complétude ; ces comptes, comparés à ceux d'avant une opération, montrent ce
// qui manque.
//
// Usage : [CHECK_REL_PLANS=1] check_rel_directions <base> [table…]   — toutes les tables de
// relations sans argument ; code de sortie 1 si un écart est trouvé, 2 sur une erreur.
#include <cstdlib>
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
bool showPlans = false;

bool check(Connection& conn, const std::string& name) {
    std::vector<std::string> properties;
    std::string projection = "id(a), id(b)";
    for (auto& property : rows(conn, "CALL table_info('" + name + "') RETURN name;")) {
        properties.push_back(property[0]);
        projection += ", r.`" + property[0] + "`";
    }
    // Les étiquettes des bouts, puis chaque nœud de chacune, ancré par tranches de 500 offsets.
    std::set<std::string> fromLabels, toLabels;
    for (auto& row : rows(conn, "CALL show_connection('" + name + "') RETURN *;")) {
        fromLabels.insert(row[0]);
        toLabels.insert(row[1]);
    }
    constexpr uint64_t slice = 500;
    auto sliceQuery = [&](const std::string& label, bool forwardSide, uint64_t from) {
        const std::string node = forwardSide ? "a" : "b";
        return "MATCH (" + node + ":`" + label + "`) WHERE offset(id(" + node +
               ")) >= " + std::to_string(from) + " AND offset(id(" + node + ")) < " +
               std::to_string(from + slice) + " MATCH (a)-[r:`" + name + "`]->(b) RETURN id(r), " +
               projection + ";";
    };
    auto read = [&](const std::set<std::string>& labels, bool forwardSide,
                    std::multimap<std::string, Reading>& out) {
        for (auto& label : labels) {
            const std::string node = forwardSide ? "a" : "b";
            // Le compte d'abord : sur une étiquette vide, max(offset) est NULL (relevé d'hotfixs) ;
            // zéro tranche.
            auto top = rows(conn, "MATCH (" + node + ":`" + label +
                                      "`) RETURN count(*), max(offset(id(" + node + ")));");
            if (top.empty() || top[0][0] == "0") {
                continue;
            }
            if (showPlans) {
                std::cout << "    plan " << (forwardSide ? "avant" : "arrière") << " depuis "
                          << label << " :\n";
                for (auto& line : rows(conn, "EXPLAIN " + sliceQuery(label, forwardSide, 0))) {
                    std::cout << "      " << line[0] << "\n";
                }
            }
            const uint64_t end = std::stoull(top[0][1]) + 1;
            for (uint64_t from = 0; from < end; from += slice) {
                for (auto& row : rows(conn, sliceQuery(label, forwardSide, from))) {
                    out.emplace(row[0], Reading(row.begin() + 1, row.end()));
                }
            }
        }
    };
    std::multimap<std::string, Reading> forward, backward;
    read(fromLabels, true, forward);
    read(toLabels, false, backward);
    std::set<std::string> forwardIds, backwardIds, unionIds;
    for (auto& [rel, reading] : forward) {
        forwardIds.insert(rel);
        unionIds.insert(rel);
    }
    for (auto& [rel, reading] : backward) {
        backwardIds.insert(rel);
        unionIds.insert(rel);
    }
    uint64_t lost = 0;
    for (auto& rel : unionIds) {
        lost += forwardIds.count(rel) == 0 || backwardIds.count(rel) == 0;
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
    std::cout << name << " : " << fromLabels.size() << " étiquettes sources, " << toLabels.size()
              << " destinations ; lectures avant " << forward.size() << ", arrière "
              << backward.size() << (gap ? " — ÉCART" : " — égales") << "\n";
    std::cout << "    comptes : avant " << forwardIds.size() << ", arrière " << backwardIds.size()
              << ", union " << unionIds.size() << ", perdues d'un seul sens " << lost << "\n";
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
    if (const char* plans = std::getenv("CHECK_REL_PLANS"); plans && std::string(plans) == "1") {
        showPlans = true;
    }
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
