#include "rag3db_arrow.h"

namespace rag3db_arrow {

ArrowSchema query_result_get_arrow_schema(const rag3db::main::QueryResult& result) {
    // Could use directly, except that we can't (yet) mark ArrowSchema as being safe to store in a
    // cxx::UniquePtr
    return *result.getArrowSchema();
}

ArrowArray query_result_get_next_arrow_chunk(rag3db::main::QueryResult& result, uint64_t chunkSize) {
    return *result.getNextArrowChunk(chunkSize);
}

} // namespace rag3db_arrow
