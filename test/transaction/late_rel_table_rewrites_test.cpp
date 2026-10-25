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

#include <atomic>
#include <map>
#include <mutex>
#include <random>
#include <set>
#include <string>
#include <thread>
#include <vector>

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

} // namespace
