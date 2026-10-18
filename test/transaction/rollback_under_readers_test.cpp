// L'annulation d'une insertion sous des lecteurs (tickets du 18 octobre 2026 : « le rollback d'un
// écrivain détruit un bloc en mémoire sous un lecteur », l'hôte de luciepc figé).
//
// Deux défauts, corrigés ensemble :
// - NodeGroup::rollbackInsert et NodeGroupCollection::rollbackInsert détruisaient les blocs et
//   les groupes de queue vidés par l'annulation. Un balayage d'une autre transaction prend le
//   pointeur d'un bloc sous le verrou de la collection, le rend, puis balaie sous inMemAppendMtx
//   partagé : entre les deux, l'annulation libérait le bloc, et le lecteur se figeait sur le
//   verrou de versions d'un objet mort. Désormais rien n'est détruit au rollback : un bloc vide
//   n'est pas plein, le prochain append le remplit, le point de reprise le retire.
// - Les listes CSR en mémoire gardaient les lignes annulées ; l'insertion suivante réutilisait
//   leurs emplacements, et le nœud d'avant lisait les relations d'un autre.
//
// Le chemin : un CREATE dans une transaction vit dans le stockage local jusqu'au COMMIT, son
// annulation ne touche pas les blocs partagés. C'est le COPY, écrit directement dans la table
// (insertion par lots), dont le ROLLBACK passe par NodeGroup::rollbackInsert et les listes CSR.
// Les recettes annulent donc des COPY.

#include <sys/wait.h>
#include <unistd.h>

#include <atomic>
#include <cstdio>
#include <fstream>
#include <memory>
#include <string>
#include <thread>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class RollbackUnderReaders : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
    }

    static void ok(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }
    void ok(const std::string& query) { ok(*conn, query); }

    static int64_t scalar(Connection& connection, const std::string& query) {
        auto result = connection.query(query);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }
    int64_t scalar(const std::string& query) { return scalar(*conn, query); }

    void expectThroughCheckpointAndReopen(const std::string& query, int64_t expected) {
        EXPECT_EQ(scalar(query), expected) << "avant le point de reprise";
        ok("CHECKPOINT;");
        EXPECT_EQ(scalar(query), expected) << "après le point de reprise";
        if (inMemMode) {
            return;
        }
        createDBAndConn();
        EXPECT_EQ(scalar(query), expected) << "après réouverture";
    }

    // Quatre nœuds A et une table R, après un point de reprise : ce qui suit est en mémoire.
    void relSetUp() {
        ok("CREATE NODE TABLE A(id INT64 PRIMARY KEY);");
        ok("CREATE REL TABLE R(FROM A TO A, k INT64);");
        ok("UNWIND range(0, 3) AS i CREATE (:A {id: i});");
        ok("CHECKPOINT;");
    }
    // Un fichier de `count` relations de `from` vers `to`, k = firstK … firstK + count - 1.
    std::string relsCsv(const std::string& name, int64_t from, int64_t to, int64_t firstK,
        int64_t count) {
        const auto path = databasePath + "." + name + ".csv";
        std::ofstream csv(path);
        for (int64_t i = 0; i < count; i++) {
            csv << from << "," << to << "," << firstK + i << "\n";
        }
        return path;
    }
    std::string nodesCsv(const std::string& name, int64_t firstId, int64_t count,
        const std::string& s) {
        const auto path = databasePath + "." + name + ".csv";
        std::ofstream csv(path);
        for (int64_t i = 0; i < count; i++) {
            csv << firstId + i << "," << s << "\n";
        }
        return path;
    }
};

TEST_F(RollbackUnderReaders, ARolledBackRelInsertLeavesNoRowInTheNodesList) {
    relSetUp();
    const auto annulees = relsCsv("annulees", 0, 1, 1, 50);
    ok("BEGIN TRANSACTION;");
    ok("COPY R FROM '" + annulees + "' (header=false);");
    EXPECT_EQ(scalar("MATCH (a:A {id: 0})-[r:R]->() RETURN count(r);"), 50);
    ok("ROLLBACK;");
    EXPECT_EQ(scalar("MATCH (a:A {id: 0})-[r:R]->() RETURN count(r);"), 0);
    // L'insertion suivante réutilise les emplacements annulés : le nœud 0 ne doit rien y lire.
    ok("MATCH (c:A {id: 2}), (d:A {id: 3}) UNWIND range(1, 30) AS i CREATE (c)-[:R {k: 100 + i}]->(d);");
    EXPECT_EQ(scalar("MATCH (c:A {id: 2})-[r:R]->() RETURN count(r);"), 30);
    expectThroughCheckpointAndReopen("MATCH (a:A {id: 0})-[r:R]->() RETURN count(r);", 0);
    EXPECT_EQ(scalar("MATCH ()-[r:R]->() RETURN count(r);"), 30);
    EXPECT_EQ(scalar("MATCH (c:A {id: 2})-[r:R]->() RETURN min(r.k);"), 101);
}

TEST_F(RollbackUnderReaders, ARolledBackFirstRelInsertIsStillInvisibleToACopiedList) {
    // La première insertion dans un bloc neuf, annulée depuis sa ligne 0 : la marque des versions
    // doit rester posée (elle était effacée), sinon une liste copiée avant l'annulation rendait
    // toutes les lignes visibles.
    relSetUp();
    const auto annulees = relsCsv("annulees", 0, 1, 1, 20);
    ok("BEGIN TRANSACTION;");
    ok("COPY R FROM '" + annulees + "' (header=false);");
    ok("ROLLBACK;");
    ok("MATCH (a:A {id: 0}), (b:A {id: 1}) UNWIND range(1, 5) AS i CREATE (a)-[:R {k: 200 + i}]->(b);");
    expectThroughCheckpointAndReopen("MATCH (a:A {id: 0})-[r:R]->() RETURN count(r);", 5);
    EXPECT_EQ(scalar("MATCH (a:A {id: 0})-[r:R]->() RETURN min(r.k);"), 201);
}

TEST_F(RollbackUnderReaders, ARolledBackCopyDoesNotComeBackWithTheNextCopy) {
    // Les relations d'avant en mémoire (pas de point de reprise : avec des relations persistantes,
    // c'est le ticket du COPY annulé qui les efface, à part). Avant le correctif, les 1 500
    // lignes du COPY annulé restaient dans la liste du nœud 0 et le COPY suivant, rangé aux mêmes
    // emplacements, les faisait compter deux fois : 3 010 au lieu de 1 510.
    ok("CREATE NODE TABLE A(id INT64 PRIMARY KEY);");
    ok("CREATE REL TABLE R(FROM A TO A, k INT64);");
    ok("UNWIND range(0, 3) AS i CREATE (:A {id: i});");
    ok("MATCH (a:A {id: 0}), (b:A {id: 1}) UNWIND range(1, 10) AS i CREATE (a)-[:R {k: i}]->(b);");
    const auto annulees = relsCsv("annulees", 0, 2, 1001, 1500);
    const auto gardees = relsCsv("gardees", 0, 2, 5001, 1500);
    ok("BEGIN TRANSACTION;");
    ok("COPY R FROM '" + annulees + "' (header=false);");
    ok("ROLLBACK;");
    EXPECT_EQ(scalar("MATCH (a:A {id: 0})-[r:R]->() RETURN count(r);"), 10);
    ok("COPY R FROM '" + gardees + "' (header=false);");
    EXPECT_EQ(scalar("MATCH (a:A {id: 0})-[r:R]->() RETURN count(r);"), 1510);
    EXPECT_EQ(scalar("MATCH ()-[r:R]->(b:A {id: 1}) RETURN count(r);"), 10);
    EXPECT_EQ(scalar("MATCH ()-[r:R]->() WHERE r.k > 1000 AND r.k <= 2500 RETURN count(r);"), 0);
    expectThroughCheckpointAndReopen("MATCH ()-[r:R]->() RETURN count(r);", 1510);
}

TEST_F(RollbackUnderReaders, ARolledBackNodeInsertKeepsItsBlocksAndTheNextInsertReusesThem) {
    ok("CREATE NODE TABLE T(id INT64 PRIMARY KEY, s STRING);");
    ok("CREATE (:T {id: -1, s: 'avant'});");
    ok("CHECKPOINT;");
    const auto annulees = nodesCsv("annulees", 0, 5000, "annulée");
    ok("BEGIN TRANSACTION;");
    ok("COPY T FROM '" + annulees + "' (header=false);");
    ok("ROLLBACK;");
    EXPECT_EQ(scalar("MATCH (t:T) RETURN count(*);"), 1);
    ok("UNWIND range(0, 9) AS i CREATE (:T {id: i, s: 'gardée'});");
    EXPECT_EQ(scalar("MATCH (t:T) WHERE t.s = 'gardée' RETURN count(*);"), 10);
    EXPECT_EQ(scalar("MATCH (t:T) WHERE t.s = 'annulée' RETURN count(*);"), 0);
    expectThroughCheckpointAndReopen("MATCH (t:T) RETURN count(*);", 11);
    EXPECT_EQ(scalar("MATCH (t:T) RETURN sum(t.id);"), 44);
}

TEST_F(RollbackUnderReaders, ACopyAfterARolledBackInsertLandsAtTheRightOffsets) {
    // Le COPY écrit un bloc entier et le pose directement dans un groupe vide ; après une
    // annulation, le groupe vide garde des blocs : le COPY doit passer par l'append ordinaire.
    ok("CREATE NODE TABLE T(id INT64 PRIMARY KEY, s STRING);");
    const auto annulees = nodesCsv("annulees", 100000, 500, "annulée");
    const auto copie = nodesCsv("copie", 0, 3000, "copie");
    ok("BEGIN TRANSACTION;");
    ok("COPY T FROM '" + annulees + "' (header=false);");
    ok("ROLLBACK;");
    ok("COPY T FROM '" + copie + "' (header=false);");
    EXPECT_EQ(scalar("MATCH (t:T) RETURN count(*);"), 3000);
    EXPECT_EQ(scalar("MATCH (t:T) WHERE t.id = 2999 RETURN count(*);"), 1);
    EXPECT_EQ(scalar("MATCH (t:T) WHERE t.s = 'annulée' RETURN count(*);"), 0);
    expectThroughCheckpointAndReopen("MATCH (t:T) RETURN sum(t.id);", 2999LL * 3000 / 2);
}

// Un lecteur balaie en boucle les relations d'un nœud pendant qu'un autre écrivain insère puis
// annule, dans un processus fils borné par alarm() : un fils tué par le signal (figé sur un bloc
// détruit, ou avorté par ASAN) est un rouge nommé, pas la fin de la passe. Sous
// -fsanitize=address, le défaut d'avant se voit en quelques tours (heap-use-after-free dans
// ChunkedNodeGroup::scan) ; en Release, c'est le figement.
TEST_F(RollbackUnderReaders, AReaderSurvivesTheRollbackOfAnotherWriter) {
    if (inMemMode) {
        GTEST_SKIP() << "on-disk database needed";
    }
    const auto stepsPath = databasePath + ".steps";
    std::remove(stepsPath.c_str());
    // Assez de relations pour remplir plusieurs blocs : ce sont les blocs de queue, vidés par
    // l'annulation, que le rollback détruisait sous le lecteur.
    const auto annulees = relsCsv("annulees", 0, 2, 1001, 5000);
    // Le fils ouvre la base lui-même : celle du parent est fermée d'abord (un seul écrivain par
    // fichier, le verrou est par processus).
    conn.reset();
    database.reset();
    const auto pid = fork();
    if (pid == 0) {
        alarm(120);
        std::ofstream steps(stepsPath);
        Database database(databasePath, *systemConfig);
        Connection setup(&database);
        auto say = [&](Connection& connection, const std::string& query) {
            auto result = connection.query(query);
            if (!result->isSuccess()) {
                steps << "refus « " << result->getErrorMessage() << " » sur " << query
                      << std::endl;
            }
            return std::move(result);
        };
        say(setup, "CREATE NODE TABLE A(id INT64 PRIMARY KEY);");
        say(setup, "CREATE REL TABLE R(FROM A TO A, k INT64);");
        say(setup, "UNWIND range(0, 3) AS i CREATE (:A {id: i});");
        // Les dix relations d'avant restent en mémoire (pas de point de reprise) : un COPY vers un
        // nœud qui a des relations persistantes les fait disparaître, défaut à part (ticket du 18
        // octobre, « un COPY de relations vers un nœud déjà relié perd ses relations
        // persistantes »), qui n'est pas celui que ce témoin juge.
        say(setup, "MATCH (a:A {id: 0}), (b:A {id: 1}) UNWIND range(1, 10) AS i CREATE (a)-[:R {k: i}]->(b);");
        std::atomic<bool> stop{false};
        std::atomic<int64_t> wrongCounts{0};
        std::atomic<int64_t> reads{0};
        std::string wrongValues;
        std::thread reader([&] {
            Connection connection(&database);
            while (!stop.load()) {
                auto result =
                    connection.query("MATCH (a:A {id: 0})-[r:R]->() RETURN count(r);");
                if (result->isSuccess() && result->hasNext()) {
                    const auto count = result->getNext()->getValue(0)->getValue<int64_t>();
                    if (count != 10) {
                        if (wrongCounts < 8) {
                            wrongValues += " " + std::to_string(count);
                        }
                        wrongCounts++;
                    }
                } else if (!result->isSuccess()) {
                    steps << "lecture refusée « " << result->getErrorMessage() << " »" << std::endl;
                }
                reads++;
            }
        });
        Connection writer(&database);
        for (auto round = 0; round < 60; round++) {
            say(writer, "BEGIN TRANSACTION;");
            say(writer, "COPY R FROM '" + annulees + "' (header=false);");
            say(writer, "ROLLBACK;");
        }
        stop.store(true);
        reader.join();
        steps << "READS " << reads.load() << " WRONG " << wrongCounts.load()
              << " (premières valeurs fausses :" << wrongValues << ")" << std::endl;
        _exit(0);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    std::string steps;
    {
        std::ifstream in(stepsPath);
        for (std::string line; std::getline(in, line);) {
            steps += line + "\n";
        }
    }
    EXPECT_FALSE(WIFSIGNALED(status))
        << "[check: reader-survives-rollback] the child died of signal " << WTERMSIG(status)
        << " (alarm = the reader hung on a destroyed block)\n"
        << steps;
    EXPECT_NE(steps.find("WRONG 0"), std::string::npos)
        << "[check: reader-sees-only-committed-rows] " << steps;
    EXPECT_EQ(steps.find("refus"), std::string::npos) << steps;
}

} // namespace
