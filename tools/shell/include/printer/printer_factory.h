#include <memory>

#include "printer/printer.h"

namespace rag3db {
namespace main {

class PrinterFactory {
public:
    static std::unique_ptr<Printer> getPrinter(PrinterType type);
};

} // namespace main
} // namespace rag3db
