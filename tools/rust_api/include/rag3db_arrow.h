#pragma once

#include "rust/cxx.h"
#ifdef RAG3DB_BUNDLED
#include "main/rag3db.h"
#else
#include <rag3db.hpp>
#endif

namespace rag3db_arrow {

ArrowSchema query_result_get_arrow_schema(const rag3db::main::QueryResult& result);
ArrowArray query_result_get_next_arrow_chunk(rag3db::main::QueryResult& result, uint64_t chunkSize);

} // namespace rag3db_arrow
