#include "processor/operator/scan/scan_node_table.h"

#include "binder/expression/expression_util.h"
#include "processor/execution_context.h"
#include "storage/local_storage/local_node_table.h"
#include "storage/local_storage/local_storage.h"

using namespace rag3db::common;
using namespace rag3db::storage;

namespace rag3db {
namespace processor {

std::string ScanNodeTablePrintInfo::toString() const {
    std::string result = "Tables: ";
    for (auto& tableName : tableNames) {
        result += tableName;
        if (tableName != tableNames.back()) {
            result += ", ";
        }
    }
    if (!alias.empty()) {
        result += ",Alias: ";
        result += alias;
    }
    if (!properties.empty()) {
        result += ",Properties: ";
        result += binder::ExpressionUtil::toString(properties);
    }
    return result;
}

void ScanNodeTableSharedState::initialize(const transaction::Transaction* transaction,
    NodeTable* table, ScanNodeTableProgressSharedState& progressSharedState) {
    this->table = table;
    this->currentCommittedGroupIdx = 0;
    this->currentUnCommittedGroupIdx = 0;
    this->numCommittedNodeGroups = table->getNumCommittedNodeGroups();
    if (transaction->isWriteTransaction()) {
        if (const auto localTable =
                transaction->getLocalStorage()->getLocalTable(this->table->getTableID())) {
            auto& localNodeTable = localTable->cast<LocalNodeTable>();
            this->numUnCommittedNodeGroups = localNodeTable.getNumNodeGroups();
        }
    }
    progressSharedState.numGroups += numCommittedNodeGroups;
    // Le petit masque : la cardinalité est exacte ici, le pipeline du masqueur est fini. Un masque
    // vide passe aussi par décalages : aucune ligne à rendre.
    byOffsets = false;
    maskCursor = 0;
    maskExhausted = false;
    if (semiMask && semiMask->isEnabled()) {
        const auto numMasked = semiMask->getNumMaskedNodes();
        const auto numRows = table->getNumTotalRows(transaction);
        byOffsets = numMasked * OFFSET_MODE_RATIO < numRows;
    }
}

void ScanNodeTableSharedState::nextMorsel(NodeTableScanState& scanState,
    ScanNodeTableProgressSharedState& progressSharedState) {
    std::unique_lock lck{mtx};
    if (byOffsets) {
        // Une tranche de décalages masqués (validés ou locaux : lookupOffsets fait la part), au
        // plus une fenêtre ; épuisé, le balayage est fini — les lignes non masquées ne peuvent
        // rien apparier.
        scanState.byOffsets = true;
        scanState.maskedOffsets.clear();
        if (!maskExhausted) {
            semiMask->collectFrom(maskCursor, DEFAULT_VECTOR_CAPACITY, scanState.maskedOffsets);
            // Une tranche plus courte que demandé est la dernière. Et un masqueur peut poser un
            // décalage qui n'est pas une ligne (INVALID_OFFSET, qu'une recherche vectorielle
            // rend pour un résultat absent) : au-delà de la taille du masque, il n'est pas lu, et
            // il ne fait pas avancer le curseur au-delà de ce que le masque sait représenter
            // (4294967295 + 1 revenait à 0 sur un masque 32 bits : la même tranche sans fin).
            if (scanState.maskedOffsets.size() < DEFAULT_VECTOR_CAPACITY) {
                maskExhausted = true;
            }
            const auto maxOffset = semiMask->getMaxOffset();
            while (!scanState.maskedOffsets.empty() && scanState.maskedOffsets.back() >= maxOffset) {
                scanState.maskedOffsets.pop_back();
                maskExhausted = true;
            }
        }
        if (scanState.maskedOffsets.empty()) {
            if (!maskExhausted) {
                maskExhausted = true;
            }
            progressSharedState.numGroupsScanned += numCommittedNodeGroups;
            scanState.source = TableScanSource::NONE;
            return;
        }
        maskCursor = scanState.maskedOffsets.back() + 1;
        scanState.source = TableScanSource::COMMITTED;
        return;
    }
    if (currentCommittedGroupIdx < numCommittedNodeGroups) {
        scanState.nodeGroupIdx = currentCommittedGroupIdx++;
        progressSharedState.numGroupsScanned++;
        scanState.source = TableScanSource::COMMITTED;
        return;
    }
    if (currentUnCommittedGroupIdx < numUnCommittedNodeGroups) {
        scanState.nodeGroupIdx = currentUnCommittedGroupIdx++;
        scanState.source = TableScanSource::UNCOMMITTED;
        return;
    }
    scanState.source = TableScanSource::NONE;
}

table_id_map_t<SemiMask*> ScanNodeTable::getSemiMasks() const {
    table_id_map_t<SemiMask*> result;
    KU_ASSERT(tableInfos.size() == sharedStates.size());
    for (auto i = 0u; i < sharedStates.size(); ++i) {
        result.insert({tableInfos[i].table->getTableID(), sharedStates[i]->getSemiMask()});
    }
    return result;
}

void ScanNodeTableInfo::initScanState(TableScanState& scanState,
    const std::vector<ValueVector*>& outVectors, main::ClientContext* context) {
    auto transaction = transaction::Transaction::Get(*context);
    scanState.setToTable(transaction, table, columnIDs, copyVector(columnPredicates));
    initScanStateVectors(scanState, outVectors, MemoryManager::Get(*context));
}

void ScanNodeTable::initLocalStateInternal(ResultSet* resultSet, ExecutionContext* context) {
    ScanTable::initLocalStateInternal(resultSet, context);
    auto nodeIDVector = resultSet->getValueVector(opInfo.nodeIDPos).get();
    scanState = std::make_unique<NodeTableScanState>(nodeIDVector, outVectors, nodeIDVector->state);
    currentTableIdx = 0;
    initCurrentTable(context);
}

void ScanNodeTable::initCurrentTable(ExecutionContext* context) {
    auto& currentInfo = tableInfos[currentTableIdx];
    currentInfo.initScanState(*scanState, outVectors, context->clientContext);
    scanState->semiMask = sharedStates[currentTableIdx]->getSemiMask();
    scanState->byOffsets = false;
    scanState->maskedOffsets.clear();
    // Une autre table : le groupe en cours ne vaut plus rien (lookupOffsets ne réinitialise que
    // quand la source ou le groupe change, et une autre table peut avoir le même numéro de groupe).
    scanState->source = TableScanSource::NONE;
    scanState->nodeGroup = nullptr;
}

void ScanNodeTable::initGlobalStateInternal(ExecutionContext* context) {
    KU_ASSERT(sharedStates.size() == tableInfos.size());
    for (auto i = 0u; i < tableInfos.size(); i++) {
        sharedStates[i]->initialize(transaction::Transaction::Get(*context->clientContext),
            tableInfos[i].table->ptrCast<NodeTable>(), *progressSharedState);
    }
}

bool ScanNodeTable::getNextTuplesInternal(ExecutionContext* context) {
    const auto transaction = transaction::Transaction::Get(*context->clientContext);
    while (currentTableIdx < tableInfos.size()) {
        auto& info = tableInfos[currentTableIdx];
        if (scanState->byOffsets) {
            // Sous un petit masque : une tranche de décalages relue ligne à ligne.
            if (!scanState->maskedOffsets.empty()) {
                info.table->ptrCast<NodeTable>()->lookupOffsets(transaction, *scanState);
                scanState->maskedOffsets.clear();
                const auto outputSize = scanState->outState->getSelVector().getSelSize();
                if (outputSize > 0) {
                    info.castColumns();
                    scanState->outState->setToUnflat();
                    metrics->numOutputTuple.increase(outputSize);
                    return true;
                }
            }
        } else {
            while (info.table->scan(transaction, *scanState)) {
                const auto outputSize = scanState->outState->getSelVector().getSelSize();
                if (outputSize > 0) {
                    info.castColumns();
                    scanState->outState->setToUnflat();
                    metrics->numOutputTuple.increase(outputSize);
                    return true;
                }
            }
        }
        sharedStates[currentTableIdx]->nextMorsel(*scanState, *progressSharedState);
        if (scanState->source == TableScanSource::NONE) {
            currentTableIdx++;
            if (currentTableIdx < tableInfos.size()) {
                initCurrentTable(context);
            }
        } else if (!scanState->byOffsets) {
            info.table->initScanState(transaction, *scanState);
        }
    }
    return false;
}

double ScanNodeTable::getProgress(ExecutionContext* /*context*/) const {
    if (currentTableIdx >= tableInfos.size()) {
        return 1.0;
    }
    if (progressSharedState->numGroups == 0) {
        return 0.0;
    }
    return static_cast<double>(progressSharedState->numGroupsScanned) /
           progressSharedState->numGroups;
}

} // namespace processor
} // namespace rag3db
