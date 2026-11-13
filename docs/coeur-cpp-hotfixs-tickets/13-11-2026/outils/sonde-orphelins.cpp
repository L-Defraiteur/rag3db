// Sonde des bases de Lucie (13 novembre 2026, hotfixs, à la demande de c7 et du cœur) : sur une
// COPIE ouverte en lecture seule, les relations dont un bout n'existe plus et les morceaux dont le
// parent manque. Jamais une propriété d'un bout manquant n'est lue.
//
// Par table de nœuds T : le nombre de nœuds, le décalage maximal, et l'ensemble des décalages vivants.
// Par table de relations R et par paire (X, Y) de ses tables :
//   - sens avant, ancré sur la source : MATCH (a:X)-[r:R]->(b) RETURN offset(id(b)), label(b) ;
//     une destination « au-delà » a un décalage > maxY (la mesure du cœur) ; une destination « absente »
//     a un décalage qui n'est pas dans l'ensemble des vivants de Y (elle couvre aussi les trous) ;
//   - sens arrière, ancré sur la destination : MATCH (b:Y)<-[r:R]-(a) RETURN offset(id(a)), label(a),
//     de même pour les sources.
// Le plan de chaque requête est lu (EXPLAIN) : une requête qui ne part pas du bout voulu est dite
// « SENS NON PROUVÉ » et la sonde rend 2. Limites : une relation dont les deux bouts manquent échappe
// aux deux sens ; une table de nœuds vide ne peut pas servir d'ancrage.
// Les morceaux : MATCH (k:<T>_Chunk) OPTIONAL MATCH (p:<T> {_uuid: k._parent_uuid}) … p IS NULL.
//
// Usage : sonde-orphelins <copie>/<base>.rag3db   (rc 0 : rien d'orphelin ; 1 : des orphelins ; 2 : erreur)
#include <cstdint>
#include <cstdio>
#include <map>
#include <set>
#include <string>
#include <unordered_set>
#include <vector>

#include "rag3db.hpp"

using namespace rag3db::main;

static std::vector<std::vector<std::string>> rows(Connection& c, const std::string& q, bool& ok) {
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

int main(int argc, char** argv) {
    if (argc < 2) return 2;
    SystemConfig config;
    config.readOnly = true;
    Database db(argv[1], config);
    Connection c(&db);
    bool ok = true;
    auto version = rows(c, "CALL build_id() RETURN *;", ok);
    printf("base %s\nlib %s\n", argv[1], version.empty() ? "?" : version[0][0].c_str());

    std::vector<std::string> nodeTables, relTables;
    for (auto& t : rows(c, "CALL show_tables() RETURN name, type;", ok)) {
        if (t[1] == "NODE") nodeTables.push_back(t[0]);
        else if (t[1] == "REL") relTables.push_back(t[0]);
    }
    std::map<std::string, std::unordered_set<uint64_t>> alive;
    std::map<std::string, int64_t> maxOffset;
    printf("\n## Tables de nœuds\n");
    for (auto& t : nodeTables) {
        auto& set = alive[t];
        int64_t mx = -1;
        for (auto& r : rows(c, "MATCH (n:" + bq(t) + ") RETURN offset(id(n));", ok)) {
            auto o = std::stoull(r[0]);
            set.insert(o);
            mx = std::max<int64_t>(mx, (int64_t)o);
        }
        maxOffset[t] = mx;
        printf("NOEUDS %s : %zu nœuds, décalage max %lld, trous sous le max %lld\n", t.c_str(), set.size(),
            (long long)mx, (long long)(mx + 1 - (int64_t)set.size()));
    }

    uint64_t totalOrphans = 0, unproved = 0;
    printf("\n## Tables de relations (avant = ancré sur la source, arrière = ancré sur la destination)\n");
    bool explained = false;
    for (auto& rel : relTables) {
        auto pairs = rows(c, "CALL show_connection('" + rel + "') RETURN *;", ok);
        for (auto& p : pairs) {
            const auto& x = p[0];
            const auto& y = p[1];
            // Le filtre sur le bout d'ancrage fait partir le plan de ce bout ; le plan de chaque requête est
            // lu, et une requête qui ne lit pas dans le sens voulu ne compte pas (SENS NON PROUVÉ).
            const auto fwdQuery = "MATCH (a:" + bq(x) + ")-[r:" + bq(rel) +
                                  "]->(b) WHERE offset(id(a)) >= 0 RETURN offset(id(b)), label(b);";
            // Le motif s'écrit depuis le bout d'ancrage : (b:Y)<-[r]-(a) part de b là où (a)-[r]->(b:Y)
            // partait parfois de a (MENTIONS, 13 novembre).
            const auto bwdQuery = "MATCH (b:" + bq(y) + ")<-[r:" + bq(rel) +
                                  "]-(a) WHERE offset(id(b)) >= 0 RETURN offset(id(a)), label(a);";
            auto direction = [&](const std::string& q, const std::string& want) {
                for (auto& r : rows(c, "EXPLAIN " + q, ok)) {
                    if (r[0].find("Direction: " + want) != std::string::npos) return true;
                }
                return false;
            };
            const bool fwdProved = direction(fwdQuery, "(a)-[r]->(b)");
            const bool bwdProved = direction(bwdQuery, "(b)<-[r]-(a)");
            if (!fwdProved || !bwdProved) {
                unproved++;
            }
            (void)explained;
            uint64_t fwd = 0, fwdBeyond = 0, fwdAbsent = 0;
            for (auto& r : rows(c, fwdQuery, ok)) {
                if (r[1] != y) continue;
                fwd++;
                auto o = std::stoull(r[0]);
                if ((int64_t)o > maxOffset[y]) fwdBeyond++;
                if (!alive[y].count(o)) fwdAbsent++;
            }
            uint64_t bwd = 0, bwdBeyond = 0, bwdAbsent = 0;
            for (auto& r : rows(c, bwdQuery, ok)) {
                if (r[1] != x) continue;
                bwd++;
                auto o = std::stoull(r[0]);
                if ((int64_t)o > maxOffset[x]) bwdBeyond++;
                if (!alive[x].count(o)) bwdAbsent++;
            }
            totalOrphans += fwdAbsent + bwdAbsent;
            printf("REL %s (%s -> %s) : avant %llu, destinations au-delà %llu, absentes %llu ; arrière %llu, "
                   "sources au-delà %llu, absentes %llu%s%s%s\n",
                rel.c_str(), x.c_str(), y.c_str(), (unsigned long long)fwd, (unsigned long long)fwdBeyond,
                (unsigned long long)fwdAbsent, (unsigned long long)bwd, (unsigned long long)bwdBeyond,
                (unsigned long long)bwdAbsent, (fwdAbsent || bwdAbsent) ? "  <== ORPHELINES" : "",
                fwdProved ? "" : "  <== SENS AVANT NON PROUVÉ (le plan part de la destination)",
                bwdProved ? "" : "  <== SENS ARRIÈRE NON PROUVÉ (le plan part de la source)");
        }
    }

    printf("\n## Morceaux dont le parent manque\n");
    std::set<std::string> names(nodeTables.begin(), nodeTables.end());
    for (auto& t : nodeTables) {
        const std::string suffix = "_Chunk";
        if (t.size() <= suffix.size() || t.compare(t.size() - suffix.size(), suffix.size(), suffix) != 0) continue;
        const auto parent = t.substr(0, t.size() - suffix.size());
        if (!names.count(parent)) {
            printf("MORCEAUX %s : pas de table %s\n", t.c_str(), parent.c_str());
            continue;
        }
        auto r = rows(c, "MATCH (k:" + bq(t) + ") OPTIONAL MATCH (p:" + bq(parent) +
                             " {_uuid: k._parent_uuid}) WITH k, p WHERE p IS NULL RETURN count(k);", ok);
        auto all = rows(c, "MATCH (k:" + bq(t) + ") RETURN count(k);", ok);
        auto n = r.empty() ? std::string("?") : r[0][0];
        if (!r.empty() && r[0][0] != "0") totalOrphans += std::stoull(r[0][0]);
        printf("MORCEAUX %s : %s morceaux, %s sans parent %s%s\n", t.c_str(), all.empty() ? "?" : all[0][0].c_str(),
            n.c_str(), parent.c_str(), (!r.empty() && r[0][0] != "0") ? "  <== ORPHELINS" : "");
    }
    printf("\nBILAN %s : %llu orphelins (relations aux bouts absents, morceaux sans parent) ; %llu paires de "
           "tables au sens non prouvé%s\n",
        argv[1], (unsigned long long)totalOrphans, (unsigned long long)unproved,
        ok ? "" : " ; DES REQUÊTES ONT ÉCHOUÉ");
    return (!ok || unproved) ? 2 : (totalOrphans ? 1 : 0);
}
