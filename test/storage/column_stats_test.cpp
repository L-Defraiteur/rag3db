#include <cmath>

#include "common/data_chunk/data_chunk_state.h"
#include "common/system_config.h"
#include "common/vector/value_vector.h"
#include "gtest/gtest.h"
#include "storage/stats/column_stats.h"

using namespace rag3db::common;
using namespace rag3db::storage;

namespace {

// L'HyperLogLog a 64 registres : une erreur type d'environ 1,04 / √64 ≈ 13 %. Trois erreurs
// types laissent de la marge sans laisser passer un compte faux d'un ordre de grandeur.
void expectDistinctCountNear(const ColumnStats& stats, uint64_t truth) {
    const auto estimate = static_cast<double>(stats.getNumDistinctValues());
    EXPECT_LT(std::abs(estimate - static_cast<double>(truth)), 0.4 * static_cast<double>(truth))
        << "estimation " << estimate << " pour " << truth << " valeurs distinctes";
}

// Une sélection filtrée : les hachages sont rangés aux positions sélectionnées
// (VectorHashFunction::computeHash) ; ColumnStats::update doit les relire là, pas aux
// premières cases du vecteur.
TEST(ColumnStatsTest, DistinctCountReadsTheSelectedPositions) {
    static constexpr sel_t numSelected = 1000;
    auto state = std::make_shared<DataChunkState>(DEFAULT_VECTOR_CAPACITY);
    ValueVector values(LogicalType::INT64());
    values.setState(state);
    // Les cases non sélectionnées portent toutes la même valeur : relues par erreur, elles
    // ne compteraient que pour une.
    for (auto pos = 0u; pos < DEFAULT_VECTOR_CAPACITY; pos++) {
        values.setValue<int64_t>(pos, -1);
    }
    auto& selVector = state->getSelVectorUnsafe();
    selVector.setToFiltered(numSelected);
    auto buffer = selVector.getMutableBuffer();
    for (auto i = 0u; i < numSelected; i++) {
        const sel_t pos = DEFAULT_VECTOR_CAPACITY - numSelected + i;
        buffer[i] = pos;
        values.setValue<int64_t>(pos, static_cast<int64_t>(i) * 7919);
    }

    ColumnStats stats(LogicalType::INT64());
    stats.update(&values);
    expectDistinctCountNear(stats, numSelected);
}

// Le témoin de la même mesure sans filtre : il dit que la marge tient pour l'HyperLogLog
// lui-même.
TEST(ColumnStatsTest, DistinctCountUnfiltered) {
    static constexpr sel_t numValues = 1000;
    auto state = std::make_shared<DataChunkState>(DEFAULT_VECTOR_CAPACITY);
    ValueVector values(LogicalType::INT64());
    values.setState(state);
    state->getSelVectorUnsafe().setToUnfiltered(numValues);
    for (auto i = 0u; i < numValues; i++) {
        values.setValue<int64_t>(i, static_cast<int64_t>(i) * 7919);
    }

    ColumnStats stats(LogicalType::INT64());
    stats.update(&values);
    expectDistinctCountNear(stats, numValues);
}

} // namespace
