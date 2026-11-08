// Les deux rangements d'une table de liens restent égaux sous la réécriture de la montre (ticket
// 2026-10-25-une-relation-locks-trouvee-d-un-seul-cote-fait-echouer-le-drain-de-la-montre).
//
// Sur la base du dépôt de code de Lucie, deux tables de liens sur quinze divergent entre leurs
// deux sens : des lignes vides lues comme des relations (relation 0 vers le nœud 0), des relations
// d'un seul côté, des relations rattachées au mauvais nœud. Ce sont les deux plus jeunes (LOCKS,
// QUERIES) : créées sur une base déjà peuplée, remplies ensuite par petits drains qui effacent
// puis recréent par MERGE les arêtes des scopes d'un fichier, vers quelques destinations très
// chargées, sous lecteurs et points de reprise, avec une colonne ajoutée en cours de route.
//
// Le témoin rejoue ce régime, puis relève les deux sens nœud par nœud (une requête ancrée par
// source pour le sens avant, par destination pour le sens arrière) : les deux ensembles de
// (relation, source, destination) doivent être égaux, base ouverte puis rouverte.

#include <algorithm>
#include <atomic>
#include <map>
#include <mutex>
#include <random>
#include <set>
#include <string>
#include <thread>
#include <vector>

#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

constexpr int64_t NUM_SCOPES = 70000;
constexpr int64_t NUM_SYMBOLS = 75000;

class LateRelTableRewrites : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        // Le tampon des tests est petit ; 145 000 nœuds et la table de liens demandent plus.
        systemConfig->bufferPoolSize = 2ull * 1024 * 1024 * 1024;
        createDBAndConn();
    }

    static void ok(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    void ok(const std::string& query) { ok(*conn, query); }

    // (relation, source, destination) d'un sens, lu nœud par nœud.
    using Edges = std::multimap<int64_t, std::pair<int64_t, int64_t>>;
    Edges anchored(bool forward, const std::set<int64_t>& nodes) {
        Edges edges;
        for (auto node : nodes) {
            const auto query =
                forward ? "MATCH (a:Scope)-[r:R]->(b:Symbol) WHERE a.id = " +
                              std::to_string(node) + " RETURN offset(id(r)), a.id, b.id;" :
                          "MATCH (a:Scope)-[r:R]->(b:Symbol) WHERE b.id = " +
                              std::to_string(node) + " RETURN offset(id(r)), a.id, b.id;";
            auto result = conn->query(query);
            EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
            while (result->isSuccess() && result->hasNext()) {
                auto row = result->getNext();
                edges.emplace(row->getValue(0)->getValue<int64_t>(),
                    std::make_pair(row->getValue(1)->getValue<int64_t>(),
                        row->getValue(2)->getValue<int64_t>()));
            }
        }
        return edges;
    }

    // Le nombre d'écarts entre les deux sens ; les premiers sont écrits.
    uint64_t gaps(const std::set<int64_t>& sources, const std::set<int64_t>& destinations) {
        const auto fwd = anchored(true, sources);
        const auto bwd = anchored(false, destinations);
        uint64_t n = 0;
        auto report = [&](const std::string& what, int64_t rel, std::pair<int64_t, int64_t> ends) {
            if (++n <= 20) {
                ADD_FAILURE() << what << " : relation " << rel << " " << ends.first << " -> "
                              << ends.second;
            }
        };
        for (auto& [rel, ends] : fwd) {
            if (bwd.count(rel) != 1 || fwd.count(rel) != 1) {
                report("sens avant seul ou double", rel, ends);
            } else if (bwd.find(rel)->second != ends) {
                report("bouts différents", rel, ends);
            }
        }
        for (auto& [rel, ends] : bwd) {
            if (fwd.count(rel) != 1 || bwd.count(rel) != 1) {
                report("sens arrière seul ou double", rel, ends);
            }
        }
        std::cout << "relevé : sens avant " << fwd.size() << ", sens arrière " << bwd.size()
                  << ", écarts " << n << "\n";
        return n;
    }
};

// Une table de liens née sur des tables de nœuds déjà grandes et au disque, remplie seulement par
// des drains (effacer les arêtes des sources d'un fichier, puis MERGE vers des destinations dont
// quelques-unes sont très chargées), un lecteur qui tient des instantanés, un point de reprise de
// loin en loin, une colonne ajoutée au milieu.
// La forme d'un `index confirm` complet (8 novembre 2026, « BASE ATTEINTE » sous cbc06c3e1 sur
// code-lourd et code-principal, après deux réindexations, alors que les mêmes bases étaient à 0 écart
// juste avant) : une transaction crée des nœuds neufs, efface toutes les arêtes d'une table de liens
// née tard, les recrée par un COPY (qui verse d'abord les nœuds locaux, G1), puis met à jour une
// propriété d'une partie des relations copiées (le chemin du rejeu A), et valide ; un point de reprise
// suit. Trois fois. Les deux sens doivent rester égaux, base ouverte puis rouverte.
TEST_F(LateRelTableRewrites, BothDirectionsStayEqualUnderAFullReindex) {
    constexpr int64_t SCOPES = 3000, SYMBOLS = 3500, EDGES = 12000;
    ok("CREATE NODE TABLE Scope(id INT64, PRIMARY KEY(id));");
    ok("CREATE NODE TABLE Symbol(id INT64, PRIMARY KEY(id));");
    ok("UNWIND range(0, " + std::to_string(SCOPES - 1) + ") AS i CREATE (:Scope {id: i});");
    ok("UNWIND range(0, " + std::to_string(SYMBOLS - 1) + ") AS i CREATE (:Symbol {id: i});");
    ok("CHECKPOINT;");
    ok("CREATE REL TABLE R(FROM Scope TO Symbol, line INT64, resolution STRING);");
    // Les arêtes d'un tour : depuis toutes les sources (les neuves comprises), vers quelques
    // destinations très chargées pour deux tiers d'entre elles.
    auto edges = [&](int round, int64_t scopes, int64_t symbols) {
        return "COPY R FROM (UNWIND range(0, " + std::to_string(EDGES - 1) +
               ") AS i RETURN (i * 13 + " + std::to_string(round) + ") % " + std::to_string(scopes) +
               " AS f, CASE WHEN i % 3 = 0 THEN (i * 31 + " + std::to_string(round) + ") % " +
               std::to_string(symbols) + " ELSE (i % 5) * 700 END AS t, i AS line, 'r" +
               std::to_string(round) + "' AS resolution);";
    };
    ok(edges(0, SCOPES, SYMBOLS));
    ok("CHECKPOINT;");
    int64_t scopes = SCOPES, symbols = SYMBOLS;
    for (int round = 1; round <= 3; round++) {
        ok("BEGIN TRANSACTION;");
        ok("UNWIND range(" + std::to_string(scopes) + ", " + std::to_string(scopes + 9) +
           ") AS i CREATE (:Scope {id: i});");
        ok("UNWIND range(" + std::to_string(symbols) + ", " + std::to_string(symbols + 4) +
           ") AS i CREATE (:Symbol {id: i});");
        scopes += 10;
        symbols += 5;
        ok("MATCH ()-[r:R]->() DELETE r;");
        ok(edges(round, scopes, symbols));
        ok("MATCH ()-[r:R]->() WHERE r.line % 17 = 0 SET r.line = r.line + 1000000, "
           "r.resolution = 'mise à jour';");
        ok("COMMIT;");
        ok("CHECKPOINT;");
    }
    std::set<int64_t> sources, destinations;
    for (int64_t s = 0; s < scopes; s++) {
        sources.insert(s);
    }
    for (int64_t d = 0; d < symbols; d++) {
        destinations.insert(d);
    }
    EXPECT_EQ(gaps(sources, destinations), 0u) << "base ouverte";
    conn.reset();
    database.reset();
    createDBAndConn();
    EXPECT_EQ(gaps(sources, destinations), 0u) << "base rouverte";
}

// Deux versements d'une même table de nœuds dans une transaction, à un seul écrivain (relecture du
// cœur, 8 novembre 2026) : deux tables de liens Scope→Scope, un COPY vers la première (qui verse
// les Scope locaux), d'autres Scope locaux et des relations locales vers eux, un COPY vers la
// seconde (qui verse encore). Les deux sens de chaque table doivent être égaux, sans mort ni rejeu.
TEST_F(LateRelTableRewrites, TwoFlushesOfOneNodeTableKeepBothDirectionsEqual) {
    ok("CREATE NODE TABLE Scope(id INT64, PRIMARY KEY(id));");
    ok("UNWIND range(0, 1999) AS i CREATE (:Scope {id: i});");
    ok("CHECKPOINT;");
    ok("CREATE REL TABLE A(FROM Scope TO Scope, line INT64);");
    ok("CREATE REL TABLE B(FROM Scope TO Scope, line INT64);");
    ok("BEGIN TRANSACTION;");
    ok("UNWIND range(2000, 2009) AS i CREATE (:Scope {id: i});");
    ok("COPY A FROM (UNWIND range(0, 2999) AS i RETURN (i * 7) % 2010 AS f, (i % 4) * 400 AS t, i AS line);");
    ok("UNWIND range(2010, 2019) AS i CREATE (:Scope {id: i});");
    ok("MATCH (x:Scope {id: 2015}), (y:Scope {id: 3}) CREATE (x)-[:B {line: -1}]->(y);");
    ok("MATCH (x:Scope {id: 4}), (y:Scope {id: 2016}) CREATE (x)-[:B {line: -2}]->(y);");
    ok("COPY B FROM (UNWIND range(0, 2999) AS i RETURN (i * 11) % 2020 AS f, ((i % 3) * 600 + i) % 2020 AS t, i AS line);");
    ok("MATCH ()-[r:A]->() WHERE r.line % 13 = 0 SET r.line = r.line + 100000;");
    ok("COMMIT;");
    ok("CHECKPOINT;");
    auto edgesOf = [&](const std::string& table, bool forward) {
        Edges edges;
        for (int64_t node = 0; node < 2020; node++) {
            const auto query = "MATCH (a:Scope)-[r:" + table + "]->(b:Scope) WHERE " +
                               (forward ? "a" : "b") + ".id = " + std::to_string(node) +
                               " RETURN offset(id(r)), a.id, b.id;";
            auto result = conn->query(query);
            EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
            while (result->isSuccess() && result->hasNext()) {
                auto row = result->getNext();
                edges.emplace(row->getValue(0)->getValue<int64_t>(),
                    std::make_pair(row->getValue(1)->getValue<int64_t>(),
                        row->getValue(2)->getValue<int64_t>()));
            }
        }
        return edges;
    };
    for (const std::string table : {"A", "B"}) {
        SCOPED_TRACE(table);
        const auto fwd = edgesOf(table, true);
        const auto bwd = edgesOf(table, false);
        EXPECT_EQ(fwd.size(), bwd.size()) << "[check: directions-agree] " << table;
        uint64_t gaps = 0;
        for (auto& [rel, ends] : fwd) {
            if (bwd.count(rel) != 1 || fwd.count(rel) != 1 || bwd.find(rel)->second != ends) {
                if (++gaps <= 10) {
                    ADD_FAILURE() << "[check: directions-agree] " << table << " relation " << rel
                                  << " " << ends.first << " -> " << ends.second;
                }
            }
        }
        EXPECT_EQ(gaps, 0u) << table;
    }
}

// La réindexation d'une base pleine : les scopes d'un fichier sont effacés avec leurs arêtes (DETACH
// DELETE), puis recréés à de nouveaux décalages, et leurs arêtes recréées par un COPY ; d'autres
// fichiers suivent, puis un point de reprise. La table de liens est née tard, sur des nœuds déjà au
// disque, et remplie avant par un COPY posé au disque (groupes CSR persistés, régions à réécrire).
TEST_F(LateRelTableRewrites, BothDirectionsStayEqualWhenFilesAreReindexedWithTheirNodes) {
    constexpr int64_t SCOPES = 4000, SYMBOLS = 4000;
    ok("CREATE NODE TABLE Scope(id INT64, file INT64, PRIMARY KEY(id));");
    ok("CREATE NODE TABLE Symbol(id INT64, PRIMARY KEY(id));");
    ok("UNWIND range(0, " + std::to_string(SCOPES - 1) + ") AS i CREATE (:Scope {id: i, file: i / 20});");
    ok("UNWIND range(0, " + std::to_string(SYMBOLS - 1) + ") AS i CREATE (:Symbol {id: i});");
    ok("CHECKPOINT;");
    ok("CREATE REL TABLE R(FROM Scope TO Symbol, line INT64, resolution STRING);");
    ok("COPY R FROM (UNWIND range(0, 15999) AS i RETURN (i * 13) % " + std::to_string(SCOPES) +
       " AS f, CASE WHEN i % 3 = 0 THEN (i * 31) % " + std::to_string(SYMBOLS) +
       " ELSE (i % 5) * 700 END AS t, i AS line, 'r0' AS resolution);");
    ok("CHECKPOINT;");
    std::mt19937_64 random{20261108};
    int64_t nextId = SCOPES;
    for (int round = 0; round < 60; round++) {
        const auto file = static_cast<int64_t>(random() % (SCOPES / 20));
        ok("BEGIN TRANSACTION;");
        ok("MATCH (a:Scope) WHERE a.file = " + std::to_string(file) + " DETACH DELETE a;");
        const auto first = nextId;
        nextId += 20;
        ok("UNWIND range(" + std::to_string(first) + ", " + std::to_string(first + 19) +
           ") AS i CREATE (:Scope {id: i, file: " + std::to_string(file) + "});");
        ok("COPY R FROM (UNWIND range(0, 79) AS i RETURN " + std::to_string(first) +
           " + i % 20 AS f, CASE WHEN i % 2 = 0 THEN (i * 37 + " + std::to_string(round) + ") % " +
           std::to_string(SYMBOLS) + " ELSE (i % 5) * 700 END AS t, i AS line, 'r" +
           std::to_string(round) + "' AS resolution);");
        ok("MATCH (a:Scope)-[r:R]->() WHERE a.file = " + std::to_string(file) +
           " AND r.line % 7 = 0 SET r.line = r.line + 1000;");
        ok("COMMIT;");
        if (round % 15 == 14) {
            ok("CHECKPOINT;");
        }
    }
    ok("CHECKPOINT;");
    std::set<int64_t> sources, destinations;
    for (int64_t s = 0; s < nextId; s++) {
        sources.insert(s);
    }
    for (int64_t d = 0; d < SYMBOLS; d++) {
        destinations.insert(d);
    }
    EXPECT_EQ(gaps(sources, destinations), 0u) << "base ouverte";
    conn.reset();
    database.reset();
    createDBAndConn();
    EXPECT_EQ(gaps(sources, destinations), 0u) << "base rouverte";
}

// Une liste qui grossit fortement dans un groupe CSR au disque, au milieu de listes qui ne bougent
// pas (8 novembre 2026, lu sur la base abîmée par un index complet) : la liste arrière de RecordBase
// (5354), que l'index n'avait pas touchée, pointait après le point de reprise sur les rangées d'une
// autre destination (3223), avec une case vide au début. Une région feuille fait 1 024 nœuds ;
// une liste qui déborde fait monter la réécriture jusqu'à une région qui couvre des nœuds intacts.
TEST_F(LateRelTableRewrites, AListThatGrowsLeavesItsUntouchedNeighboursInPlace) {
    constexpr int64_t SCOPES = 6000, SYMBOLS = 8192;
    ok("CREATE NODE TABLE Scope(id INT64, PRIMARY KEY(id));");
    ok("CREATE NODE TABLE Symbol(id INT64, PRIMARY KEY(id));");
    ok("UNWIND range(0, " + std::to_string(SCOPES - 1) + ") AS i CREATE (:Scope {id: i});");
    ok("UNWIND range(0, " + std::to_string(SYMBOLS - 1) + ") AS i CREATE (:Symbol {id: i});");
    ok("CHECKPOINT;");
    ok("CREATE REL TABLE R(FROM Scope TO Symbol, line INT64);");
    // Une arête vers chaque destination, et quelques-unes vers 5354 : des listes courtes, au disque.
    ok("COPY R FROM (UNWIND range(0, " + std::to_string(SYMBOLS - 1) + ") AS i RETURN i % " +
       std::to_string(SCOPES) + " AS f, i AS t, i AS line);");
    ok("COPY R FROM (UNWIND range(0, 4) AS i RETURN 5000 + i AS f, 5354 AS t, -i AS line);");
    ok("CHECKPOINT;");
    // Trois tours : la liste de 3223 grossit de 1 500 arêtes par un COPY, puis un point de reprise.
    for (int round = 0; round < 3; round++) {
        ok("COPY R FROM (UNWIND range(0, 1499) AS i RETURN (i * 7 + " + std::to_string(round) +
           ") % " + std::to_string(SCOPES) + " AS f, 3223 AS t, 100000 * " +
           std::to_string(round + 1) + " + i AS line);");
        ok("CHECKPOINT;");
    }
    std::set<int64_t> sources, destinations;
    for (int64_t s = 0; s < SCOPES; s++) {
        sources.insert(s);
    }
    for (int64_t d = 0; d < SYMBOLS; d++) {
        destinations.insert(d);
    }
    EXPECT_EQ(gaps(sources, destinations), 0u) << "base ouverte";
    conn.reset();
    database.reset();
    createDBAndConn();
    EXPECT_EQ(gaps(sources, destinations), 0u) << "base rouverte";
}

// Le chemin réel d'un `index confirm` sur les petites tables (trace Cypher du 8 novembre 2026,
// lib d'étude) : elles s'écrivent par `UNWIND $items … MERGE (a)-[r]->(b) SET r.line = …`, en
// instructions validées une à une, jamais par COPY ; aucune arête n'est effacée, sauf à la fin, par
// le DETACH DELETE des scopes des fichiers disparus (103 dans la trace). La table Scope→Scope est liée
// à elle-même, comme INHERITS_FROM : un scope effacé y est source et destination.
TEST_F(LateRelTableRewrites, MergedRelsKeepBothDirectionsThroughADetachDeleteOfTheirNodes) {
    constexpr int64_t SCOPES = 6000, SYMBOLS = 3000;
    ok("CREATE NODE TABLE Scope(id INT64, PRIMARY KEY(id));");
    ok("CREATE NODE TABLE Symbol(id INT64, PRIMARY KEY(id));");
    ok("UNWIND range(0, " + std::to_string(SCOPES - 1) + ") AS i CREATE (:Scope {id: i});");
    ok("UNWIND range(0, " + std::to_string(SYMBOLS - 1) + ") AS i CREATE (:Symbol {id: i});");
    ok("CHECKPOINT;");
    ok("CREATE REL TABLE R(FROM Scope TO Symbol, line INT64, via STRING);");
    ok("CREATE REL TABLE S(FROM Scope TO Scope, line INT64);");
    // Un passage d'index : par lots de 200 sources, les arêtes lues, puis MERGE … SET, puis relues.
    auto pass = [&](int round) {
        for (int64_t first = 0; first < SCOPES; first += 200) {
            const auto last = std::min(first + 199, SCOPES - 1);
            const auto range = std::to_string(first) + ", " + std::to_string(last);
            ok("UNWIND range(" + range + ") AS f MATCH (a:Scope {id: f})-[:R]->(b:Symbol) RETURN a.id, b.id;");
            ok("UNWIND range(" + range + ") AS f WITH f, CASE WHEN f % 3 = 0 THEN (f * 7 + " +
               std::to_string(round) + ") % " + std::to_string(SYMBOLS) +
               " ELSE (f % 4) * 600 END AS t MATCH (a:Scope {id: f}), (b:Symbol {id: t}) MERGE "
               "(a)-[r:R]->(b) SET r.line = f * 10 + " + std::to_string(round) + ", r.via = 'p" +
               std::to_string(round) + "';");
            ok("UNWIND range(" + range + ") AS f WITH f WHERE f % 2 = 0 MATCH (a:Scope {id: f}), "
               "(b:Scope {id: (f * 13 + " + std::to_string(round) + ") % " + std::to_string(SCOPES) +
               "}) MERGE (a)-[r:S]->(b) SET r.line = f + " + std::to_string(round) + ";");
            ok("UNWIND range(" + range + ") AS f MATCH (a:Scope {id: f})-[:R]->(b:Symbol) RETURN a.id, b.id;");
        }
    };
    pass(0);
    ok("CHECKPOINT;");
    pass(1);
    // Les scopes des fichiers disparus, effacés avec leurs arêtes, sources et destinations.
    ok("UNWIND range(0, 102) AS i MATCH (n:Scope {id: (i * 53) % " + std::to_string(SCOPES) +
       "}) DETACH DELETE n;");
    ok("CHECKPOINT;");
    std::set<int64_t> sources, destinations;
    for (int64_t s = 0; s < SCOPES; s++) {
        sources.insert(s);
    }
    for (int64_t d = 0; d < SYMBOLS; d++) {
        destinations.insert(d);
    }
    EXPECT_EQ(gaps(sources, destinations), 0u) << "R, base ouverte";
    // La table Scope→Scope, ses deux sens.
    Edges fwd, bwd;
    for (int64_t node = 0; node < SCOPES; node++) {
        for (const bool forward : {true, false}) {
            auto result = conn->query(std::string("MATCH (a:Scope)-[r:S]->(b:Scope) WHERE ") +
                                      (forward ? "a" : "b") + ".id = " + std::to_string(node) +
                                      " RETURN offset(id(r)), a.id, b.id;");
            while (result->isSuccess() && result->hasNext()) {
                auto row = result->getNext();
                (forward ? fwd : bwd)
                    .emplace(row->getValue(0)->getValue<int64_t>(),
                        std::make_pair(row->getValue(1)->getValue<int64_t>(),
                            row->getValue(2)->getValue<int64_t>()));
            }
        }
    }
    uint64_t gapsS = 0;
    for (auto& [rel, ends] : fwd) {
        if (bwd.count(rel) != 1 || fwd.count(rel) != 1 || bwd.find(rel)->second != ends) {
            if (++gapsS <= 10) {
                ADD_FAILURE() << "[check: directions-agree] S relation " << rel << " " << ends.first
                              << " -> " << ends.second;
            }
        }
    }
    std::cout << "S : sens avant " << fwd.size() << ", sens arrière " << bwd.size() << ", écarts "
              << gapsS << "\n";
    EXPECT_EQ(fwd.size(), bwd.size());
    EXPECT_EQ(gapsS, 0u) << "S, base ouverte";
}

TEST_F(LateRelTableRewrites, BothDirectionsStayEqualUnderDrains) {
    ok("CREATE NODE TABLE Scope(id INT64, PRIMARY KEY(id));");
    ok("CREATE NODE TABLE Symbol(id INT64, PRIMARY KEY(id));");
    ok("UNWIND range(0, " + std::to_string(NUM_SCOPES - 1) + ") AS i CREATE (:Scope {id: i});");
    ok("UNWIND range(0, " + std::to_string(NUM_SYMBOLS - 1) + ") AS i CREATE (:Symbol {id: i});");
    ok("CHECKPOINT;");
    ok("CREATE REL TABLE R(FROM Scope TO Symbol, line INT64);");

    std::atomic<bool> stop{false};
    std::thread reader([&] {
        Connection connection(database.get());
        while (!stop.load()) {
            connection.query("BEGIN TRANSACTION READ ONLY;");
            connection.query("MATCH (a:Scope)-[r:R]->(b:Symbol) RETURN count(*);");
            std::this_thread::sleep_for(std::chrono::milliseconds(20));
            connection.query("MATCH (b:Symbol)<-[r:R]-(a:Scope) RETURN count(*);");
            connection.query("COMMIT;");
        }
    });

    std::mt19937_64 random{20261025};
    const std::vector<int64_t> hot{3344, 3389, 9179, 13803, 17616};
    std::set<int64_t> sources, destinations;
    constexpr int ROUNDS = 300;
    for (int round = 0; round < ROUNDS; round++) {
        // Un fichier : quelques scopes voisins, n'importe où dans la table.
        const auto first = static_cast<int64_t>(random() % (NUM_SCOPES - 8));
        const auto count = 1 + static_cast<int64_t>(random() % 8);
        std::string ids, pairs;
        for (auto s = first; s < first + count; s++) {
            ids += (ids.empty() ? "" : ",") + std::to_string(s);
            sources.insert(s);
            const auto edges = static_cast<int>(random() % 4);
            for (int e = 0; e < edges; e++) {
                const auto dst = random() % 3 == 0 ?
                                     static_cast<int64_t>(random() % NUM_SYMBOLS) :
                                     hot[random() % hot.size()];
                destinations.insert(dst);
                pairs += (pairs.empty() ? "" : ",") + std::string("[") + std::to_string(s) +
                         "," + std::to_string(dst) + "," + std::to_string(round) + "]";
            }
        }
        ok("BEGIN TRANSACTION;");
        ok("MATCH (a:Scope)-[r:R]->() WHERE a.id IN [" + ids + "] DELETE r;");
        if (!pairs.empty()) {
            ok("UNWIND [" + pairs +
               "] AS p MATCH (a:Scope {id: p[1]}), (b:Symbol {id: p[2]}) MERGE (a)-[r:R]->(b) "
               "SET r.line = p[3];");
        }
        ok("COMMIT;");
        if (round == ROUNDS / 2) {
            ok("ALTER TABLE R ADD via STRING;");
        }
        if (round % 40 == 39) {
            ok("CHECKPOINT;");
        }
    }
    stop = true;
    reader.join();
    for (auto dst : hot) {
        destinations.insert(dst);
    }
    // Une case vide lue comme une ligne rend le nœud 0 : il est relevé des deux côtés.
    sources.insert(0);
    destinations.insert(0);

    EXPECT_EQ(gaps(sources, destinations), 0u) << "base ouverte";
    ok("CHECKPOINT;");
    conn.reset();
    database.reset();
    createDBAndConn();
    EXPECT_EQ(gaps(sources, destinations), 0u) << "base rouverte";
}

// Le même régime avec deux écrivains à la fois, dont l'un annule une transaction sur trois après
// avoir écrit (un drain en échec est défait) : les deux réécrivent les arêtes de sources voisines
// vers les mêmes destinations chargées, donc dans les mêmes groupes CSR des deux sens.
TEST_F(LateRelTableRewrites, BothDirectionsStayEqualUnderConcurrentAndRolledBackDrains) {
    ok("CREATE NODE TABLE Scope(id INT64, PRIMARY KEY(id));");
    ok("CREATE NODE TABLE Symbol(id INT64, PRIMARY KEY(id));");
    ok("UNWIND range(0, " + std::to_string(NUM_SCOPES - 1) + ") AS i CREATE (:Scope {id: i});");
    ok("UNWIND range(0, " + std::to_string(NUM_SYMBOLS - 1) + ") AS i CREATE (:Symbol {id: i});");
    ok("CHECKPOINT;");
    ok("CREATE REL TABLE R(FROM Scope TO Symbol, line INT64);");

    const std::vector<int64_t> hot{3344, 3389, 9179, 13803, 17616};
    std::mutex touchedMutex;
    std::set<int64_t> sources, destinations;
    std::atomic<bool> stop{false};
    std::atomic<int> refused{0}, rolledBack{0}, committed{0};
    std::thread reader([&] {
        Connection connection(database.get());
        while (!stop.load()) {
            connection.query("BEGIN TRANSACTION READ ONLY;");
            connection.query("MATCH (b:Symbol)<-[r:R]-(a:Scope) RETURN count(*);");
            std::this_thread::sleep_for(std::chrono::milliseconds(15));
            connection.query("COMMIT;");
        }
    });
    auto writer = [&](int id, bool rollsBack) {
        Connection connection(database.get());
        std::mt19937_64 random{20261026u + id};
        // Les deux écrivains travaillent dans la même bande de sources : mêmes groupes CSR.
        for (int round = 0; round < 200; round++) {
            const auto first = 20000 + static_cast<int64_t>(random() % 4000);
            const auto count = 1 + static_cast<int64_t>(random() % 6);
            std::string ids, pairs;
            for (auto s = first; s < first + count; s++) {
                ids += (ids.empty() ? "" : ",") + std::to_string(s);
                for (int e = 0; e < 3; e++) {
                    const auto dst = random() % 3 == 0 ?
                                         static_cast<int64_t>(random() % NUM_SYMBOLS) :
                                         hot[random() % hot.size()];
                    pairs += (pairs.empty() ? "" : ",") + std::string("[") + std::to_string(s) +
                             "," + std::to_string(dst) + "," + std::to_string(round) + "]";
                    std::lock_guard lock{touchedMutex};
                    destinations.insert(dst);
                }
                std::lock_guard lock{touchedMutex};
                sources.insert(s);
            }
            const auto rollback = rollsBack && random() % 3 == 0;
            if (!connection.query("BEGIN TRANSACTION;")->isSuccess()) {
                refused++;
                continue;
            }
            auto step = [&](const std::string& query) {
                return connection.query(query)->isSuccess();
            };
            const auto written = step("MATCH (a:Scope)-[r:R]->() WHERE a.id IN [" + ids + "] DELETE r;") &&
                                 step("UNWIND [" + pairs +
                                      "] AS p MATCH (a:Scope {id: p[1]}), (b:Symbol {id: p[2]}) "
                                      "MERGE (a)-[r:R]->(b) SET r.line = p[3];");
            if (!written) {
                // Refusée (un verrou tenu par l'autre écrivain) : la transaction est déjà défaite.
                refused++;
                connection.query("ROLLBACK;");
            } else if (rollback) {
                rolledBack++;
                connection.query("ROLLBACK;");
            } else if (connection.query("COMMIT;")->isSuccess()) {
                committed++;
            } else {
                refused++;
            }
            if (id == 0 && round % 50 == 49) {
                connection.query("CHECKPOINT;");
            }
        }
    };
    std::thread first(writer, 0, false), second(writer, 1, true);
    first.join();
    second.join();
    stop = true;
    reader.join();
    std::cout << "validées " << committed << ", annulées " << rolledBack << ", refusées " << refused
              << "\n";
    for (auto dst : hot) {
        destinations.insert(dst);
    }
    sources.insert(0);
    destinations.insert(0);

    EXPECT_GT(committed.load(), 100);
    EXPECT_EQ(gaps(sources, destinations), 0u) << "base ouverte";
    ok("CHECKPOINT;");
    conn.reset();
    database.reset();
    createDBAndConn();
    EXPECT_EQ(gaps(sources, destinations), 0u) << "base rouverte";
}

// Le même régime à travers des arrêts brutaux : huit vies de processus, chacune ouvre la base,
// draine (effacer puis MERGE vers des destinations chargées, points de reprise automatiques au
// seuil de 1 Mio), puis meurt base ouverte (SIGKILL), après une validation ou au milieu d'une
// transaction. La vie suivante rejoue le journal. Puis le relevé des deux sens.
TEST_F(LateRelTableRewrites, BothDirectionsStayEqualAcrossKilledProcesses) {
    ok("CREATE NODE TABLE Scope(id INT64, PRIMARY KEY(id));");
    ok("CREATE NODE TABLE Symbol(id INT64, PRIMARY KEY(id));");
    ok("UNWIND range(0, " + std::to_string(NUM_SCOPES - 1) + ") AS i CREATE (:Scope {id: i});");
    ok("UNWIND range(0, " + std::to_string(NUM_SYMBOLS - 1) + ") AS i CREATE (:Symbol {id: i});");
    ok("CHECKPOINT;");
    ok("CREATE REL TABLE R(FROM Scope TO Symbol, line INT64);");
    conn.reset();
    database.reset();

    // Les destinations connues du parent : cinq très chargées, deux cents ordinaires.
    std::vector<int64_t> targets{3344, 3389, 9179, 13803, 17616};
    for (int64_t i = 0; i < 200; i++) {
        targets.push_back(30000 + i * 211);
    }
    constexpr int LIVES = 8;
    for (int life = 0; life < LIVES; life++) {
        const auto child = fork();
        ASSERT_GE(child, 0);
        if (child == 0) {
            auto config = *systemConfig;
            config.checkpointThreshold = 1 << 20;
            Database db(databasePath, config);
            Connection connection(&db);
            std::mt19937_64 random{20261107u + life};
            const auto dieAt = 100 + static_cast<int>(random() % 50);
            const auto midTransaction = life % 2 == 1;
            for (int round = 0;; round++) {
                const auto first = 20000 + static_cast<int64_t>(random() % 4000);
                const auto count = 1 + static_cast<int64_t>(random() % 6);
                std::string ids, pairs;
                for (auto s = first; s < first + count; s++) {
                    ids += (ids.empty() ? "" : ",") + std::to_string(s);
                    for (int e = 0; e < 4; e++) {
                        const auto dst = random() % 4 == 0 ? targets[5 + random() % 200] :
                                                             targets[random() % 5];
                        pairs += (pairs.empty() ? "" : ",") + std::string("[") +
                                 std::to_string(s) + "," + std::to_string(dst) + "," +
                                 std::to_string(round) + "]";
                    }
                }
                connection.query("BEGIN TRANSACTION;");
                connection.query("MATCH (a:Scope)-[r:R]->() WHERE a.id IN [" + ids + "] DELETE r;");
                if (round == dieAt && midTransaction) {
                    kill(getpid(), SIGKILL);
                }
                connection.query("UNWIND [" + pairs +
                                 "] AS p MATCH (a:Scope {id: p[1]}), (b:Symbol {id: p[2]}) "
                                 "MERGE (a)-[r:R]->(b) SET r.line = p[3];");
                connection.query("COMMIT;");
                if (round == dieAt) {
                    kill(getpid(), SIGKILL);
                }
            }
        }
        int status = 0;
        ASSERT_EQ(waitpid(child, &status, 0), child);
        ASSERT_TRUE(WIFSIGNALED(status) && WTERMSIG(status) == SIGKILL)
            << "la vie " << life << " n'est pas morte par SIGKILL (statut " << status << ")";
    }

    createDBAndConn();
    std::set<int64_t> sources, destinations(targets.begin(), targets.end());
    for (int64_t s = 20000; s < 24006; s++) {
        sources.insert(s);
    }
    sources.insert(0);
    destinations.insert(0);
    EXPECT_EQ(gaps(sources, destinations), 0u) << "base rouverte après les arrêts brutaux";
    ok("CHECKPOINT;");
    conn.reset();
    database.reset();
    createDBAndConn();
    EXPECT_EQ(gaps(sources, destinations), 0u) << "après un point de reprise et une réouverture";
}

// Les drains qui partent par COPY (au-delà de 200 arêtes par relation, le seuil du crate) et
// sont annulés après avoir écrit : une instruction suivante échoue et fait rouler la transaction,
// comme les drains de Lucie condamnés par la recherche dense le 25 octobre. Un COPY pose ses
// lignes dans les blocs partagés ; son annulation (CSRNodeGroup::rollbackInsert) les retire en
// tronquant au rang où elles commencent. Un second écrivain valide pendant ce temps des drains
// par MERGE vers les mêmes destinations, un lecteur tient des instantanés, des points de reprise
// passent.
TEST_F(LateRelTableRewrites, BothDirectionsStayEqualUnderRolledBackCopyDrains) {
    ok("CREATE NODE TABLE Scope(id INT64, PRIMARY KEY(id));");
    ok("CREATE NODE TABLE Symbol(id INT64, PRIMARY KEY(id));");
    ok("UNWIND range(0, " + std::to_string(NUM_SCOPES - 1) + ") AS i CREATE (:Scope {id: i});");
    ok("UNWIND range(0, " + std::to_string(NUM_SYMBOLS - 1) + ") AS i CREATE (:Symbol {id: i});");
    ok("CHECKPOINT;");
    ok("CREATE REL TABLE R(FROM Scope TO Symbol, line INT64);");

    const std::vector<int64_t> hot{3344, 3389, 9179, 13803, 17616};
    std::mutex touchedMutex;
    std::set<int64_t> sources, destinations;
    std::atomic<bool> stop{false};
    std::atomic<int> copied{0}, copyRolledBack{0}, copyRefused{0}, merged{0}, mergeRefused{0};
    std::thread reader([&] {
        Connection connection(database.get());
        while (!stop.load()) {
            connection.query("BEGIN TRANSACTION READ ONLY;");
            connection.query("MATCH (b:Symbol)<-[r:R]-(a:Scope) RETURN count(*);");
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
            connection.query("MATCH (a:Scope)-[r:R]->(b:Symbol) RETURN count(*);");
            connection.query("COMMIT;");
        }
    });
    // Un drain : effacer les arêtes des sources du fichier, puis les reposer (par COPY ou MERGE).
    auto drain = [&](std::mt19937_64& random, int64_t band, int edges, int round) {
        const auto first = band + static_cast<int64_t>(random() % 3000);
        std::string ids, pairs;
        for (auto s = first; s < first + 60; s++) {
            ids += (ids.empty() ? "" : ",") + std::to_string(s);
            std::lock_guard lock{touchedMutex};
            sources.insert(s);
        }
        for (int e = 0; e < edges; e++) {
            const auto s = first + static_cast<int64_t>(random() % 60);
            const auto dst = random() % 4 == 0 ? static_cast<int64_t>(random() % NUM_SYMBOLS) :
                                                 hot[random() % hot.size()];
            pairs += (pairs.empty() ? "" : ",") + std::string("[") + std::to_string(s) + "," +
                     std::to_string(dst) + "," + std::to_string(round) + "]";
            std::lock_guard lock{touchedMutex};
            destinations.insert(dst);
        }
        return std::make_pair(ids, pairs);
    };
    std::thread copier([&] {
        Connection connection(database.get());
        std::mt19937_64 random{20261107u};
        for (int round = 0; round < 150; round++) {
            const auto [ids, pairs] = drain(random, 20000, 250, round);
            const auto fails = random() % 3 == 0;
            if (!connection.query("BEGIN TRANSACTION;")->isSuccess()) {
                copyRefused++;
                continue;
            }
            const auto written =
                connection.query("MATCH (a:Scope)-[r:R]->() WHERE a.id IN [" + ids + "] DELETE r;")
                    ->isSuccess() &&
                connection.query("COPY R FROM (UNWIND [" + pairs +
                                 "] AS p RETURN p[1], p[2], p[3]);")
                    ->isSuccess();
            if (!written) {
                copyRefused++;
                connection.query("ROLLBACK;");
                continue;
            }
            if (fails) {
                // L'instruction suivante échoue à l'exécution : la transaction roule.
                connection.query("RETURN CAST('pas un nombre' AS INT64);");
                connection.query("ROLLBACK;");
                copyRolledBack++;
            } else if (connection.query("COMMIT;")->isSuccess()) {
                copied++;
            } else {
                copyRefused++;
            }
        }
    });
    std::thread merger([&] {
        Connection connection(database.get());
        std::mt19937_64 random{20261108u};
        for (int round = 0; round < 300; round++) {
            const auto [ids, pairs] = drain(random, 26000, 40, round);
            if (!connection.query("BEGIN TRANSACTION;")->isSuccess()) {
                mergeRefused++;
                continue;
            }
            const auto written =
                connection.query("MATCH (a:Scope)-[r:R]->() WHERE a.id IN [" + ids + "] DELETE r;")
                    ->isSuccess() &&
                connection.query("UNWIND [" + pairs +
                                 "] AS p MATCH (a:Scope {id: p[1]}), (b:Symbol {id: p[2]}) "
                                 "MERGE (a)-[r:R]->(b) SET r.line = p[3];")
                    ->isSuccess();
            if (written && connection.query("COMMIT;")->isSuccess()) {
                merged++;
            } else {
                mergeRefused++;
                connection.query("ROLLBACK;");
            }
            if (round % 30 == 29) {
                connection.query("CHECKPOINT;");
            }
        }
    });
    copier.join();
    merger.join();
    stop = true;
    reader.join();
    std::cout << "COPY validés " << copied << ", COPY annulés " << copyRolledBack
              << ", COPY refusés " << copyRefused << " ; MERGE validés " << merged
              << ", MERGE refusés " << mergeRefused << "\n";
    for (auto dst : hot) {
        destinations.insert(dst);
    }
    sources.insert(0);
    destinations.insert(0);

    EXPECT_GT(copyRolledBack.load(), 10);
    EXPECT_GT(merged.load(), 100);
    EXPECT_EQ(gaps(sources, destinations), 0u) << "base ouverte";
    ok("CHECKPOINT;");
    conn.reset();
    database.reset();
    createDBAndConn();
    EXPECT_EQ(gaps(sources, destinations), 0u) << "base rouverte";
}

// Le COPY de relations refusé faute de tampon, et le filet du crate depuis le 5 octobre : la base
// rouverte sous un tampon serré, des drains par COPY dont les lots grossissent jusqu'au refus
// (« The buffer pool is full », au COPY ou au point de reprise de son COMMIT), puis ROLLBACK,
// fermeture sans point de reprise, réouverture (le journal rejoué), et le drain suivant. Les
// chaînes longues des propriétés (comme `usage` et `via` de LOCKS) remplissent le tampon.
TEST_F(LateRelTableRewrites, BothDirectionsStayEqualAfterACopyRefusedForMemory) {
    ok("CREATE NODE TABLE Scope(id INT64, PRIMARY KEY(id));");
    ok("CREATE NODE TABLE Symbol(id INT64, PRIMARY KEY(id));");
    ok("UNWIND range(0, " + std::to_string(NUM_SCOPES - 1) + ") AS i CREATE (:Scope {id: i});");
    ok("UNWIND range(0, " + std::to_string(NUM_SYMBOLS - 1) + ") AS i CREATE (:Symbol {id: i});");
    ok("CHECKPOINT;");
    ok("CREATE REL TABLE R(FROM Scope TO Symbol, line INT64, usage STRING, via STRING);");
    conn.reset();
    database.reset();
    systemConfig->bufferPoolSize = 128ull * 1024 * 1024;
    createDBAndConn();

    const std::vector<int64_t> hot{3344, 3389, 9179, 13803, 17616};
    std::mt19937_64 random{20261109u};
    std::set<int64_t> sources, destinations(hot.begin(), hot.end());
    const std::string padding(900, 'u');
    int refusals = 0, commitRefusals = 0, copies = 0;
    for (int round = 0; round < 40 && refusals < 3; round++) {
        const auto edges = 500 << (round % 7);
        const auto first = 20000 + static_cast<int64_t>(random() % 3000);
        std::string ids, pairs;
        for (auto s = first; s < first + 80; s++) {
            ids += (ids.empty() ? "" : ",") + std::to_string(s);
            sources.insert(s);
        }
        for (int e = 0; e < edges; e++) {
            const auto s = first + static_cast<int64_t>(random() % 80);
            const auto dst = random() % 4 == 0 ? static_cast<int64_t>(random() % NUM_SYMBOLS) :
                                                 hot[random() % hot.size()];
            destinations.insert(dst);
            pairs += (pairs.empty() ? "" : ",") + std::string("[") + std::to_string(s) + "," +
                     std::to_string(dst) + "," + std::to_string(round) + "]";
        }
        ok("BEGIN TRANSACTION;");
        ok("MATCH (a:Scope)-[r:R]->() WHERE a.id IN [" + ids + "] DELETE r;");
        auto copy = conn->query("COPY R FROM (UNWIND [" + pairs + "] AS p RETURN p[1], p[2], p[3], '" +
                                padding + "' + string(p[3]), 'appel');");
        // Le refus peut venir du COPY, ou du point de reprise qui porte sa durabilité, au COMMIT :
        // la base est alors à rouvrir, et rien du COPY ne reste.
        std::string refusal;
        if (!copy->isSuccess()) {
            refusal = copy->getErrorMessage();
        } else {
            auto commit = conn->query("COMMIT;");
            if (commit->isSuccess()) {
                copies++;
            } else {
                refusal = commit->getErrorMessage();
                commitRefusals++;
            }
        }
        if (!refusal.empty()) {
            ASSERT_NE(refusal.find("buffer pool is full"), std::string::npos) << refusal;
            refusals++;
            // Le filet du crate (99e6f0ee0) : défaire, fermer sans point de reprise, rouvrir.
            conn->query("ROLLBACK;");
            conn->query("CALL force_checkpoint_on_close=false;");
            conn.reset();
            database.reset();
            createDBAndConn();
            // Le crate ne se replie plus sur une base à rouvrir (99e6f0ee0) : le drain échoue avec
            // sa cause, et le suivant repart sur la base rouverte.
        }
    }
    std::cout << "COPY validés " << copies << ", COPY refusés pour mémoire " << refusals
              << " (dont au point de reprise du COMMIT " << commitRefusals << ")\n";
    ASSERT_GT(refusals, 0) << "aucun COPY n'a été refusé : le tampon n'est pas assez serré";
    sources.insert(0);
    destinations.insert(0);

    EXPECT_EQ(gaps(sources, destinations), 0u) << "base ouverte";
    ok("CHECKPOINT;");
    conn.reset();
    database.reset();
    systemConfig->bufferPoolSize = 2ull * 1024 * 1024 * 1024;
    createDBAndConn();
    EXPECT_EQ(gaps(sources, destinations), 0u) << "base rouverte, tampon large";
}

} // namespace
