#include "processor/operator/index_lookup.h"

#include "binder/expression/expression_util.h"
#include "common/assert.h"
#include "common/exception/message.h"
#include "common/types/types.h"
#include "common/utils.h"
#include "common/vector/value_vector.h"
#include "processor/warning_context.h"
#include "storage/index/hash_index.h"
#include "storage/local_storage/local_storage.h"
#include "storage/table/node_table.h"
#include "transaction/lock_manager.h"
#include "transaction/transaction.h"

using namespace rag3db::common;
using namespace rag3db::storage;

namespace rag3db {
namespace processor {

namespace {

std::optional<WarningSourceData> getWarningSourceData(
    const std::vector<ValueVector*>& warningDataVectors, sel_t pos) {
    std::optional<WarningSourceData> ret;
    if (!warningDataVectors.empty()) {
        ret.emplace(WarningSourceData::constructFromData(warningDataVectors,
            safeIntegerConversion<idx_t>(pos)));
    }
    return ret;
}

// TODO(Guodong): Add short path for unfiltered case.
bool checkNullKey(ValueVector* keyVector, offset_t vectorOffset,
    BatchInsertErrorHandler* errorHandler, const std::vector<ValueVector*>& warningDataVectors) {
    bool isNull = keyVector->isNull(vectorOffset);
    if (isNull) {
        errorHandler->handleError(ExceptionMessage::nullPKException(),
            getWarningSourceData(warningDataVectors, vectorOffset));
    }
    return !isNull;
}

struct OffsetVectorManager {
    OffsetVectorManager(ValueVector* resultVector, BatchInsertErrorHandler* errorHandler)
        : ignoreErrors(errorHandler->getIgnoreErrors()), resultVector(resultVector),
          insertOffset(0) {
        // if we are ignoring errors we may need to filter the output sel vector
        if (ignoreErrors) {
            resultVector->state->getSelVectorUnsafe().setToFiltered();
        }
    }

    ~OffsetVectorManager() {
        if (ignoreErrors) {
            resultVector->state->getSelVectorUnsafe().setSelSize(insertOffset);
        }
    }

    void insertEntry(offset_t entry, sel_t posInKeyVector) {
        auto* offsets = reinterpret_cast<offset_t*>(resultVector->getData());
        offsets[posInKeyVector] = entry;
        if (ignoreErrors) {
            // if the lookup was successful we may add the current entry to the output selection
            resultVector->state->getSelVectorUnsafe()[insertOffset] = posInKeyVector;
        }
        ++insertOffset;
    }

    bool ignoreErrors;
    ValueVector* resultVector;

    offset_t insertOffset;
};

// TODO(Guodong): Add short path for unfiltered case.
template<bool hasNoNullsGuarantee>
void fillOffsetArraysFromVector(transaction::Transaction* transaction, const IndexLookupInfo& info,
    ValueVector* keyVector, ValueVector* resultVector,
    const std::vector<ValueVector*>& warningDataVectors, BatchInsertErrorHandler* errorHandler) {
    KU_ASSERT(resultVector->dataType.getPhysicalType() == PhysicalTypeID::INT64);
    TypeUtils::visit(
        keyVector->dataType.getPhysicalType(),
        [&]<IndexHashable T>(T) {
            auto numKeys = keyVector->state->getSelVector().getSelSize();

            // fetch all the selection pos at the start
            // since we may modify the selection vector in the middle of the lookup
            std::vector<sel_t> lookupPos(numKeys);
            for (idx_t i = 0; i < numKeys; ++i) {
                lookupPos[i] = (keyVector->state->getSelVector()[i]);
            }

            OffsetVectorManager resultManager{resultVector, errorHandler};
            for (auto i = 0u; i < numKeys; i++) {
                auto pos = lookupPos[i];
                if constexpr (!hasNoNullsGuarantee) {
                    if (!checkNullKey(keyVector, pos, errorHandler, warningDataVectors)) {
                        continue;
                    }
                }
                offset_t lookupOffset = 0;
                if (!info.nodeTable->lookupPK(transaction, keyVector, pos, lookupOffset)) {
                    TypeUtils::visit(keyVector->dataType, [&]<typename type>(type) {
                        errorHandler->handleError(
                            ExceptionMessage::nonExistentPKException(
                                TypeUtils::toString(keyVector->getValue<type>(pos), keyVector)),
                            getWarningSourceData(warningDataVectors, pos));
                    });
                } else {
                    resultManager.insertEntry(lookupOffset, pos);
                }
            }
        },
        [&](auto) { KU_UNREACHABLE; });
}
} // namespace

std::string IndexLookupPrintInfo::toString() const {
    std::string result = "Indexes: ";
    result += binder::ExpressionUtil::toString(expressions);
    return result;
}

bool IndexLookup::getNextTuplesInternal(ExecutionContext* context) {
    if (!children[0]->getNextTuple(context)) {
        return false;
    }
    for (auto& info : infos) {
        info.keyEvaluator->evaluate();
        lookup(transaction::Transaction::Get(*context->clientContext), info);
    }
    localState->errorHandler->flushStoredErrors();
    return true;
}

// Le versement qui suit (initGlobalStateInternal) donne aux nœuds locaux leurs décalages
// définitifs en milieu de transaction. Un autre écrivain qui validerait ensuite des nœuds dans
// la même table les placerait derrière eux ; le rejeu, qui repasse les transactions dans l'ordre
// de leurs commits, donnerait à ceux de l'autre les décalages que celle-ci avait en service, et
// ses relations journalisées désigneraient les nœuds de l'autre (témoin ProvisionalOffsets.
// TwoFlushesInOneTransaction…, 8 novembre 2026). Le versement tient donc l'index de la table en
// exclusif jusqu'à la fin de la transaction, comme un COPY de nœuds
// (NodeBatchInsert::acquireLocksBeforeExecution) : l'autre écrivain attend, ou est refusé par le
// délai. Pris ici, avant l'exécution, et non sous le mutex de la tâche, où l'attente gelait
// l'ordonnanceur.
void IndexLookup::acquireLocksBeforeExecution(ExecutionContext* context) {
    const auto transaction = transaction::Transaction::Get(*context->clientContext);
    const auto localStorage = transaction->getLocalStorage();
    if (!transaction->usesLocks() || !localStorage) {
        return;
    }
    for (auto& info : infos) {
        const auto tableID = info.nodeTable->getTableID();
        const auto localTable = localStorage->getLocalTable(tableID);
        if (localTable && localTable->getNumTotalRows() > 0) {
            transaction->acquireLock(transaction::LockResource::index(tableID),
                transaction::LockMode::EXCLUSIVE);
        }
    }
}

void IndexLookup::initGlobalStateInternal(ExecutionContext* context) {
    // Les nœuds que la transaction a créés par le chemin ordinaire vivent dans son stockage local,
    // à des décalages provisoires. Résolus par leur clé, ils donneraient ces décalages au COPY de
    // relations, qui les écrit tels quels : si un autre écrivain valide des nœuds dans la même
    // table avant la validation de celle-ci, les nœuds atterrissent plus loin et les relations du
    // COPY désignent ceux de l'autre (ticket 2026-10-05-copy-de-relations-vers-des-noeuds-locaux-
    // sans-remappage). Ils sont donc versés dans leur table avant la première résolution, comme
    // avant un COPY de nœuds : ils y prennent leurs décalages définitifs, et les relations locales
    // qui les désignent sont remappées. Une fois par opérateur, avant tout fil.
    const auto transaction = transaction::Transaction::Get(*context->clientContext);
    if (const auto localStorage = transaction->getLocalStorage()) {
        for (auto& info : infos) {
            localStorage->flushNodeTable(info.nodeTable->getTableID());
        }
    }
}

void IndexLookup::initLocalStateInternal(ResultSet* resultSet, ExecutionContext* context) {
    auto errorHandler = std::make_unique<BatchInsertErrorHandler>(context,
        WarningContext::Get(*context->clientContext)->getIgnoreErrorsOption());
    localState = std::make_unique<IndexLookupLocalState>(std::move(errorHandler));
    for (auto& pos : warningDataVectorPos) {
        localState->warningDataVectors.push_back(resultSet->getValueVector(pos).get());
    }
    for (auto& info : infos) {
        info.keyEvaluator->init(*resultSet, context->clientContext);
    }
}

void IndexLookup::lookup(transaction::Transaction* transaction, const IndexLookupInfo& info) {
    auto keyVector = info.keyEvaluator->resultVector.get();
    auto resultVector = resultSet->getValueVector(info.resultVectorPos).get();

    if (keyVector->hasNoNullsGuarantee()) {
        fillOffsetArraysFromVector<true>(transaction, info, keyVector, resultVector,
            localState->warningDataVectors, localState->errorHandler.get());
    } else {
        fillOffsetArraysFromVector<false>(transaction, info, keyVector, resultVector,
            localState->warningDataVectors, localState->errorHandler.get());
    }
}

} // namespace processor
} // namespace rag3db
