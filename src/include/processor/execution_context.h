#pragma once

#include "common/profiler.h"

namespace rag3db {
namespace main {
class ClientContext;
}
namespace processor {

struct RAG3DB_API ExecutionContext {
    uint64_t queryID;
    common::Profiler* profiler;
    main::ClientContext* clientContext;

    ExecutionContext(common::Profiler* profiler, main::ClientContext* clientContext,
        uint64_t queryID)
        : queryID{queryID}, profiler{profiler}, clientContext{clientContext} {}
};

} // namespace processor
} // namespace rag3db
