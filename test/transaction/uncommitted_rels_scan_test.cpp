// Relire ses propres écritures : dans une transaction, les relations d'un nœud dont on vient
// de supprimer des relations validées et d'en créer de nouvelles.
//
// Le balayage d'un nœud lit d'abord ses relations validées, puis celles de la transaction.
// Les deux écrivent dans les mêmes vecteurs de sortie, avec le même vecteur de sélection. Le
// premier le laisse filtré quand la transaction a supprimé certaines des relations validées ;
// le second n'en changeait que la taille, et écrivait chaque relation locale à une position
// périmée. Le lecteur recevait alors le même voisin répété, ou des voisins d'avant.
// Le défaut passe par l'interface de graphe (graph::OnDiskGraph), celle de l'index vectoriel
// et des algorithmes de graphe ; la lecture d'une relation par un MATCH ordinaire n'est pas
// touchée. C'est par l'index vectoriel, qui relit ses arêtes pendant une mise à jour, qu'il a
// été trouvé (4 octobre 2026). Chaque cas est joué par les deux chemins.

#include <set>
#include <string>

#include "catalog/catalog.h"
#include "catalog/catalog_entry/rel_group_catalog_entry.h"
#include "graph/graph_entry.h"
#include "graph/on_disk_graph.h"
#include "graph_test/private_graph_test.h"
#include "main/client_context.h"
#include "main/connection.h"
#include "transaction/transaction.h"

using namespace rag3db::testing;

namespace {

class UncommittedRelsScanTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        createDBAndConn();
        ok("CREATE NODE TABLE N(id INT64 PRIMARY KEY);");
        ok("CREATE REL TABLE E(FROM N TO N);");
        ok("UNWIND range(0, 199) AS i CREATE (:N {id: i});");
        // Le nœud 0 mène aux nœuds 1 à 60, validés.
        ok("MATCH (a:N {id: 0}), (b:N) WHERE b.id >= 1 AND b.id <= 60 CREATE (a)-[:E]->(b);");
        // Sur disque : c'est le balayage des relations écrites par un point de reprise qui
        // laisse le vecteur de sélection filtré quand certaines ont été supprimées.
        ok("CHECKPOINT;");
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    std::multiset<int64_t> neighboursOfZero() {
        std::multiset<int64_t> found;
        auto result = conn->query("MATCH (a:N {id: 0})-[:E]->(b:N) RETURN b.id;");
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        while (result->isSuccess() && result->hasNext()) {
            found.insert(result->getNext()->getValue(0)->getValue<int64_t>());
        }
        return found;
    }

    // Les mêmes voisins, lus par l'interface de graphe, dans la transaction en cours.
    std::multiset<int64_t> neighboursOfZeroThroughTheGraph() {
        auto context = conn->getClientContext();
        auto transaction = rag3db::transaction::Transaction::Get(*context);
        auto catalog = rag3db::catalog::Catalog::Get(*context);
        auto nodeEntry = catalog->getTableCatalogEntry(transaction, "N");
        auto relEntry = catalog->getTableCatalogEntry(transaction, "E");
        const auto relTableID =
            relEntry->constCast<rag3db::catalog::RelGroupCatalogEntry>().getSingleRelEntryInfo().oid;
        rag3db::graph::NativeGraphEntry graphEntry{{nodeEntry}, {relEntry}};
        rag3db::graph::OnDiskGraph graph{context, std::move(graphEntry)};
        // Le décalage du nœud 0 : c'est la première ligne de la table.
        const auto scanState = graph.prepareRelScan(*relEntry, relTableID, nodeEntry->getTableID(), {});
        std::multiset<int64_t> found;
        for (const auto& chunk :
            graph.scanFwd(rag3db::common::nodeID_t{0, nodeEntry->getTableID()}, *scanState)) {
            chunk.forEach([&](auto neighbours, auto, auto i) {
                found.insert(static_cast<int64_t>(neighbours[i].offset));
            });
        }
        return found;
    }

    static std::multiset<int64_t> range(int64_t first, int64_t last) {
        std::multiset<int64_t> expected;
        for (auto id = first; id <= last; ++id) {
            expected.insert(id);
        }
        return expected;
    }
};

// Une partie des relations validées supprimée, puis de nouvelles créées, dans la même
// transaction : le nœud doit rendre les survivantes et les nouvelles, chacune une fois.
TEST_F(UncommittedRelsScanTest, SomeCommittedRelsDeletedThenNewOnesCreated) {
    ok("BEGIN TRANSACTION;");
    ok("MATCH (a:N {id: 0})-[r:E]->(b:N) WHERE b.id % 2 = 0 DELETE r;");
    ok("MATCH (a:N {id: 0}), (b:N) WHERE b.id >= 100 AND b.id <= 158 CREATE (a)-[:E]->(b);");
    auto expected = range(100, 158);
    for (int64_t id = 1; id <= 60; id += 2) {
        expected.insert(id);
    }
    EXPECT_EQ(neighboursOfZero(), expected);
    EXPECT_EQ(neighboursOfZeroThroughTheGraph(), expected);
    ok("COMMIT;");
    EXPECT_EQ(neighboursOfZero(), expected);
}

// Toutes les relations validées supprimées, puis de nouvelles créées.
TEST_F(UncommittedRelsScanTest, AllCommittedRelsDeletedThenNewOnesCreated) {
    ok("BEGIN TRANSACTION;");
    ok("MATCH (a:N {id: 0})-[r:E]->(b:N) DELETE r;");
    ok("MATCH (a:N {id: 0}), (b:N) WHERE b.id >= 100 AND b.id <= 158 CREATE (a)-[:E]->(b);");
    EXPECT_EQ(neighboursOfZero(), range(100, 158));
    EXPECT_EQ(neighboursOfZeroThroughTheGraph(), range(100, 158));
    ok("COMMIT;");
    EXPECT_EQ(neighboursOfZero(), range(100, 158));
}

// Les nouvelles d'abord, les suppressions ensuite.
TEST_F(UncommittedRelsScanTest, NewRelsCreatedThenSomeCommittedOnesDeleted) {
    ok("BEGIN TRANSACTION;");
    ok("MATCH (a:N {id: 0}), (b:N) WHERE b.id >= 100 AND b.id <= 158 CREATE (a)-[:E]->(b);");
    ok("MATCH (a:N {id: 0})-[r:E]->(b:N) WHERE b.id <= 60 AND b.id % 3 = 0 DELETE r;");
    auto expected = range(100, 158);
    for (int64_t id = 1; id <= 60; ++id) {
        if (id % 3 != 0) {
            expected.insert(id);
        }
    }
    EXPECT_EQ(neighboursOfZero(), expected);
    EXPECT_EQ(neighboursOfZeroThroughTheGraph(), expected);
    ok("COMMIT;");
    EXPECT_EQ(neighboursOfZero(), expected);
}

// Une écriture fondée sur cette lecture : supprimer, dans la même transaction, les relations
// qu'on vient de créer. Une lecture fausse ferait supprimer la même relation plusieurs fois
// et laisserait les autres.
TEST_F(UncommittedRelsScanTest, ADeleteThatReadsTheNewRelsDeletesEachOfThem) {
    ok("BEGIN TRANSACTION;");
    ok("MATCH (a:N {id: 0})-[r:E]->(b:N) WHERE b.id % 2 = 0 DELETE r;");
    ok("MATCH (a:N {id: 0}), (b:N) WHERE b.id >= 100 AND b.id <= 158 CREATE (a)-[:E]->(b);");
    ok("MATCH (a:N {id: 0})-[r:E]->(b:N) WHERE b.id >= 100 DELETE r;");
    ok("COMMIT;");
    std::multiset<int64_t> expected;
    for (int64_t id = 1; id <= 60; id += 2) {
        expected.insert(id);
    }
    EXPECT_EQ(neighboursOfZero(), expected);
}

// Un MERGE fondé sur cette lecture : les relations qu'on vient de créer existent déjà, il ne
// doit rien ajouter.
TEST_F(UncommittedRelsScanTest, AMergeThatReadsTheNewRelsCreatesNoDuplicate) {
    ok("BEGIN TRANSACTION;");
    ok("MATCH (a:N {id: 0})-[r:E]->(b:N) WHERE b.id % 2 = 0 DELETE r;");
    ok("MATCH (a:N {id: 0}), (b:N) WHERE b.id >= 100 AND b.id <= 158 CREATE (a)-[:E]->(b);");
    ok("MATCH (a:N {id: 0}), (b:N) WHERE b.id >= 100 AND b.id <= 158 MERGE (a)-[:E]->(b);");
    ok("COMMIT;");
    auto expected = range(100, 158);
    for (int64_t id = 1; id <= 60; id += 2) {
        expected.insert(id);
    }
    EXPECT_EQ(neighboursOfZero(), expected);
}

// Sans aucune suppression : un MERGE par lot dont plusieurs lignes partent du même nœud, déjà
// sur disque. Chaque ligne doit voir les relations que les lignes d'avant viennent de créer.
// C'est la forme de rag3weaver (une instruction, validée seule).
TEST_F(UncommittedRelsScanTest, ABatchMergeWithoutAnyDeleteSeesItsOwnRels) {
    ok("UNWIND [100, 101, 100, 102, 101, 100, 103] AS x MATCH (a:N {id: 0}), (b:N {id: x}) "
       "MERGE (a)-[:E]->(b);");
    auto expected = range(1, 60);
    for (const int64_t id : {100, 101, 102, 103}) {
        expected.insert(id);
    }
    EXPECT_EQ(neighboursOfZero(), expected);
    // Et une seconde fois, les relations étant maintenant validées en mémoire.
    ok("UNWIND [100, 104, 104, 59, 60] AS x MATCH (a:N {id: 0}), (b:N {id: x}) "
       "MERGE (a)-[:E]->(b);");
    expected.insert(104);
    EXPECT_EQ(neighboursOfZero(), expected);
}

} // namespace
