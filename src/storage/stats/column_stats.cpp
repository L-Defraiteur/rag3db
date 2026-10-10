#include "storage/stats/column_stats.h"

#include "function/hash/vector_hash_functions.h"

namespace rag3db {
namespace storage {

ColumnStats::ColumnStats(const common::LogicalType& dataType) : hashes{nullptr} {
    if (!common::LogicalTypeUtils::isNested(dataType)) {
        hll.emplace();
    }
}

void ColumnStats::update(const common::ValueVector* vector) {
    if (hll) {
        if (!hashes) {
            hashes = std::make_unique<common::ValueVector>(common::LogicalTypeID::UINT64);
        }
        hashes->state = vector->state;
        function::VectorHashFunction::computeHash(*vector, vector->state->getSelVector(), *hashes,
            hashes->state->getSelVector());
        KU_ASSERT(hashes->hasNoNullsGuarantee());
        // computeHash range chaque hachage à sa position sélectionnée : sous une sélection
        // filtrée, les premières cases du vecteur ne sont pas les siennes.
        const auto& selVector = hashes->state->getSelVector();
        for (auto i = 0u; i < selVector.getSelSize(); i++) {
            hll->insertElement(hashes->getValue<common::hash_t>(selVector[i]));
        }
        hashes->state = nullptr;
        hashes->setAllNonNull();
    }
}

} // namespace storage
} // namespace rag3db
