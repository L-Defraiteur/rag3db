// L'étendue du fichier de données et ce qui la dépasse (ticket
// 2026-11-08-un-sigsegv-dans-optimisticread-sous-lecture-concurrente-pendant-une-indexation).
//
// La base de code-essais-c7 du 8 novembre : un en-tête qui déclarait 455 380 pages, deux
// segments vivants à la page 83 108 096 d'un fichier creux, une entrée libre au-delà de
// l'étendue. Ouverte en écriture, elle rendait libres les pages de ces segments
// (returnOwnerlessPages) ; un point de reprise baissait numPages sous eux
// (handleLastPageRange), et le lecteur suivant sortait de la chaîne des états de page.
// - ASegmentBeyondTheExtentIsNotGivenBack : un en-tête dont l'étendue tombe sous le dernier
//   segment (l'état de la copie), ouvert, point de reprise : aucune page à la fois libre et
//   tenue, tout se relit.
// - ARangeBeyondTheFileIsNotFreed : une plage au-delà du fichier n'entre pas dans l'espace libre.
// - AWriteBeyondTheFileIsRefused : écrire au-delà de numPages est une erreur, pas un trou.
// - APageBeyondTheFileIsAnErrorNotACrash : lire une page au-delà du fichier lève une erreur ;
//   un processus enfant, parce qu'avant le correctif c'est un SIGSEGV.

#include <sys/wait.h>
#include <unistd.h>

#include <algorithm>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <string>
#include <vector>

#include "common/exception/buffer_manager.h"
#include "common/exception/runtime.h"
#include "graph_test/private_graph_test.h"
#include "main/connection.h"
#include "main/database.h"
#include "storage/file_handle.h"
#include "storage/page_manager.h"
#include "storage/storage_manager.h"

using namespace rag3db::common;
using namespace rag3db::main;
using namespace rag3db::storage;
using namespace rag3db::testing;

namespace {

struct Range {
    uint64_t start, num;
};

class DataFileExtentTest : public EmptyDBTest {
protected:
    void SetUp() override {
        EmptyDBTest::SetUp();
        if (inMemMode) {
            GTEST_SKIP() << "on-disk database needed: the subject is its file";
        }
        createDBAndConn();
    }
    void run(const std::string& q) const {
        auto result = conn->query(q);
        ASSERT_TRUE(result->isSuccess()) << q << " : " << result->getErrorMessage();
    }
    std::string row(const std::string& q) const {
        auto result = conn->query(q);
        if (!result->isSuccess()) {
            return "REFUS " + result->getErrorMessage();
        }
        return result->hasNext() ? result->getNext()->toString() : "AUCUNE LIGNE";
    }
    void reopen() {
        conn.reset();
        database.reset();
        createDBAndConn();
    }
    FileHandle* dataFH() const {
        return StorageManager::Get(*conn->getClientContext())->getDataFH();
    }
    std::vector<Range> segments(const std::string& table) const {
        std::vector<Range> result;
        auto r =
            conn->query("CALL storage_info('" + table + "') RETURN start_page_idx, num_pages;");
        EXPECT_TRUE(r->isSuccess()) << r->getErrorMessage();
        while (r->hasNext()) {
            auto t = r->getNext();
            const auto start = static_cast<uint64_t>(t->getValue(0)->getValue<int64_t>());
            const auto num = static_cast<uint64_t>(t->getValue(1)->getValue<int64_t>());
            if (start != INVALID_PAGE_IDX && num > 0) {
                result.push_back({start, num});
            }
        }
        return result;
    }
    std::vector<Range> freeRanges() const {
        std::vector<Range> result;
        auto r = conn->query("CALL fsm_info() RETURN start_page_idx, num_pages;");
        EXPECT_TRUE(r->isSuccess()) << r->getErrorMessage();
        while (r->hasNext()) {
            auto t = r->getNext();
            result.push_back(
                {t->getValue(0)->getValue<uint64_t>(), t->getValue(1)->getValue<uint64_t>()});
        }
        return result;
    }
    // Les pages à la fois libres et tenues par un segment de `table`.
    uint64_t pagesBothFreeAndOwned(const std::string& table) const {
        uint64_t both = 0;
        const auto owned = segments(table);
        for (const auto& f : freeRanges()) {
            for (const auto& s : owned) {
                const auto a = std::max(f.start, s.start);
                const auto b = std::min(f.start + f.num, s.start + s.num);
                if (a < b) {
                    both += b - a;
                }
            }
        }
        return both;
    }
    // La position de numDataPages dans la page d'en-tête : la magie (6 octets), la version (8),
    // la chaîne du bâti (longueur sur 8, puis les octets), le catalogue et les métadonnées (deux
    // fois deux entiers de 4), l'identifiant de la base (16).
    static size_t extentOffset(const std::vector<char>& page) {
        uint64_t writerLength = 0;
        memcpy(&writerLength, page.data() + 14, sizeof(writerLength));
        return 22 + writerLength + 16 + 16;
    }
    std::vector<char> readHeaderPage() const {
        std::vector<char> page(RAG3DB_PAGE_SIZE);
        std::ifstream file(databasePath, std::ios::binary);
        file.read(page.data(), page.size());
        return page;
    }
};

TEST_F(DataFileExtentTest, ASegmentBeyondTheExtentIsNotGivenBack) {
    run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, s STRING, v DOUBLE);");
    run("UNWIND range(1, 300000) AS i CREATE (:Doc {id: i, s: 'texte ' + CAST(i AS STRING), "
        "v: i * 0.5});");
    run("CHECKPOINT;");
    const auto owned = segments("Doc");
    ASSERT_FALSE(owned.empty());
    const auto last = *std::max_element(owned.begin(), owned.end(),
        [](const Range& a, const Range& b) { return a.start < b.start; });
    const auto expected = row("MATCH (d:Doc) RETURN count(*), sum(d.v), min(d.s), max(d.s);");
    conn.reset();
    database.reset();

    // L'état de la copie : l'étendue de l'en-tête tombe au début du dernier segment.
    auto page = readHeaderPage();
    const auto at = extentOffset(page);
    uint32_t extent = 0;
    memcpy(&extent, page.data() + at, sizeof(extent));
    const auto pagesInFile = std::filesystem::file_size(databasePath) / RAG3DB_PAGE_SIZE;
    uint64_t version = 0;
    memcpy(&version, page.data() + 6, sizeof(version));
    ASSERT_EQ(std::string(page.data(), 6), "RAG3DB");
    ASSERT_GE(version, 43u) << "the extent is not where this test expects it";
    ASSERT_GT(extent, last.start) << "the extent read is " << extent << " for a file of "
                                  << pagesInFile << " pages";
    ASSERT_LE(extent, pagesInFile + 64);
    const auto lowered = static_cast<uint32_t>(last.start);
    {
        std::fstream file(databasePath, std::ios::binary | std::ios::in | std::ios::out);
        file.seekp(static_cast<std::streamoff>(at));
        file.write(reinterpret_cast<const char*>(&lowered), sizeof(lowered));
    }

    createDBAndConn();
    run("CHECKPOINT;");
    EXPECT_EQ(pagesBothFreeAndOwned("Doc"), 0u)
        << "pages of the segment at " << last.start << " were given back as free";
    EXPECT_EQ(row("MATCH (d:Doc) RETURN count(*), sum(d.v), min(d.s), max(d.s);"), expected);
    // Une écriture qui réalloue des pages ne doit pas écraser le dernier segment.
    run("CREATE NODE TABLE Other(id INT64 PRIMARY KEY, s STRING);");
    run("UNWIND range(1, 300000) AS i CREATE (:Other {id: i, s: 'autre ' + CAST(i AS STRING)});");
    run("CHECKPOINT;");
    EXPECT_EQ(pagesBothFreeAndOwned("Doc"), 0u);
    EXPECT_EQ(row("MATCH (d:Doc) RETURN count(*), sum(d.v), min(d.s), max(d.s);"), expected);
    reopen();
    EXPECT_EQ(row("MATCH (d:Doc) RETURN count(*), sum(d.v), min(d.s), max(d.s);"), expected);
}

TEST_F(DataFileExtentTest, ARangeBeyondTheFileIsNotFreed) {
    run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY);");
    run("UNWIND range(1, 1000) AS i CREATE (:Doc {id: i});");
    run("CHECKPOINT;");
    const auto numPages = dataFH()->getNumPages();
    PageManager::Get(*conn->getClientContext())->freePageRange(PageRange{numPages + 1000, 256});
    run("CREATE (:Doc {id: 5000});");
    run("CHECKPOINT;");
    for (const auto& f : freeRanges()) {
        EXPECT_LE(f.start + f.num, dataFH()->getNumPages())
            << "free range [" << f.start << ", +" << f.num << ") lies beyond the file";
    }
}

TEST_F(DataFileExtentTest, AWriteBeyondTheFileIsRefused) {
    run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY);");
    run("CHECKPOINT;");
    const auto sizeBefore = std::filesystem::file_size(databasePath);
    std::vector<uint8_t> buffer(RAG3DB_PAGE_SIZE, 7);
    const auto beyond = dataFH()->getNumPages() + 100000;
    EXPECT_THROW(dataFH()->writePageToFile(buffer.data(), beyond), RuntimeException);
    EXPECT_EQ(std::filesystem::file_size(databasePath), sizeBefore);
}

TEST_F(DataFileExtentTest, APageBeyondTheFileIsAnErrorNotACrash) {
    run("CREATE NODE TABLE Doc(id INT64 PRIMARY KEY);");
    run("CHECKPOINT;");
    const auto child = fork();
    ASSERT_GE(child, 0);
    if (child == 0) {
        auto status = 6; // rien n'a été levé
        try {
            // Au-delà d'un millier de blocs d'états de page, comme les deux cores de l'hôte.
            dataFH()->optimisticReadPage(dataFH()->getNumPages() + 3000000, [](uint8_t*) {});
        } catch (const BufferManagerException&) {
            status = 0;
        } catch (...) {
            status = 5;
        }
        _exit(status);
    }
    int status = 0;
    ASSERT_EQ(waitpid(child, &status, 0), child);
    ASSERT_FALSE(WIFSIGNALED(status)) << "the read died of signal " << WTERMSIG(status);
    EXPECT_EQ(WEXITSTATUS(status), 0) << "5: another exception, 6: nothing thrown";
}

} // namespace
