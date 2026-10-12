// Le balayage sous un petit masque (ticket 2026-10-11-un-saut-par-semi-masque-lit-toute-la-table) :
// un saut vers une grande table de nœuds, dont le masque ne retient que k lignes éparpillées, lit
// toute la table. Sur l'index réel de rag3db, SCAN_NODE_TABLE Scope sortait 71 657 lignes pour 117
// retenues par le SEMI_MASKER (le saut d'`usages`).
//
// - MaskedScan.ReadsAboutTheMaskNotTheTable : le témoin. N = 72 000 Scope et 10 000 Symbols, trois
//   MENTIONS par Scope (216 000), et le Symbol 's' désigné par k = 117 Scope éparpillés ; la
//   requête du saut ; PROFILE donne les lignes sorties du
//   SCAN_NODE_TABLE sous le masque, qui doivent rester sous 10·k. Rouge aujourd'hui (on attend
//   environ N), avec le chiffre au message.
// - MaskedScan.MeasureAtOneAndThirtyTwoThreads : la mesure de référence pour le lot des fils du
//   cœur C++, la même requête à CALL threads=1 puis 32, lignes sorties et durée médiane sur cinq
//   passes. Sautée sans CONCURRENCE_MEASURE=1 ; à jouer sous `poste mesure`.

#include <algorithm>
#include <cctype>
#include <chrono>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <optional>
#include <sstream>
#include <string>
#include <vector>

#include "common/string_format.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"

using namespace rag3db::common;
using namespace rag3db::testing;

namespace {

constexpr int64_t NUM_SCOPES = 72000;
constexpr int64_t NUM_MASKED = 117;
constexpr int64_t NUM_SYMBOLS = 10000;
constexpr int64_t READ_FACTOR = 10;
constexpr auto HOP = "MATCH (d:Symbol {_uuid: 's'})<-[r:MENTIONS]-(m:Scope) RETURN m._uuid, m.name;";

class MaskedScan : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        load(NUM_SCOPES);
    }

    // numScopes Scope, dont NUM_MASKED éparpillés mentionnent le Symbol 's'.
    void load(int64_t numScopes) {
        createDBAndConn();
        ok("CALL auto_checkpoint=false;");
        ok("CREATE NODE TABLE Scope(_uuid STRING PRIMARY KEY, name STRING, scope_type STRING, "
           "repo_path STRING, file_path STRING, start_line INT64, idx INT64);");
        ok("CREATE NODE TABLE Symbol(_uuid STRING PRIMARY KEY);");
        ok("CREATE REL TABLE MENTIONS(FROM Scope TO Symbol);");
        // À l'échelle du cas réel, pour que le planificateur parte du Symbol et masque Scope :
        // MENTIONS compte des centaines de milliers de relations (trois par Scope, vers des
        // Symbols ordinaires), et seuls k Scope éparpillés mentionnent le Symbol 's'. Avec 117
        // relations en tout, il balayait Scope puis remontait les relations, sans masque.
        const auto scopes = databasePath + ".scopes.csv";
        const auto symbols = databasePath + ".symbols.csv";
        const auto mentions = databasePath + ".mentions.csv";
        {
            std::ofstream out(scopes);
            for (int64_t i = 0; i < numScopes; ++i) {
                out << "scope-" << i << ",name " << i << ",function,repo,src/file" << i % 900
                    << ".rs," << i % 4000 << "," << i << "\n";
            }
        }
        {
            std::ofstream out(symbols);
            out << "s\n";
            for (int64_t i = 0; i < NUM_SYMBOLS; ++i) {
                out << "sym-" << i << "\n";
            }
        }
        {
            std::ofstream out(mentions);
            const auto step = numScopes / NUM_MASKED;
            int64_t masked = 0;
            for (int64_t i = 0; i < numScopes; ++i) {
                for (const auto factor : {1, 7, 13}) {
                    out << "scope-" << i << ",sym-" << (i * factor) % NUM_SYMBOLS << "\n";
                }
                if (i % step == 0 && masked < NUM_MASKED) {
                    out << "scope-" << i << ",s\n";
                    ++masked;
                }
            }
        }
        ok("COPY Scope FROM '" + scopes + "' (header=false);");
        ok("COPY Symbol FROM '" + symbols + "' (header=false);");
        ok("COPY MENTIONS FROM '" + mentions + "' (header=false);");
        ok("CHECKPOINT;");
        std::filesystem::remove(scopes);
        std::filesystem::remove(symbols);
        std::filesystem::remove(mentions);
    }

    void ok(const std::string& query) {
        auto result = conn->query(query);
        ASSERT_TRUE(result->isSuccess()) << query << " : " << result->getErrorMessage();
    }

    int64_t count(const std::string& query) {
        auto result = conn->query(query);
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        return result->isSuccess() && result->hasNext() ?
                   result->getNext()->getValue(0)->getValue<int64_t>() :
                   -1;
    }

    // Le plan profilé, tel que le moteur le rend.
    std::string profile(const std::string& query) {
        auto result = conn->query("PROFILE " + query);
        EXPECT_TRUE(result->isSuccess()) << result->getErrorMessage();
        std::string out;
        while (result->isSuccess() && result->hasNext()) {
            out += result->getNext()->toString() + "\n";
        }
        return out;
    }

    // Les lignes sorties d'un opérateur, lues dans le plan dessiné : la boîte de l'opérateur, puis
    // le premier « NumOutputTuples: » dont la colonne tombe dans la largeur de cette boîte.
    static std::optional<int64_t> outputTuplesOf(const std::string& plan, const std::string& name) {
        std::vector<std::string> lines;
        std::istringstream in(plan);
        for (std::string line; std::getline(in, line);) {
            lines.push_back(line);
        }
        // Le nom entier seulement : « SCAN_NODE_TABLE » ne doit pas prendre la boîte de
        // PRIMARY_KEY_SCAN_NODE_TABLE.
        const auto partOfName = [](char c) {
            return std::isalnum(static_cast<unsigned char>(c)) || c == '_';
        };
        const auto wholeName = [&](const std::string& text) {
            for (auto at = text.find(name); at != std::string::npos; at = text.find(name, at + 1)) {
                const auto end = at + name.size();
                if ((at == 0 || !partOfName(text[at - 1])) &&
                    (end == text.size() || !partOfName(text[end]))) {
                    return at;
                }
            }
            return std::string::npos;
        };
        for (auto i = 0u; i < lines.size(); ++i) {
            const auto column = wholeName(lines[i]);
            if (column == std::string::npos) {
                continue;
            }
            for (auto j = i + 1; j < lines.size() && j < i + 12; ++j) {
                const auto key = lines[j].find("NumOutputTuples:", column > 20 ? column - 20 : 0);
                if (key != std::string::npos && key < column + name.size() + 20) {
                    return std::stoll(lines[j].substr(key + 16));
                }
            }
        }
        return std::nullopt;
    }

    // Les lignes sorties du SCAN_NODE_TABLE dont la boîte dit « Tables: <table> » : le titre d'une
    // boîte (pas la ligne « Operators: » d'un SEMI_MASKER), puis, dans sa largeur, la table et le
    // premier « NumOutputTuples: ».
    static std::optional<int64_t> scanOutputOf(const std::string& plan, const std::string& table) {
        std::vector<std::string> lines;
        std::istringstream in(plan);
        for (std::string line; std::getline(in, line);) {
            lines.push_back(line);
        }
        const std::string title = "SCAN_NODE_TABLE[";
        for (auto i = 0u; i < lines.size(); ++i) {
            for (auto at = lines[i].find(title); at != std::string::npos;
                at = lines[i].find(title, at + 1)) {
                // Un titre de boîte : rien que des espaces entre le bord « │ » et le nom.
                const auto border = lines[i].rfind("│", at);
                if (border == std::string::npos ||
                    lines[i].find_first_not_of(' ', border + std::string("│").size()) != at) {
                    continue;
                }
                const auto from = at > 20 ? at - 20 : 0;
                const auto inBox = [&](const std::string& line, const std::string& what) {
                    const auto found = line.find(what, from);
                    return found != std::string::npos && found < at + title.size() + 20 ?
                               found :
                               std::string::npos;
                };
                bool isTable = false;
                for (auto j = i + 1; j < lines.size() && j < i + 4 && !isTable; ++j) {
                    isTable = inBox(lines[j], "Tables: " + table + " ") != std::string::npos;
                }
                if (!isTable) {
                    continue;
                }
                for (auto j = i + 1; j < lines.size() && j < i + 16; ++j) {
                    const auto key = inBox(lines[j], "NumOutputTuples:");
                    if (key != std::string::npos) {
                        return std::stoll(lines[j].substr(key + 16));
                    }
                }
            }
        }
        return std::nullopt;
    }

    // Les lignes sorties de la boîte qui porte `marker` (par exemple « Alias: d » ou « Tables:
    // HAS_PARENT ») ; si `title` n'est pas vide, la boîte doit avoir ce titre (dans les six lignes
    // au-dessus du marqueur, à la même colonne). Rien si aucune boîte ne porte le marqueur.
    static std::optional<int64_t> boxOutputOf(const std::string& plan, const std::string& marker,
        const std::string& title) {
        std::vector<std::string> lines;
        std::istringstream in(plan);
        for (std::string line; std::getline(in, line);) {
            lines.push_back(line);
        }
        const auto near = [](const std::string& line, const std::string& what, size_t column) {
            const auto from = column > 25 ? column - 25 : 0;
            const auto found = line.find(what, from);
            return found != std::string::npos && found < column + 25 ? found : std::string::npos;
        };
        for (auto i = 0u; i < lines.size(); ++i) {
            for (auto at = lines[i].find(marker); at != std::string::npos;
                at = lines[i].find(marker, at + 1)) {
                bool titled = title.empty();
                for (auto j = i; j > 0 && j + 6 > i && !titled; --j) {
                    titled = near(lines[j - 1], title, at) != std::string::npos;
                }
                if (!titled) {
                    continue;
                }
                for (auto j = i + 1; j < lines.size() && j < i + 14; ++j) {
                    const auto key = near(lines[j], "NumOutputTuples:", at);
                    if (key != std::string::npos) {
                        return std::stoll(lines[j].substr(key + 16));
                    }
                }
            }
        }
        return std::nullopt;
    }

    static int64_t wholeWordCount(const std::string& text, const std::string& word) {
        int64_t count = 0;
        for (auto at = text.find(word); at != std::string::npos; at = text.find(word, at + 1)) {
            const auto end = at + word.size();
            if ((at == 0 || !(std::isalnum(static_cast<unsigned char>(text[at - 1])) ||
                                text[at - 1] == '_')) &&
                (end == text.size() || text[end] == '[')) {
                count++;
            }
        }
        return count;
    }
};

TEST_F(MaskedScan, ReadsAboutTheMaskNotTheTable) {
    ASSERT_EQ(count("MATCH (:Symbol {_uuid: 's'})<-[:MENTIONS]-(m:Scope) RETURN count(m);"),
        NUM_MASKED);
    const auto plan = profile(HOP);
    std::cerr << plan;
    ASSERT_NE(plan.find("SEMI_MASKER"), std::string::npos)
        << "[check: plan] the hop is no longer planned through a semi-mask";
    const auto read = outputTuplesOf(plan, "SCAN_NODE_TABLE");
    ASSERT_TRUE(read.has_value()) << "[check: plan] no SCAN_NODE_TABLE output count in the plan";
    std::cerr << "  [masque] k = " << NUM_MASKED << ", N = " << NUM_SCOPES
              << ", lignes sorties du balayage = " << *read << "\n";
    EXPECT_LE(*read, READ_FACTOR * NUM_MASKED)
        << "[check: scan-reads-the-mask] the scan under a mask of " << NUM_MASKED
        << " rows put out " << *read << " rows of a table of " << NUM_SCOPES;
}

TEST_F(MaskedScan, MeasureAtOneAndThirtyTwoThreads) {
    if (std::getenv("CONCURRENCE_MEASURE") == nullptr) {
        GTEST_SKIP() << "a measure: CONCURRENCE_MEASURE=1, under `poste mesure`";
    }
    for (const auto threads : {1, 32}) {
        ok(stringFormat("CALL threads={};", threads));
        const auto read = outputTuplesOf(profile(HOP), "SCAN_NODE_TABLE");
        std::vector<double> durations;
        for (auto pass = 0; pass < 5; ++pass) {
            const auto start = std::chrono::steady_clock::now();
            auto result = conn->query(HOP);
            ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
            while (result->hasNext()) {
                result->getNext();
            }
            durations.push_back(std::chrono::duration<double, std::milli>(
                std::chrono::steady_clock::now() - start)
                                    .count());
        }
        std::sort(durations.begin(), durations.end());
        std::cerr << "  [masque mesure] threads=" << threads << " lignes sorties du balayage="
                  << (read ? std::to_string(*read) : std::string("?"))
                  << " durée médiane=" << durations[2] << " ms (min " << durations.front()
                  << ", max " << durations.back() << ")\n";
    }
}

// Le voisin d'un saut depuis un ensemble masqué (second niveau de la jointure par clé du cœur C++,
// 12 octobre 2026) : la V1 de F telle que l'IR la compile, UNWIND d'une liste de clés puis le saut,
// et plusieurs propriétés du voisin. Devant un voisin grand lu par plusieurs propriétés, le
// planificateur balaie Scope côté build et le joint par hachage aux m venus de MENTIONS ; le départ
// est masqué par clés, mais Scope était lu entier (81 407 lignes, environ 20 ms au PROFILE de F).
// - MaskedNeighbour.ReadsAboutTheMaskNotTheTable : PROFILE ; le contrôle plan (un SEMI_MASKER au
//   moins, un SCAN_NODE_TABLE de Scope) ; le nombre de SEMI_MASKER imprimé (deux attendus : les
//   clés sur Symbol, les identifiants sur Scope) ; les lignes sorties du balayage de Scope au plus
//   10 × 117.
// - MaskedNeighbour.MeasureAtOneAndThirtyTwoThreads : la même requête à 1 puis 32 fils, médiane
//   de cinq passes. Sautée sans CONCURRENCE_MEASURE=1 ; à jouer sous `poste mesure`.
constexpr int64_t NEIGHBOUR_SCOPES = 81407;
constexpr auto NEIGHBOUR_HOP =
    "UNWIND ['s'] AS u MATCH (d:Symbol {_uuid: u})<-[r:MENTIONS]-(m:Scope) RETURN u, m._uuid, "
    "m.name, m.scope_type, m.repo_path, m.file_path, m.start_line;";

class MaskedNeighbour : public MaskedScan {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        load(NEIGHBOUR_SCOPES);
    }
};

TEST_F(MaskedNeighbour, ReadsAboutTheMaskNotTheTable) {
    ASSERT_EQ(count("MATCH (:Symbol {_uuid: 's'})<-[:MENTIONS]-(m:Scope) RETURN count(m);"),
        NUM_MASKED);
    const auto plan = profile(NEIGHBOUR_HOP);
    std::cerr << plan;
    const auto maskers = wholeWordCount(plan, "SEMI_MASKER");
    ASSERT_GE(maskers, 1) << "[check: plan] the keys of the UNWIND are no longer a mask";
    const auto read = scanOutputOf(plan, "Scope");
    ASSERT_TRUE(read.has_value()) << "[check: plan] no SCAN_NODE_TABLE of Scope in the plan";
    std::cerr << "  [voisin] k = " << NUM_MASKED << ", N = " << NEIGHBOUR_SCOPES
              << ", SEMI_MASKER = " << maskers << ", lignes sorties du balayage de Scope = "
              << *read << "\n";
    EXPECT_LE(*read, READ_FACTOR * NUM_MASKED)
        << "[check: neighbour-scan-reads-the-mask] the scan of Scope put out " << *read
        << " rows for " << NUM_MASKED << " neighbours";
}

TEST_F(MaskedNeighbour, MeasureAtOneAndThirtyTwoThreads) {
    if (std::getenv("CONCURRENCE_MEASURE") == nullptr) {
        GTEST_SKIP() << "a measure: CONCURRENCE_MEASURE=1, under `poste mesure`";
    }
    for (const auto threads : {1, 32}) {
        ok(stringFormat("CALL threads={};", threads));
        const auto read = scanOutputOf(profile(NEIGHBOUR_HOP), "Scope");
        std::vector<double> durations;
        for (auto pass = 0; pass < 5; ++pass) {
            const auto start = std::chrono::steady_clock::now();
            auto result = conn->query(NEIGHBOUR_HOP);
            ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
            while (result->hasNext()) {
                result->getNext();
            }
            durations.push_back(std::chrono::duration<double, std::milli>(
                std::chrono::steady_clock::now() - start)
                                    .count());
        }
        std::sort(durations.begin(), durations.end());
        std::cerr << "  [voisin mesure] threads=" << threads << " lignes sorties du balayage de "
                  << "Scope=" << (read ? std::to_string(*read) : std::string("?"))
                  << " durée médiane=" << durations[2] << " ms (min " << durations.front()
                  << ", max " << durations.back() << ")\n";
    }
}

// Le saut d'une table vers elle-même depuis un nœud lié par sa clé (troisième pas de la jointure
// par clé du cœur C++, proposé par F, 12 octobre 2026) : UNWIND d'une clé de Scope, puis HAS_PARENT
// vers un autre Scope. Le départ d est masqué (une ligne), mais les cardinalités estimées de d et
// de m sont égales et l'extension part de m : Scope entière (81 407), HAS_PARENT entière, et une
// table de hachage sur d._ID que l'unique d sonde à la fin (18 à 20 ms par saut dans usages).
// - MaskedSelfHop.StartsFromTheKeyedNode : le contrôle plan (un SEMI_MASKER au moins) ; les lignes
//   sorties d'un balayage de Scope porté par l'alias d, puis par l'alias m, chacune au plus
//   10 × (parents de d) quand ce balayage existe (m lu par ses propriétés au bout de l'extension
//   n'en a pas) ; les lignes sorties de la boîte de HAS_PARENT au plus 10.
// - MaskedSelfHop.MeasureAtOneAndThirtyTwoThreads : 1 puis 32 fils, médiane de cinq passes.
//   Sautée sans CONCURRENCE_MEASURE=1 ; à jouer sous `poste mesure`.
constexpr auto SELF_HOP = "UNWIND ['scope-40001'] AS u MATCH (d:Scope {_uuid: u})-[r:HAS_PARENT]->"
                          "(m:Scope) RETURN u, m._uuid, m.name, m.repo_path, m.file_path;";

class MaskedSelfHop : public MaskedNeighbour {
protected:
    void SetUp() override {
        MaskedNeighbour::SetUp();
        // Un parent pour 6 Scope sur 13, environ 37 600 relations (37 802 dans l'index de F).
        ok("CREATE REL TABLE HAS_PARENT(FROM Scope TO Scope);");
        const auto parents = databasePath + ".parents.csv";
        {
            std::ofstream out(parents);
            for (int64_t i = 1; i < NEIGHBOUR_SCOPES; ++i) {
                if (i % 13 < 6) {
                    out << "scope-" << i << ",scope-" << i / 3 << "\n";
                }
            }
        }
        ok("COPY HAS_PARENT FROM '" + parents + "' (header=false);");
        ok("CHECKPOINT;");
        std::filesystem::remove(parents);
    }
};

TEST_F(MaskedSelfHop, StartsFromTheKeyedNode) {
    const auto numParents =
        count("MATCH (d:Scope {_uuid: 'scope-40001'})-[:HAS_PARENT]->(m:Scope) RETURN count(m);");
    ASSERT_GE(numParents, 1);
    ASSERT_GE(count("MATCH ()-[r:HAS_PARENT]->() RETURN count(r);"), 30000);
    const auto plan = profile(SELF_HOP);
    std::cerr << plan;
    const auto maskers = wholeWordCount(plan, "SEMI_MASKER");
    ASSERT_GE(maskers, 1) << "[check: plan] the key of the UNWIND is no longer a mask";
    const auto fromD = boxOutputOf(plan, "Alias: d ", "SCAN_NODE_TABLE[");
    const auto fromM = boxOutputOf(plan, "Alias: m ", "SCAN_NODE_TABLE[");
    const auto hop = boxOutputOf(plan, "Tables: HAS_PARENT ", "");
    const auto text = [](const std::optional<int64_t>& value) {
        return value ? std::to_string(*value) : std::string("aucun balayage");
    };
    std::cerr << "  [saut] parents de d = " << numParents << ", SEMI_MASKER = " << maskers
              << ", Scope sous d = " << text(fromD) << ", Scope sous m = " << text(fromM)
              << ", HAS_PARENT = " << text(hop) << "\n";
    ASSERT_TRUE(hop.has_value()) << "[check: plan] no HAS_PARENT box in the plan";
    EXPECT_LE(fromD.value_or(0), READ_FACTOR * numParents)
        << "[check: self-hop-reads-the-keyed-node] the scan of Scope for d put out " << *fromD;
    EXPECT_LE(fromM.value_or(0), READ_FACTOR * numParents)
        << "[check: self-hop-reads-the-neighbours] the scan of Scope for m put out "
        << fromM.value_or(0);
    EXPECT_LE(*hop, READ_FACTOR)
        << "[check: self-hop-reads-few-relations] the HAS_PARENT box put out " << *hop;
}

TEST_F(MaskedSelfHop, MeasureAtOneAndThirtyTwoThreads) {
    if (std::getenv("CONCURRENCE_MEASURE") == nullptr) {
        GTEST_SKIP() << "a measure: CONCURRENCE_MEASURE=1, under `poste mesure`";
    }
    for (const auto threads : {1, 32}) {
        ok(stringFormat("CALL threads={};", threads));
        const auto plan = profile(SELF_HOP);
        const auto fromM = boxOutputOf(plan, "Alias: m ", "SCAN_NODE_TABLE[");
        std::vector<double> durations;
        for (auto pass = 0; pass < 5; ++pass) {
            const auto start = std::chrono::steady_clock::now();
            auto result = conn->query(SELF_HOP);
            ASSERT_TRUE(result->isSuccess()) << result->getErrorMessage();
            while (result->hasNext()) {
                result->getNext();
            }
            durations.push_back(std::chrono::duration<double, std::milli>(
                std::chrono::steady_clock::now() - start)
                                    .count());
        }
        std::sort(durations.begin(), durations.end());
        std::cerr << "  [saut mesure] threads=" << threads << " Scope sous m="
                  << (fromM ? std::to_string(*fromM) : std::string("aucun balayage"))
                  << " durée médiane=" << durations[2] << " ms (min " << durations.front()
                  << ", max " << durations.back() << ")\n";
    }
}

} // namespace
