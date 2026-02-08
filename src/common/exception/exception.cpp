#include "common/exception/exception.h"

#ifdef RAG3DB_BACKTRACE
#include <cpptrace/cpptrace.hpp>
#endif

namespace rag3db {
namespace common {

Exception::Exception(std::string msg) : exception(), exception_message_(std::move(msg)) {
#ifdef RAG3DB_BACKTRACE
    cpptrace::generate_trace(1 /*skip this function's frame*/).print();
#endif
}

} // namespace common
} // namespace rag3db
