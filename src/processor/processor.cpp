#include "processor/processor.h"

#include <cstdlib>

#include <algorithm>
#include <optional>

#include "common/system_config.h"
#include "common/task_system/progress_bar.h"
#include "main/query_result.h"
#include "processor/operator/sink.h"
#include "processor/physical_plan.h"
#include "processor/processor_task.h"

using namespace rag3db::common;
using namespace rag3db::storage;

namespace rag3db {
namespace processor {

#if defined(__APPLE__)
QueryProcessor::QueryProcessor(uint64_t numThreads, uint32_t threadQos)
    : traceTasks{std::getenv("RAG3DB_TRACE_TASKS") != nullptr} {
    taskScheduler = std::make_unique<TaskScheduler>(numThreads, threadQos);
}
#else
QueryProcessor::QueryProcessor(uint64_t numThreads)
    : traceTasks{std::getenv("RAG3DB_TRACE_TASKS") != nullptr} {
    taskScheduler = std::make_unique<TaskScheduler>(numThreads);
}
#endif

std::unique_ptr<main::QueryResult> QueryProcessor::execute(PhysicalPlan* physicalPlan,
    ExecutionContext* context) {
    auto lastOperator = physicalPlan->lastOperator.get();
    // The root pipeline(task) consists of operators and its prevOperator only, because we
    // expect to have linear plans. For binary operators, e.g., HashJoin, we  keep probe and its
    // prevOperator in the same pipeline, and decompose build and its prevOperator into another
    // one.
    auto sink = lastOperator->ptrCast<Sink>();
    auto task = std::make_shared<ProcessorTask>(sink, context);
    for (auto i = (int64_t)sink->getNumChildren() - 1; i >= 0; --i) {
        decomposePlanIntoTask(sink->getChild(i), task.get(), context);
    }
    initTask(task.get());
    acquireLocksBeforeExecution(task.get(), context);
    auto progressBar = ProgressBar::Get(*context->clientContext);
    progressBar->startProgress(context->queryID);
    taskScheduler->scheduleTaskAndWaitOrError(task, context);
    progressBar->endProgress(context->queryID);
    return sink->getQueryResult();
}

void QueryProcessor::decomposePlanIntoTask(PhysicalOperator* op, Task* task,
    ExecutionContext* context) {
    if (op->isSource()) {
        ProgressBar::Get(*context->clientContext)->addPipeline();
    }
    if (op->isSink()) {
        auto childTask = std::make_unique<ProcessorTask>(ku_dynamic_cast<Sink*>(op), context);
        for (auto i = (int64_t)op->getNumChildren() - 1; i >= 0; --i) {
            decomposePlanIntoTask(op->getChild(i), childTask.get(), context);
        }
        task->addChildTask(std::move(childTask));
    } else {
        // Schedule the right most side (e.g., build side of the hash join) first.
        for (auto i = (int64_t)op->getNumChildren() - 1; i >= 0; --i) {
            decomposePlanIntoTask(op->getChild(i), task, context);
        }
    }
}

// Les verrous de tout le plan, pris sur le fil du client avant qu'une tâche ne soit
// ordonnancée (PhysicalOperator::acquireLocksBeforeExecution).
void QueryProcessor::acquireLocksBeforeExecution(Task* task, ExecutionContext* context) {
    for (auto& child : task->children) {
        acquireLocksBeforeExecution(child.get(), context);
    }
    ku_dynamic_cast<ProcessorTask*>(task)->sink->acquireLocksBeforeExecution(context);
}

// Les fils d'une tâche proportionnés à la cardinalité estimée par le planificateur (page
// coeur-cpp/12) : un pipeline qui lit trois lignes n'a pas besoin de trente-deux fils, et leur mise
// en route coûte plus que le travail (impact : -47 % à un fil, mesure de F). La plus grande
// cardinalité estimée le long du pipeline (la source, ou une extension qui multiplie) donne le
// nombre de fils, un par tranche de ROWS_PER_THREAD lignes, sous le plafond `threads` de la
// connexion. Un opérateur sans estimation (un CALL, le lecteur d'un COPY) garde le plafond : jamais
// moins de fils qu'aujourd'hui sur ce qu'on ne sait pas estimer. Comme PostgreSQL, qui décide
// le nombre d'ouvriers au plan sur la taille estimée ; Neo4j n'en donne aucun à une petite requête.
static std::optional<uint64_t> threadsForEstimatedRows(PhysicalOperator* sink) {
    constexpr uint64_t ROWS_PER_THREAD = common::DEFAULT_VECTOR_CAPACITY;
    uint64_t rows = 0;
    PhysicalOperator* op = sink;
    while (true) {
        const auto estimate = op->getEstimatedCardinality();
        if (!estimate.has_value()) {
            return std::nullopt;
        }
        rows = std::max(rows, *estimate);
        if (op->isSource()) {
            break;
        }
        op = op->getChild(0);
    }
    return std::max<uint64_t>(1, (rows + ROWS_PER_THREAD - 1) / ROWS_PER_THREAD);
}

void QueryProcessor::initTask(Task* task) {
    auto processorTask = ku_dynamic_cast<ProcessorTask*>(task);
    PhysicalOperator* op = processorTask->sink;
    while (!op->isSource()) {
        if (!op->isParallel()) {
            task->setSingleThreadedTask();
        }
        op = op->getChild(0);
    }
    if (!op->isParallel()) {
        task->setSingleThreadedTask();
    }
    if (const auto threads = threadsForEstimatedRows(processorTask->sink)) {
        task->limitNumThreads(*threads);
    }
    for (auto& child : task->children) {
        initTask(child.get());
    }
}

} // namespace processor
} // namespace rag3db
