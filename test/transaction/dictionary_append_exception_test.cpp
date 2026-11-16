#include <iostream>
#include <string>
#include <vector>

#include "common/build_id.h"
#include "common/exception/buffer_manager.h"
#include "gtest/gtest.h"
#include "storage/buffer_manager/buffer_manager.h"
#include "storage/buffer_manager/memory_manager.h"
#include "storage/table/dictionary_chunk.h"

using namespace rag3db::common;
using namespace rag3db::storage;

namespace {

// Le même point de substitution que string_finalize_test : aucun crochet global,
// et un refus réel du gestionnaire de mémoire sur l'allocation des offsets.
class DictionaryFailingBufferManager : public BufferManager {
public:
    DictionaryFailingBufferManager()
        : BufferManager(":memory:", "", 64ull << 20, 128ull << 20, nullptr, true) {}

    bool rejectNextReservation = false;
    uint64_t rejectedBytes = 0;

    bool reserve(uint64_t bytes) override {
        if (rejectNextReservation) {
            rejectNextReservation = false;
            rejectedBytes = bytes;
            return false;
        }
        return BufferManager::reserve(bytes);
    }
};

TEST(DictionaryAppendException, OffsetAllocationFailureKeepsExistingStringsUnchanged) {
    std::cout << "BUILD_ID=" << RAG3DB_BUILD_ID << std::endl;
    DictionaryFailingBufferManager bm;
    MemoryManager mm(&bm, nullptr);
    DictionaryChunk dictionary(mm, 3, true, ResidencyState::IN_MEMORY);
    // La quatrième chaîne tient dans les octets déjà alloués ; seule la table
    // d'offsets doit grandir. Le refus n'arrive donc pas avant la copie du texte.
    dictionary.getStringDataChunk()->resize(64);
    const std::vector<std::string> before{"alpha", "beta", "gamma"};
    for (uint64_t i = 0; i < before.size(); ++i) {
        ASSERT_EQ(dictionary.appendString(before[i]), i);
    }
    ASSERT_EQ(dictionary.getOffsetChunk()->getCapacity(), 3);
    ASSERT_EQ(dictionary.getOffsetChunk()->getNumValues(), before.size());
    const auto bytesBefore = dictionary.getStringDataChunk()->getNumValues();
    ASSERT_GE(dictionary.getStringDataChunk()->getCapacity() - bytesBefore, 5);

    bm.rejectNextReservation = true;
    EXPECT_THROW(dictionary.appendString("delta"), BufferManagerException);
    ASSERT_FALSE(bm.rejectNextReservation) << "the intended allocation was not reached";
    ASSERT_EQ(bm.rejectedBytes, 6 * sizeof(uint64_t)) << "not the expected offset resize";
    ASSERT_EQ(dictionary.getOffsetChunk()->getNumValues(), before.size());
    // Comparer les valeurs réellement lisibles, pas seulement un compteur : la
    // longueur de la dernière entrée dépend du nombre d'octets du dictionnaire.
    for (uint64_t i = 0; i < before.size(); ++i) {
        ASSERT_EQ(dictionary.getString(i), before[i]) << "existing dictionary entry " << i;
    }
    EXPECT_EQ(dictionary.getStringDataChunk()->getNumValues(), bytesBefore);

    // Ne sonder le hash qu'après avoir vérifié que ses clés sont restées valides.
    for (uint64_t i = 0; i < before.size(); ++i) {
        EXPECT_EQ(dictionary.appendString(before[i]), i) << "existing dictionary hash entry " << i;
    }
    EXPECT_EQ(dictionary.getOffsetChunk()->getNumValues(), before.size());
    EXPECT_EQ(dictionary.getStringDataChunk()->getNumValues(), bytesBefore);

    const auto added = dictionary.appendString("delta");
    ASSERT_EQ(added, before.size());
    EXPECT_EQ(dictionary.getString(added), "delta");
    for (uint64_t i = 0; i < before.size(); ++i) {
        EXPECT_EQ(dictionary.getString(i), before[i]);
        EXPECT_EQ(dictionary.appendString(before[i]), i);
    }
    EXPECT_EQ(dictionary.appendString("delta"), added);
    EXPECT_EQ(dictionary.getOffsetChunk()->getNumValues(), before.size() + 1);
    EXPECT_EQ(dictionary.getStringDataChunk()->getNumValues(), bytesBefore + 5);
}

} // namespace
