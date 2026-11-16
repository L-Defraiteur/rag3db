// Diagnostic privé : extraction binaire, jamais de writer ni de conversion toString(BLOB).
#include "rag3db.hpp"
#include <cstdint>
#include <iostream>
#include <stdexcept>
#include <string>
using namespace rag3db::main;
using rag3db::common::Value;
using rag3db::common::LogicalType;
using rag3db::common::LogicalTypeID;
static void u64(uint64_t value) {
    for (unsigned i = 0; i < 8; ++i) std::cout.put(static_cast<char>((value >> (8 * i)) & 255));
}
static void row(const Value& key, const Value& blob) {
    if (key.isNull() || blob.isNull() || key.getDataType().getLogicalTypeID() != LogicalTypeID::STRING ||
        blob.getDataType().getLogicalTypeID() != LogicalTypeID::BLOB) throw std::runtime_error("non STRING/BLOB row");
    const auto name = key.getValue<std::string>();
    const auto bytes = blob.getValue<std::string>();
    if (name.empty() || name.find('\0') != std::string::npos) throw std::runtime_error("invalid blob key");
    u64(name.size()); u64(bytes.size());
    std::cout.write(name.data(), name.size()); std::cout.write(bytes.data(), bytes.size());
    if (!std::cout) throw std::runtime_error("binary write failed");
}
int main(int argc, char** argv) {
    try {
        if (argc == 2 && std::string(argv[1]) == "--self-test") {
            const std::string bytes = { '\0', static_cast<char>(255), '\\', '\n', '"' };
            std::cout.write("RFTSB001", 8); u64(1);
            row(Value(std::string("index/blob")), Value(LogicalType::BLOB(), bytes));
            std::cerr << "TYPED_BINARY_SELFTEST\n";
            return 0;
        }
        if (argc != 2) throw std::runtime_error("usage: dump-blobs BASE | --self-test");
        SystemConfig config; config.readOnly = true; config.bufferPoolSize = 1ull << 30;
        Database database(argv[1], config); Connection connection(&database);
        auto build = connection.query("CALL BUILD_ID() RETURN *");
        if (!build->isSuccess()) throw std::runtime_error(build->getErrorMessage());
        while (build->hasNext()) std::cerr << "BUILD_ID\t" << build->getNext()->toString();
        // Exactement la visibilité de CypherBlobStore, sans ALTER/prepare/save.
        auto result = connection.query("MATCH (b:_index_blobs) WHERE b._deleted_gen IS NULL OR b._deleted_gen < 0 RETURN b._key, b._data ORDER BY b._key");
        if (!result->isSuccess()) throw std::runtime_error(result->getErrorMessage());
        const auto count = result->getNumTuples();
        std::cout.write("RFTSB001", 8); u64(count); uint64_t seen = 0;
        while (result->hasNext()) {
            auto values = result->getNext(); row(*values->getValue(0), *values->getValue(1)); ++seen;
        }
        if (seen != count || !std::cout) throw std::runtime_error("row count or binary stream differs");
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "BLOB_DUMP_REFUSED\t" << error.what() << '\n'; return 1;
    }
}
