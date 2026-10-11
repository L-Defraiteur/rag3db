// Les fils d'une tâche proportionnés à la cardinalité estimée par le planificateur (page
// coeur-cpp/12, ticket « les fils d'un plan sont proportionnés à la cardinalité estimée »).
//
// Un pipeline qui lit trois lignes recevait autant de fils que de cœurs ; la mesure de F (impact :
// −47 % à un fil) et la trace par tâche (91 % du temps dans les tâches à 32 fils, 63 % d'entre
// elles à cardinalité estimée ≤ 2 048) ont fixé la règle : un fil par tranche de 2 048 lignes
// estimées le long du pipeline, sous le plafond `threads` de la connexion, le plafond gardé quand
// un opérateur n'a pas d'estimation.
//
// Le témoin lit la trace par tâche (RAG3DB_TRACE_TASKS, posée ici par setenv avant d'ouvrir la
// base : le processeur la lit à son ouverture), l'erreur standard étant détournée vers un fichier
// le temps des requêtes. Il compte, pour chaque balayage, les fils donnés.

#ifndef _WIN32

#include <fcntl.h>
#include <unistd.h>

#include <cstdlib>
#include <fstream>
#include <regex>
#include <sstream>

#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::main;
using namespace rag3db::testing;

namespace {

class ThreadsByCardinality : public EmptyDBTest {
protected:
    void SetUp() override {
        setenv("RAG3DB_TRACE_TASKS", "1", 1 /* overwrite */);
        EmptyDBTest::SetUp();
        createDBAndConn();
        tracePath = databasePath + ".trace.txt";
    }

    void TearDown() override {
        std::filesystem::remove(tracePath);
        EmptyDBTest::TearDown();
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    // Joue la requête avec l'erreur standard détournée vers le fichier de trace, et rend les
    // lignes « [tache] … ».
    std::vector<std::string> traced(const std::string& query) {
        fflush(stderr);
        const int saved = dup(STDERR_FILENO);
        const int fd = open(tracePath.c_str(), O_WRONLY | O_CREAT | O_TRUNC, 0644);
        dup2(fd, STDERR_FILENO);
        close(fd);
        auto result = conn->query(query);
        fflush(stderr);
        dup2(saved, STDERR_FILENO);
        close(saved);
        EXPECT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
        std::vector<std::string> lines;
        std::ifstream in(tracePath);
        std::string line;
        while (std::getline(in, line)) {
            if (line.rfind("[tache]", 0) == 0) {
                lines.push_back(line);
            }
        }
        return lines;
    }

    // Les fils donnés (« fils=<venus>/<donnés> ») de la première tâche dont la source est `source`.
    static int64_t threadsGivenTo(const std::vector<std::string>& lines, const std::string& source) {
        const std::regex rx("source=" + source + " .*fils=([0-9]+)/([0-9]+)");
        for (const auto& line : lines) {
            std::smatch m;
            if (std::regex_search(line, m, rx)) {
                return std::stoll(m[2]);
            }
        }
        return -1;
    }

    static std::string joined(const std::vector<std::string>& lines) {
        if (lines.empty()) {
            return "(no trace)";
        }
        std::string out;
        for (const auto& line : lines) {
            out += "\n    " + line;
        }
        return out;
    }

    std::string tracePath;
};

} // namespace

// Trois lignes : le balayage de la table reçoit un seul fil ; trois cent mille : le plafond de la
// connexion (ici posé à 8, pour ne pas dépendre des cœurs du poste). Et un `threads=1` explicite
// reste 1 sur la grande table : la règle ne fait que réduire.
TEST_F(ThreadsByCardinality, AScanOfThreeRowsGetsOneThreadAndALargeOneTheCap) {
    ok("CALL threads=8;");
    ok("CREATE NODE TABLE Small(id INT64 PRIMARY KEY, v INT64);");
    ok("CREATE NODE TABLE Large(id INT64 PRIMARY KEY, v INT64);");
    ok("UNWIND range(0, 2) AS i CREATE (:Small {id: i, v: i});");
    ok("UNWIND range(0, 299999) AS i CREATE (:Large {id: i, v: i});");
    ok("CHECKPOINT;");
    const auto small = traced("MATCH (n:Small) RETURN count(n);");
    EXPECT_EQ(threadsGivenTo(small, "SCAN_NODE_TABLE"), 1)
        << "[check: three-rows-one-thread] " << joined(small);
    const auto large = traced("MATCH (n:Large) RETURN count(n);");
    EXPECT_EQ(threadsGivenTo(large, "SCAN_NODE_TABLE"), 8)
        << "[check: large-table-cap] " << joined(large);
    ok("CALL threads=1;");
    const auto one = traced("MATCH (n:Large) RETURN count(n);");
    EXPECT_EQ(threadsGivenTo(one, "SCAN_NODE_TABLE"), 1)
        << "[check: explicit-one-thread-kept] " << joined(one);
}

// L'UNWIND d'une liste écrite dans la requête (un appel de LIST_CREATION, comme `UNWIND $uuids`
// est un paramètre) : sa longueur est connue à la liaison, la tâche reçoit un fil.
TEST_F(ThreadsByCardinality, AnUnwindOfAParameterListGetsOneThread) {
    ok("CALL threads=8;");
    ok("CREATE NODE TABLE Small(id INT64 PRIMARY KEY, v INT64);");
    ok("UNWIND range(0, 9) AS i CREATE (:Small {id: i, v: i});");
    const auto literal = traced("UNWIND [1, 2, 3] AS x MATCH (n:Small {id: x}) RETURN n.v;");
    EXPECT_EQ(threadsGivenTo(literal, "TABLE_FUNCTION_CALL"), 1)
        << "[check: literal-list-one-thread] " << joined(literal);
}

#endif
