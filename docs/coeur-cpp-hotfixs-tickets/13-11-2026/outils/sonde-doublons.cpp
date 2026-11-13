// Sonde des doublons (13 novembre 2026, hotfixs, à la demande de c7 et du cœur) : sur une COPIE ouverte
// en lecture seule, les nœuds qui partagent la même clé primaire ou la même clé logique. Le cœur a
// prouvé (PkLookupUnderCheckpoint) qu'une recherche par clé primaire pouvait ne pas trouver une clé
// validée pendant un point de reprise V2 : un MERGE a pu créer une seconde rangée de même clé.
//
// Par table de nœuds T à clé primaire :
//   - doublons de clé : MATCH (n:T) WITH n.<pk> AS k, count(*) AS c WHERE c > 1 … (agrégation sur les
//     rangées, sans passer par l'index de clé) ; le nombre de clés en double et de rangées en trop ;
//   - pour les dix premières : le décalage de chaque rangée, son _snapshot et son _absent_since s'ils
//     existent, ce que rend une recherche par la clé (l'index), et pour chaque rangée le nombre de
//     morceaux qui la rattachent par <T>_CHUNKED_FROM et parmi eux ceux qui portent un vecteur.
// Clés logiques (information : un doublon y peut être légitime) : Scope.key, File.path, Symbol.name
// par _project, Library.import_path par _project.
//
// Le moteur refuse d'écrire une seconde rangée de même clé primaire (« duplicated primary key ») : un
// doublon venu du défaut naîtrait côté rag3weaver, une lecture qui rate la clé puis un CREATE sous un
// autre uuid. C'est le doublon de clé logique, le chiffre décisif ; il compte dans le code de sortie.
// Témoin rouge : une base à deux Scope de même key sous deux uuid (temoin-doublons.cpp, joint).
// Usage : sonde-doublons <copie>/<base>.rag3db   (rc 0 : aucun doublon ; 1 : des doublons ; 2 : erreur)
#include <cstdint>
#include <cstdio>
#include <map>
#include <set>
#include <string>
#include <vector>

#include "rag3db.hpp"

using namespace rag3db::main;

static bool ok = true;
static std::vector<std::vector<std::string>> rows(Connection& c, const std::string& q) {
    std::vector<std::vector<std::string>> out;
    auto r = c.query(q);
    if (!r->isSuccess()) {
        printf("ERREUR %s : %s\n", q.c_str(), r->getErrorMessage().c_str());
        ok = false;
        return out;
    }
    while (r->hasNext()) {
        auto t = r->getNext();
        std::vector<std::string> row;
        for (auto i = 0u; i < t->len(); i++) row.push_back(t->getValue(i)->toString());
        out.push_back(std::move(row));
    }
    return out;
}
static std::string bq(const std::string& n) { return "`" + n + "`"; }
static std::string lit(const std::string& s) {
    std::string o = "'";
    for (char ch : s) {
        if (ch == '\'' || ch == '\\') o += '\\';
        o += ch;
    }
    return o + "'";
}

int main(int argc, char** argv) {
    if (argc < 2) return 2;
    SystemConfig config;
    config.readOnly = true;
    Database db(argv[1], config);
    Connection c(&db);
    auto version = rows(c, "CALL build_id() RETURN *;");
    printf("base %s\nlib %s\n", argv[1], version.empty() ? "?" : version[0][0].c_str());

    std::vector<std::string> nodeTables, relTables;
    for (auto& t : rows(c, "CALL show_tables() RETURN name, type;")) {
        if (t[1] == "NODE") nodeTables.push_back(t[0]);
        else if (t[1] == "REL") relTables.push_back(t[0]);
    }
    std::set<std::string> rels(relTables.begin(), relTables.end());
    uint64_t totalKeys = 0, totalExtra = 0, totalLogical = 0;
    printf("\n## Doublons de clé primaire\n");
    for (auto& t : nodeTables) {
        std::string pk;
        std::set<std::string> cols;
        for (auto& col : rows(c, "CALL table_info('" + t + "') RETURN name, `primary key`;")) {
            cols.insert(col[0]);
            if (col[1] == "True") pk = col[0];
        }
        if (pk.empty()) continue;
        auto n = rows(c, "MATCH (n:" + bq(t) + ") RETURN count(*);");
        auto dups = rows(c, "MATCH (n:" + bq(t) + ") WITH n." + bq(pk) + " AS k, count(*) AS c WHERE c > 1 "
                            "RETURN k, c ORDER BY c DESC, k;");
        uint64_t extra = 0;
        for (auto& d : dups) extra += std::stoull(d[1]) - 1;
        totalKeys += dups.size();
        totalExtra += extra;
        printf("TABLE %s (clé %s) : %s rangées, %zu clés en double, %llu rangées en trop%s\n", t.c_str(), pk.c_str(),
            n.empty() ? "?" : n[0][0].c_str(), dups.size(), (unsigned long long)extra,
            dups.empty() ? "" : "  <== DOUBLONS");
        const std::string chunkRel = t + "_CHUNKED_FROM";
        const std::string chunkTable = t + "_Chunk";
        bool hasVectors = false;
        std::string vectorCol;
        if (rels.count(chunkRel)) {
            for (auto& col : rows(c, "CALL table_info('" + chunkTable + "') RETURN name, type;")) {
                if (col[0].rfind("embedding__", 0) == 0) {
                    hasVectors = true;
                    vectorCol = col[0];
                }
            }
        }
        size_t shown = 0;
        for (auto& d : dups) {
            if (shown++ >= 10) break;
            std::string proj = "offset(id(n))";
            if (cols.count("_snapshot")) proj += ", n._snapshot";
            if (cols.count("_absent_since")) proj += ", n._absent_since";
            auto copies = rows(c, "MATCH (n:" + bq(t) + ") WHERE n." + bq(pk) + " = " + lit(d[0]) + " RETURN " + proj +
                                      " ORDER BY offset(id(n));");
            auto viaIndex = rows(c, "MATCH (n:" + bq(t) + " {" + bq(pk) + ": " + lit(d[0]) + "}) RETURN offset(id(n));");
            std::string idx;
            for (auto& v : viaIndex) idx += (idx.empty() ? "" : ",") + v[0];
            printf("  clé %s : %s rangées ; la recherche par la clé rend [%s]\n", d[0].c_str(), d[1].c_str(), idx.c_str());
            for (auto& cp : copies) {
                std::string extraInfo;
                if (rels.count(chunkRel)) {
                    auto k = rows(c, "MATCH (k:" + bq(chunkTable) + ")-[:" + bq(chunkRel) + "]->(n:" + bq(t) +
                                         ") WHERE offset(id(n)) = " + cp[0] + " RETURN count(k)" +
                                         (hasVectors ? ", count(k." + bq(vectorCol) + ")" : "") + ";");
                    if (!k.empty()) {
                        extraInfo = ", morceaux " + k[0][0] + (hasVectors ? ", vecteurs " + k[0][1] : "");
                    }
                }
                printf("    rangée %s%s%s%s\n", cp[0].c_str(), cp.size() > 1 ? (", _snapshot " + cp[1]).c_str() : "",
                    cp.size() > 2 ? (", _absent_since " + cp[2]).c_str() : "", extraInfo.c_str());
            }
        }
    }

    printf("\n## Doublons de clé logique (information)\n");
    const std::vector<std::pair<std::string, std::string>> logical = {
        {"Scope", "n.key"}, {"File", "n.path"}, {"Symbol", "n._project + '|' + n.name"},
        {"Library", "n._project + '|' + n.import_path"}};
    std::set<std::string> names(nodeTables.begin(), nodeTables.end());
    for (auto& [t, expr] : logical) {
        if (!names.count(t)) continue;
        auto dups = rows(c, "MATCH (n:" + bq(t) + ") WITH " + expr + " AS k, count(*) AS c WHERE c > 1 AND k IS NOT NULL "
                            "RETURN k, c ORDER BY c DESC, k;");
        uint64_t extra = 0;
        for (auto& d : dups) extra += std::stoull(d[1]) - 1;
        printf("LOGIQUE %s (%s) : %zu clés en double, %llu rangées en trop\n", t.c_str(), expr.c_str(), dups.size(),
            (unsigned long long)extra);
        totalLogical += dups.size();
        const std::string chunkRel = t + "_CHUNKED_FROM", chunkTable = t + "_Chunk";
        std::string vectorCol;
        if (rels.count(chunkRel)) {
            for (auto& col : rows(c, "CALL table_info('" + chunkTable + "') RETURN name;")) {
                if (col[0].rfind("embedding__", 0) == 0) vectorCol = col[0];
            }
        }
        for (size_t i = 0; i < dups.size() && i < 10; i++) {
            std::set<std::string> tcols;
            for (auto& col : rows(c, "CALL table_info('" + t + "') RETURN name;")) tcols.insert(col[0]);
            auto copies = rows(c, "MATCH (n:" + bq(t) + ") WHERE " + expr + " = " + lit(dups[i][0]) +
                                      " RETURN offset(id(n)), n._uuid, " +
                                      (tcols.count("_snapshot") ? "n._snapshot" : "''") + ", " +
                                      (tcols.count("_absent_since") ? "n._absent_since" : "''") +
                                      " ORDER BY offset(id(n));");
            printf("  %s : %s rangées\n", dups[i][0].substr(0, 160).c_str(), dups[i][1].c_str());
            for (auto& cp : copies) {
                std::string info;
                if (rels.count(chunkRel)) {
                    auto k = rows(c, "MATCH (k:" + bq(chunkTable) + ")-[:" + bq(chunkRel) + "]->(n:" + bq(t) +
                                         ") WHERE offset(id(n)) = " + cp[0] + " RETURN count(k)" +
                                         (vectorCol.empty() ? "" : ", count(k." + bq(vectorCol) + ")") + ";");
                    if (!k.empty()) info = ", morceaux " + k[0][0] + (vectorCol.empty() ? "" : ", vecteurs " + k[0][1]);
                }
                printf("    rangée %s, _uuid %s, _snapshot %s, _absent_since %s%s\n", cp[0].c_str(), cp[1].c_str(),
                    cp[2].c_str(), cp[3].c_str(), info.c_str());
            }
        }
    }
    printf("\nBILAN %s : %llu clés primaires en double, %llu rangées en trop ; %llu clés logiques en double%s\n",
        argv[1], (unsigned long long)totalKeys, (unsigned long long)totalExtra, (unsigned long long)totalLogical,
        ok ? "" : " ; DES REQUÊTES ONT ÉCHOUÉ");
    return !ok ? 2 : ((totalKeys || totalLogical) ? 1 : 0);
}
