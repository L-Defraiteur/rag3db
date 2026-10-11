#include "processor/processor_task.h"

#include <cstdio>
#include <cstdlib>
#include <string>

#include "common/task_system/progress_bar.h"
#include "main/client_context.h"
#include "main/settings.h"
#include "processor/execution_context.h"
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
    // venus, la source et sa cardinalité estimée, la durée. Sous RAG3DB_TRACE_TASKS seulement.
    static const bool trace = std::getenv("RAG3DB_TRACE_TASKS") != nullptr;
    if (trace) {
        const auto source = getSource();
        const auto card = source->getEstimatedCardinality();
        const auto ms = std::chrono::duration_cast<std::chrono::microseconds>(
                            std::chrono::steady_clock::now() - createdAt)
                            .count() /
                        1000.0;
        fprintf(stderr, "[tache] source=%s card=%s puits=%s fils=%llu/%llu duree=%.2f ms\n",
            PhysicalOperatorUtils::operatorTypeToString(source->getOperatorType()).c_str(),
            card.has_value() ? std::to_string(*card).c_str() : "?",
            PhysicalOperatorUtils::operatorTypeToString(sink->getOperatorType()).c_str(),
            static_cast<unsigned long long>(numThreadsRegistered),
            static_cast<unsigned long long>(maxNumThreads), ms);
    }
}

bool ProcessorTask::terminate() {
    return sink->terminate();
}

} // namespace processor
} // namespace rag3db
