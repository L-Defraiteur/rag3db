#include "processor/processor_task.h"

#include <cstdio>
#include <string>

#include "common/task_system/progress_bar.h"
#include "main/client_context.h"
#include "main/database.h"
#include "main/settings.h"
#include "processor/execution_context.h"
#include "processor/processor.h"
#include "storage/buffer_manager/memory_manager.h"

using namespace rag3db::common;

namespace rag3db {
namespace processor {

ProcessorTask::ProcessorTask(Sink* sink, ExecutionContext* executionContext)
    : Task{executionContext->clientContext->getCurrentSetting(main::ThreadsSetting::name)
               .getValue<uint64_t>()},
      sharedStateInitialized{false}, sink{sink}, executionContext{executionContext},
      createdAt{std::chrono::steady_clock::now()} {}

PhysicalOperator* ProcessorTask::getSource() const {
    PhysicalOperator* op = sink;
    while (!op->isSource()) {
        op = op->getChild(0);
    }
    return op;
}

void ProcessorTask::run() {
    // We need the lock when cloning because multiple threads can be accessing to clone,
    // which is not thread safe
    lock_t lck{taskMtx};
    if (!sharedStateInitialized) {
        sink->initGlobalState(executionContext);
        sharedStateInitialized = true;
    }
    auto taskRoot = sink->copy();
    lck.unlock();
    auto resultSet =
        sink->getResultSet(storage::MemoryManager::Get(*executionContext->clientContext));
    taskRoot->ptrCast<Sink>()->execute(resultSet.get(), executionContext);
}

void ProcessorTask::finalize() {
    ProgressBar::Get(*executionContext->clientContext)->finishPipeline(executionContext->queryID);
    sink->finalize(executionContext);
    // La trace d'une tâche (page coeur-cpp/12, la mesure avant la forme) : les fils donnés et
    // venus, la source et sa cardinalité estimée, la durée. Sous RAG3DB_TRACE_TASKS seulement,
    // lue à l'ouverture de la base (QueryProcessor).
    if (executionContext->clientContext->getDatabase()->getQueryProcessor()->tracesTasks()) {
        const auto source = getSource();
        const auto card = source->getEstimatedCardinality();
        const auto ms = std::chrono::duration_cast<std::chrono::microseconds>(
                            std::chrono::steady_clock::now() - createdAt)
                            .count() /
                        1000.0;
        // Le premier opérateur du pipeline sans estimation, s'il y en a un : c'est lui qui garde
        // le plafond des fils (QueryProcessor::initTask).
        std::string unknown;
        for (PhysicalOperator* op = sink; unknown.empty(); op = op->getChild(0)) {
            if (!op->getEstimatedCardinality().has_value()) {
                unknown = PhysicalOperatorUtils::operatorTypeToString(op->getOperatorType());
            }
            if (op->isSource()) {
                break;
            }
        }
        fprintf(stderr,
            "[tache] source=%s card=%s puits=%s fils=%llu/%llu duree=%.2f ms%s%s\n",
            PhysicalOperatorUtils::operatorTypeToString(source->getOperatorType()).c_str(),
            card.has_value() ? std::to_string(*card).c_str() : "?",
            PhysicalOperatorUtils::operatorTypeToString(sink->getOperatorType()).c_str(),
            static_cast<unsigned long long>(numThreadsRegistered),
            static_cast<unsigned long long>(maxNumThreads), ms, unknown.empty() ? "" : " sans=",
            unknown.c_str());
    }
}

bool ProcessorTask::terminate() {
    return sink->terminate();
}

} // namespace processor
} // namespace rag3db
