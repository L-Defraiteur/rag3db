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
        createDBAndConn();
        ok("CALL auto_checkpoint=false;");
        ok("CREATE NODE TABLE Scope(_uuid STRING PRIMARY KEY, name STRING, idx INT64);");
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
            for (int64_t i = 0; i < NUM_SCOPES; ++i) {
                out << "scope-" << i << ",name " << i << "," << i << "\n";
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
            const auto step = NUM_SCOPES / NUM_MASKED;
            int64_t masked = 0;
            for (int64_t i = 0; i < NUM_SCOPES; ++i) {
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

} // namespace
