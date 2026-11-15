// releve_sens.cpp (14 nov. 2026, hotfixs) — chaque relation d'une table, lue par ses deux rangements (la
// lecture ancrée nœud par nœud de check_rel_directions, de G), écrite une ligne par lecture :
//   sens \t id(r) \t a._uuid \t b._uuid \t prop1 \t prop2 …   (sens = avant | arriere ; NULL écrit \N)
// En tête, une ligne « # props » avec les noms des propriétés, et une « # bouts » avec les étiquettes.
// Lecture seule. Usage : releve_sens <base> <table> > sortie.tsv
#include <iostream>
#include <set>
#include <string>
#include <vector>
#include "rag3db.hpp"
using namespace rag3db::main;
static std::vector<std::vector<std::string>> rows(Connection& c, const std::string& q) {
    std::vector<std::vector<std::string>> out;
    auto r = c.query(q);
    if (!r->isSuccess()) { std::cerr << "ERREUR " << q << " : " << r->getErrorMessage() << "\n"; exit(2); }
    while (r->hasNext()) {
        auto t = r->getNext(); std::vector<std::string> row;
        for (auto i = 0u; i < t->len(); i++) { auto v = t->getValue(i); row.push_back(v->isNull() ? "\\N" : v->toString()); }
        out.push_back(std::move(row));
    }
    return out;
}
static std::string clean(std::string s) { for (auto& ch : s) if (ch == '\t' || ch == '\n') ch = ' '; return s; }
int main(int argc, char** argv) {
    if (argc != 3) { std::cerr << "usage : releve_sens <base> <table>\n"; return 2; }
    SystemConfig config; config.readOnly = true;
    Database db(argv[1], config); Connection c(&db);
    std::string t = argv[2];
    std::string proj = "a._uuid, b._uuid"; std::string noms;
    for (auto& p : rows(c, "CALL table_info('" + t + "') RETURN name;")) { proj += ", r.`" + p[0] + "`"; noms += "\t" + p[0]; }
    std::cout << "# props" << noms << "\n";
    // **Les bouts, par tranches d'offsets ancrées sur chaque étiquette** : une seule lecture non ancrée des bouts
    // (la première forme, celle de check_rel_directions) est jouée dans un seul sens par le planificateur, et une
    // source que ce sens a perdue n'était jamais lue, même par l'autre sens qui l'a encore (lockIndexForWrite,
    // code-principal, 14 nov.). Chaque nœud de chaque étiquette de bout est ancré ici, une tranche à la fois.
    auto etiquettes = rows(c, "CALL show_connection('" + t + "') RETURN *;");
    std::set<std::string> la, lb;
    for (auto& e : etiquettes) { la.insert(e[0]); lb.insert(e[1]); }
    std::cout << "# bouts"; for (auto& l : la) std::cout << "\t" << l; std::cout << "\t->"; for (auto& l : lb) std::cout << "\t" << l; std::cout << "\n";
    auto emit = [&](const char* sens, const std::vector<std::vector<std::string>>& rs) {
        for (auto& row : rs) { std::cout << sens; for (auto& v : row) std::cout << "\t" << clean(v); std::cout << "\n"; }
    };
    const uint64_t tranche = 500;
    for (auto& l : la) {
        auto m = rows(c, "MATCH (a:`" + l + "`) RETURN max(offset(id(a))) + 1;")[0][0];
        const uint64_t n = m == "\\N" ? 0 : std::stoull(m);  // une étiquette vide : rien à ancrer
        for (uint64_t o = 0; o < n; o += tranche)
            emit("avant", rows(c, "MATCH (a:`" + l + "`) WHERE offset(id(a)) >= " + std::to_string(o) + " AND offset(id(a)) < " +
                std::to_string(o + tranche) + " MATCH (a)-[r:`" + t + "`]->(b) RETURN id(r), " + proj + ";"));
    }
    for (auto& l : lb) {
        auto m = rows(c, "MATCH (b:`" + l + "`) RETURN max(offset(id(b))) + 1;")[0][0];
        const uint64_t n = m == "\\N" ? 0 : std::stoull(m);  // une étiquette vide : rien à ancrer
        for (uint64_t o = 0; o < n; o += tranche)
            emit("arriere", rows(c, "MATCH (b:`" + l + "`) WHERE offset(id(b)) >= " + std::to_string(o) + " AND offset(id(b)) < " +
                std::to_string(o + tranche) + " MATCH (a)-[r:`" + t + "`]->(b) RETURN id(r), " + proj + ";"));
    }
}
