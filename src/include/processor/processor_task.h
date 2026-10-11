#pragma once

#include <chrono>

#include "common/task_system/task.h"
#include "processor/operator/sink.h"

namespace rag3db {
namespace processor {

class ProcessorTask : public common::Task {
    friend class QueryProcessor;

public:
    ProcessorTask(Sink* sink, ExecutionContext* executionContext);

    void run() override;

    void finalize() override;

    bool terminate() override;

    // L'opérateur source de la tâche (le bout du pipeline), et sa cardinalité estimée.
    PhysicalOperator* getSource() const;

private:
    bool sharedStateInitialized;
    Sink* sink;
    ExecutionContext* executionContext;
    std::chrono::steady_clock::time_point createdAt;
};

} // namespace processor
} // namespace rag3db
