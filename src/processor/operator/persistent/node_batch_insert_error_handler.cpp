#include "processor/operator/persistent/node_batch_insert_error_handler.h"

#include "processor/execution_context.h"
#include "storage/table/node_table.h"

using namespace rag3db::common;

namespace rag3db {
namespace processor {

NodeBatchInsertErrorHandler::NodeBatchInsertErrorHandler(ExecutionContext* context,
    LogicalTypeID pkType, storage::NodeTable* nodeTable, bool ignoreErrors,
    std::shared_ptr<row_idx_t> sharedErrorCounter, std::mutex* sharedErrorCounterMtx)
    : nodeTable(nodeTable), context(context),
      keyVector(std::make_shared<ValueVector>(pkType,
          storage::MemoryManager::Get(*context->clientContext))),
      offsetVector(std::make_shared<ValueVector>(LogicalTypeID::INTERNAL_ID,
          storage::MemoryManager::Get(*context->clientContext))),
      baseErrorHandler(context, ignoreErrors, sharedErrorCounter, sharedErrorCounterMtx) {
    keyVector->state = DataChunkState::getSingleValueDataChunkState();
    offsetVector->state = DataChunkState::getSingleValueDataChunkState();
}

void NodeBatchInsertErrorHandler::deleteCurrentErroneousRow() {
    storage::NodeTableDeleteState deleteState{
        *offsetVector,
        *keyVector,
    };
    // Pas au journal. La ligne écartée a été écrite par le COPY, dont les lignes ne sont pas
    // journalisées quand il en écarte : il garde alors son point de reprise. Sa suppression
    // seule au journal serait rejouée, après une mort avant la fin de ce point de reprise,
    // sur une table qui n'a pas la ligne — la base ne se rouvrait plus (plantage au rejeu).
    deleteState.logToWAL = false;
    nodeTable->delete_(transaction::Transaction::Get(*context->clientContext), deleteState);
}

void NodeBatchInsertErrorHandler::flushStoredErrors() {
    baseErrorHandler.flushStoredErrors();
}

} // namespace processor
} // namespace rag3db
