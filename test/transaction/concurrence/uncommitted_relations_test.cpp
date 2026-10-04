// Un défaut du moteur d'origine, sans index vectoriel ni concurrence, que la mise à jour
// de l'index HNSW a révélé (session cœur C++, 4 octobre ; livré aussi de son côté dans
// test/transaction/uncommitted_rels_scan_test.cpp). Un seul écrivain.
//
// Une transaction supprime des relations d'un nœud qui sont sur disque (un CHECKPOINT
// après leur création : en mémoire, le défaut ne se voit pas), en crée de nouvelles depuis
// ce même nœud, puis relit : elle reçoit la même relation locale répétée à la place des
// nouvelles. Si elle écrit d'après ce qu'elle a relu, la base reste fausse après le
// COMMIT. Cause désignée par le cœur C++ : LocalRelTable::scan ; le même défaut est
// corrigé chez Ladybug (ffe855873, 23 avril 2026).
//
// Rouges trois passes sur trois avant c8fdaf196, verts trois passes sur trois après
// (4 octobre) : ils restent comme témoins verts.

#ifndef __SINGLE_THREADED__

#include <algorithm>
#include <vector>

#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "integrity/integrity_checker.h"
#include "processor/result/flat_tuple.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

class UncommittedRelations : public EmptyDBTest {
public:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed";
        }
        createDBAndConn();
        mustRun("CALL auto_checkpoint=false;");
    }

    void mustRun(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << "[check: query] " << query << "\n"
                                         << result->getErrorMessage();
    }

    // 200 nœuds ; le nœud 0 relié aux nœuds 1 à 60 ; sur disque si onDisk.
    void createHub(bool onDisk) {
        mustRun("CREATE NODE TABLE N(id INT64 PRIMARY KEY);");
        mustRun("CREATE REL TABLE E(FROM N TO N);");
        mustRun("UNWIND range(0, 199) AS i CREATE (:N {id: i});");
        mustRun("MATCH (a:N {id: 0}), (b:N) WHERE b.id >= 1 AND b.id <= 60 CREATE (a)-[:E]->(b);");
        if (onDisk) {
            mustRun("CHECKPOINT;");
        }
    }

    void deleteEvenAndCreateNew() {
        mustRun("MATCH (a:N {id: 0})-[r:E]->(b:N) WHERE b.id % 2 = 0 DELETE r;");
        mustRun("MATCH (a:N {id: 0}), (b:N) WHERE b.id >= 100 AND b.id <= 158 CREATE (a)-[:E]->(b);");
    }

    std::vector<int64_t> hubTargets() {
        std::vector<int64_t> targets;
        auto result = conn->query("MATCH (a:N {id: 0})-[:E]->(b:N) RETURN b.id;");
        EXPECT_TRUE(result->isSuccess()) << "[check: query] " << result->getErrorMessage();
        while (result->isSuccess() && result->hasNext()) {
            targets.push_back(result->getNext()->getValue(0)->getValue<int64_t>());
        }
        std::sort(targets.begin(), targets.end());
        return targets;
    }

    // Les impairs de 1 à 59, puis 100 à 158, chacun une fois.
    static std::vector<int64_t> expectedTargets() {
        std::vector<int64_t> expected;
        for (auto i = 1; i <= 59; i += 2) {
            expected.push_back(i);
        }
        for (auto i = 100; i <= 158; ++i) {
            expected.push_back(i);
        }
        return expected;
    }

    static std::string show(const std::vector<int64_t>& ids) {
        std::string text;
        for (auto id : ids) {
            text += std::to_string(id) + " ";
        }
        return text;
    }

    void expectIntegrity() {
        const auto violations = integrity::checkLevel1(*conn);
        std::string checks;
        for (const auto& violation : violations) {
            checks += "[check: " + violation.invariant + "] ";
        }
        EXPECT_TRUE(violations.empty()) << checks << integrity::describe(violations);
    }
};

TEST_F(UncommittedRelations, TransactionReadsItsNewRelations) {
    createHub(true /* onDisk */);
    mustRun("BEGIN TRANSACTION;");
    deleteEvenAndCreateNew();
    const auto inTransaction = hubTargets();
    EXPECT_EQ(inTransaction, expectedTargets())
        << "[check: transaction-reads-its-relations] read " << show(inTransaction);
    mustRun("COMMIT;");
    const auto committed = hubTargets();
    EXPECT_EQ(committed, expectedTargets())
        << "[check: committed-relations] read " << show(committed);
    expectIntegrity();
}

// Sans lecture dans la transaction : ce qui est validé.
TEST_F(UncommittedRelations, CommittedRelationsAfterDeleteAndCreate) {
    createHub(true /* onDisk */);
    mustRun("BEGIN TRANSACTION;");
    deleteEvenAndCreateNew();
    mustRun("COMMIT;");
    const auto committed = hubTargets();
    EXPECT_EQ(committed, expectedTargets())
        << "[check: committed-relations] read " << show(committed);
    expectIntegrity();
}

// L'écriture qui suit la lecture de travers : la suppression des nouvelles relations en
// laisse.
TEST_F(UncommittedRelations, DeleteTheNewRelationsInTheSameTransaction) {
    createHub(true /* onDisk */);
    mustRun("BEGIN TRANSACTION;");
    deleteEvenAndCreateNew();
    mustRun("MATCH (a:N {id: 0})-[r:E]->(b:N) WHERE b.id >= 100 DELETE r;");
    mustRun("COMMIT;");
    std::vector<int64_t> expected;
    for (auto i = 1; i <= 59; i += 2) {
        expected.push_back(i);
    }
    const auto committed = hubTargets();
    EXPECT_EQ(committed, expected) << "[check: deleted-relations-gone] read " << show(committed);
    expectIntegrity();
}

// L'écriture qui suit la lecture de travers : un MERGE des mêmes relations les double.
TEST_F(UncommittedRelations, MergeTheNewRelationsAgain) {
    createHub(true /* onDisk */);
    mustRun("BEGIN TRANSACTION;");
    deleteEvenAndCreateNew();
    mustRun("UNWIND range(100, 158) AS i MATCH (a:N {id: 0}), (b:N {id: i}) MERGE (a)-[:E]->(b);");
    mustRun("COMMIT;");
    const auto committed = hubTargets();
    EXPECT_EQ(committed, expectedTargets())
        << "[check: merge-no-duplicate] read " << show(committed);
    expectIntegrity();
}

// Garde-fou vert : les mêmes écritures sur des relations encore en mémoire.
TEST_F(UncommittedRelations, InMemoryRelationsReadBack) {
    createHub(false /* onDisk */);
    mustRun("BEGIN TRANSACTION;");
    deleteEvenAndCreateNew();
    const auto inTransaction = hubTargets();
    EXPECT_EQ(inTransaction, expectedTargets())
        << "[check: transaction-reads-its-relations] read " << show(inTransaction);
    mustRun("COMMIT;");
    EXPECT_EQ(hubTargets(), expectedTargets()) << "[check: committed-relations] ";
}

// Garde-fou vert : un MERGE par lot dont toutes les lignes partent du même nœud, sans
// suppression.
TEST_F(UncommittedRelations, MergeBatchFromTheSameNodeWithoutDelete) {
    createHub(true /* onDisk */);
    mustRun("BEGIN TRANSACTION;");
    mustRun("UNWIND range(100, 158) AS i MATCH (a:N {id: 0}), (b:N {id: i}) MERGE (a)-[:E]->(b);");
    mustRun("COMMIT;");
    std::vector<int64_t> expected;
    for (auto i = 1; i <= 60; ++i) {
        expected.push_back(i);
    }
    for (auto i = 100; i <= 158; ++i) {
        expected.push_back(i);
    }
    const auto committed = hubTargets();
    EXPECT_EQ(committed, expected) << "[check: merge-no-duplicate] read " << show(committed);
    expectIntegrity();
}

} // namespace

// Les propriétés d'une relation se lisent pareil dans les deux sens, après toute suite
// d'écritures sur des relations (demandé par l'orchestration, 4 octobre au soir). Constat de
// la session de l'arbre principal : dans une base de reprise de rag3weaver, 140 relations
// CONSUMES, et autant de leur réciproque CONSUMED_BY, portent deux valeurs de `resolution`
// selon le sens de stockage ; le contrôle de niveau 2 (stored-rel-properties-agree) les
// trouve. Intermittent, recette inconnue. Ce témoin rejoue la forme des écritures du produit
// (clés INT64 ici, pour que le niveau 2 éprouve aussi l'index) : des nœuds sur disque, la
// suppression d'un fichier, des nœuds neufs, puis deux lots MERGE … SET sur deux tables
// réciproques ; il vérifie l'accord dans le processus écrivain, puis après réouverture.
class RelationPropertiesBothWays : public UncommittedRelations {
public:
    static std::string batch(const std::string& rel, int64_t first, int64_t last,
        const std::string& resolution, bool reverse) {
        std::string items;
        for (auto i = first; i < last; ++i) {
            const auto from = reverse ? i : i + 1;
            const auto to = reverse ? i + 1 : i;
            items += std::string(items.empty() ? "" : ", ") + "{from_id: " +
                     std::to_string(from) + ", to_id: " + std::to_string(to) +
                     ", line: 4, resolution: '" + resolution + "'}";
        }
        return "UNWIND [" + items +
               "] AS item WITH item, item.from_id AS k0 MATCH (a:Scope {id: k0}) WITH item, a, "
               "item.to_id AS k1 MATCH (b:Scope {id: k1}) MERGE (a)-[r:" +
               rel + "]->(b) SET r.line = item.line, r.resolution = item.resolution;";
    }

    void expectBothWaysAgree(const std::string& when) {
        const auto violations = integrity::checkLevel2(*conn);
        std::string checks;
        for (const auto& violation : violations) {
            checks += "[check: " + violation.invariant + "] ";
        }
        std::cerr << integrity::describe(violations);
        EXPECT_TRUE(violations.empty()) << when << ": " << checks;
    }

    // checkpoints : 0 aucun ; 1 entre les deux lots ; 2 seuil automatique minuscule.
    void run(int checkpoints) {
        if (checkpoints == 2) {
            mustRun("CALL auto_checkpoint=true;");
            mustRun("CALL checkpoint_threshold=4096;");
        }
        mustRun("CREATE NODE TABLE Scope(id INT64 PRIMARY KEY, name STRING);");
        mustRun("CREATE REL TABLE CONSUMES(FROM Scope TO Scope, line INT64, resolution STRING);");
        mustRun("CREATE REL TABLE CONSUMED_BY(FROM Scope TO Scope, line INT64, resolution STRING);");
        mustRun("UNWIND range(0, 2398) AS i CREATE (:Scope {id: i, name: 'old'});");
        mustRun("MATCH (a:Scope), (b:Scope) WHERE a.id = b.id + 1 CREATE (a)-[:CONSUMES {line: 1, "
                "resolution: 'fichier'}]->(b), (b)-[:CONSUMED_BY {line: 1, resolution: "
                "'fichier'}]->(a);");
        mustRun("CHECKPOINT;");
        // Un fichier réindexé : ses portées supprimées, de nouvelles créées.
        mustRun("MATCH (s:Scope) WHERE s.id >= 2318 DETACH DELETE s;");
        mustRun("UNWIND range(2399, 2559) AS i MERGE (s:Scope {id: i}) SET s.name = 'new';");
        mustRun(batch("CONSUMES", 2399, 2559, "fichier", false));
        mustRun(batch("CONSUMED_BY", 2399, 2559, "fichier", true));
        if (checkpoints == 1) {
            mustRun("CHECKPOINT;");
        }
        mustRun(batch("CONSUMES", 2399, 2539, "nom", false));
        mustRun(batch("CONSUMED_BY", 2399, 2539, "nom", true));
        expectBothWaysAgree("in the writing process");
        conn.reset();
        database.reset();
        createDBAndConn();
        expectBothWaysAgree("after reopening");
        auto count = conn->query("MATCH (a:Scope)-[r:CONSUMES]->(b:Scope) WHERE r.resolution = "
                                 "'nom' RETURN count(*);");
        ASSERT_TRUE(count->isSuccess());
        EXPECT_EQ(count->getNext()->getValue(0)->getValue<int64_t>(), 140)
            << "[check: second-batch-applied] ";
    }
};

TEST_F(RelationPropertiesBothWays, TwoMergeSetBatchesWithoutCheckpoint) {
    run(0);
}

TEST_F(RelationPropertiesBothWays, TwoMergeSetBatchesWithACheckpointBetween) {
    run(1);
}

TEST_F(RelationPropertiesBothWays, TwoMergeSetBatchesUnderFrequentAutomaticCheckpoints) {
    run(2);
}

// La recette trouvée par la session cœur C++ (ticket
// propriete-de-chaine-faussee-dans-le-sens-direct) : deux générations de chaînes, une
// mise à jour par lot, un point de reprise. Le point de reprise relisait chaque région
// réécrite et y échangeait les chaînes des lignes qu'aucune écriture n'avait touchées.
// Comparé par le stockage, sens par sens, avant et après le point de reprise : aucune
// forme de requête n'y sert de référence. Vu rouge le 4 octobre 2026 sur master avant
// 25b3b45dc (561 valeurs changées), vert avec le correctif.
TEST_F(RelationPropertiesBothWays, CheckpointKeepsEveryStoredString) {
    mustRun("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);");
    mustRun("CREATE REL TABLE M(FROM Doc TO Doc, res STRING, line INT64);");
    mustRun("UNWIND range(0, 2399) AS i CREATE (:Doc {id: i, name: 'ancien'});");
    mustRun("UNWIND range(0, 1999) AS i MATCH (a:Doc {id: i}) WITH a, i MATCH (b:Doc {id: (i * "
            "11) % 2400}) CREATE (a)-[:M {res: 'vieux', line: -1}]->(b);");
    mustRun("CHECKPOINT;");
    mustRun("UNWIND range(2400, 2599) AS i CREATE (:Doc {id: i, name: 'neuf'});");
    mustRun("UNWIND range(0, 2029) AS i MATCH (a:Doc {id: i % 2400}) WITH a, i MATCH (b:Doc {id: "
            "(i * 7 + 3) % 2400}) CREATE (a)-[:M {res: 'fichier', line: i}]->(b);");
    mustRun("UNWIND range(0, 139) AS i MATCH (a:Doc {id: 2400 + i}) WITH a, i MATCH (b:Doc {id: "
            "2400 + (i * 3 + 1) % 60}) CREATE (a)-[:M {res: 'fichier', line: 5000 + i}]->(b);");
    mustRun("CHECKPOINT;");
    mustRun("UNWIND range(0, 139) AS i MATCH (a:Doc {id: 2400 + i}) WITH a, i MATCH (b:Doc {id: "
            "2400 + (i * 3 + 1) % 60}) MERGE (a)-[r:M]->(b) SET r.res = 'nom';");
    const auto before = integrity::storedRelProperties(*conn);
    mustRun("CHECKPOINT;");
    const auto after = integrity::storedRelProperties(*conn);
    std::string changed;
    size_t numChanged = 0;
    for (const auto& [key, value] : before) {
        const auto it = after.find(key);
        if ((it == after.end() || it->second != value) && ++numChanged <= 5) {
            changed += " " + key + ": '" + value + "' -> '" +
                       (it == after.end() ? std::string("<missing>") : it->second) + "'";
        }
    }
    EXPECT_EQ(numChanged, 0u) << "[check: checkpoint-keeps-rel-properties] " << numChanged
                              << " stored relation values changed:" << changed;
    EXPECT_EQ(before.size(), after.size()) << "[check: checkpoint-keeps-rel-properties] ";
    expectBothWaysAgree("after the checkpoint");
}

// L'appelant de plus, signalé à la session cœur C++ : une colonne LIST<STRING> relit sa
// colonne de données sur la plage de ses listes, et liste par liste quand les décalages ne
// sont plus croissants (des listes réécrites en place). Deux générations de chaînes très
// dupliquées, des listes réécrites entre deux points de reprise, puis la comparaison avant
// et après le dernier point de reprise ; sur des nœuds (lus par Cypher, un seul rangement)
// et sur des relations (par le stockage, sens par sens).
class ListOfStringsAcrossCheckpoints : public RelationPropertiesBothWays {
public:
    // Une ligne par nœud, triées pour compareDumps.
    std::vector<std::string> nodeLists() {
        auto result = conn->query("MATCH (d:Doc) RETURN d.id, d.tags;");
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        std::vector<std::string> lines;
        while (result->isSuccess() && result->hasNext()) {
            lines.push_back(result->getNext()->toString());
        }
        std::sort(lines.begin(), lines.end());
        return lines;
    }

    // Jamais EXPECT_EQ sur les deux vidages : rouge, gtest en calcule le diff, quadratique
    // sur 400 000 lignes (la panne de mémoire du 4 octobre à 22 h 51). Comparés par ==,
    // puis un résumé borné : le nombre de lignes de chaque côté et les premières.
    void expectSameLists(const std::vector<std::string>& before,
        const std::vector<std::string>& after, const std::string& check) {
        if (before == after) {
            return;
        }
        ADD_FAILURE() << "[check: " << check << "] "
                      << integrity::describe(integrity::compareDumps(before, after,
                             "before the checkpoint", "after"));
    }

    void prepare() {
        mustRun("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, tags STRING[]);");
        mustRun("CREATE REL TABLE M(FROM Doc TO Doc, tags STRING[]);");
        mustRun("UNWIND range(0, 2399) AS i CREATE (:Doc {id: i, tags: ['vieux', 'vieux']});");
        mustRun("UNWIND range(0, 1999) AS i MATCH (a:Doc {id: i}), (b:Doc {id: (i * 11) % 2400}) "
                "CREATE (a)-[:M {tags: ['vieux', 'vieux']}]->(b);");
        mustRun("CHECKPOINT;");
        // Des listes réécrites en place, de longueurs différentes : leurs données vont
        // ailleurs, et les décalages ne sont plus croissants.
        mustRun("MATCH (d:Doc) WHERE d.id % 3 = 0 SET d.tags = ['fichier', 'nom', 'fichier'];");
        mustRun("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE a.id % 3 = 0 SET r.tags = ['fichier', 'nom', "
                "'fichier'];");
        mustRun("CHECKPOINT;");
        mustRun("MATCH (d:Doc) WHERE d.id % 7 = 0 SET d.tags = ['nom'];");
        mustRun("MATCH (a:Doc)-[r:M]->(b:Doc) WHERE a.id % 7 = 0 SET r.tags = ['nom'];");
    }
};

// Les nœuds ne s'y prennent qu'en nombre (forme trouvée par la session cœur C++) :
// 400 000 lignes à listes alternées, une mise à jour sur 500, un point de reprise.
TEST_F(ListOfStringsAcrossCheckpoints, NodeListsKeepTheirStrings) {
    mustRun("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, tags STRING[]);");
    // Par lots : une seule transaction de 400 000 lignes remplit le tampon des tests.
    for (auto first = 0; first < 400000; first += 50000) {
        mustRun("UNWIND range(" + std::to_string(first) + ", " + std::to_string(first + 49999) +
                ") AS i CREATE (:Doc {id: i, tags: CASE WHEN i % 2 = 0 THEN ['vieux', 'vieux'] "
                "ELSE ['fichier', 'vieux'] END});");
    }
    mustRun("CHECKPOINT;");
    mustRun("MATCH (d:Doc) WHERE d.id % 500 = 0 SET d.tags = ['nom'];");
    const auto before = nodeLists();
    mustRun("CHECKPOINT;");
    expectSameLists(before, nodeLists(), "checkpoint-keeps-node-lists");
    conn.reset();
    database.reset();
    createDBAndConn();
    expectSameLists(before, nodeLists(), "reopen-keeps-node-lists");
}

TEST_F(ListOfStringsAcrossCheckpoints, RelationListsKeepTheirStrings) {
    prepare();
    const auto before = integrity::storedRelProperties(*conn);
    mustRun("CHECKPOINT;");
    const auto after = integrity::storedRelProperties(*conn);
    size_t numChanged = 0;
    std::string changed;
    for (const auto& [key, value] : before) {
        const auto it = after.find(key);
        if ((it == after.end() || it->second != value) && ++numChanged <= 5) {
            changed += " " + key + ": '" + value + "' -> '" +
                       (it == after.end() ? std::string("<missing>") : it->second) + "'";
        }
    }
    EXPECT_EQ(numChanged, 0u) << "[check: checkpoint-keeps-rel-properties] " << numChanged
                              << " stored relation values changed:" << changed;
    expectBothWaysAgree("after the checkpoint");
}

#endif
