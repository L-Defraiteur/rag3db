#pragma once

#include <mutex>

#include "common/data_chunk/data_chunk.h"
#include "common/file_system/file_info.h"
#include "common/types/types.h"
#include "parquet_types.h"
#include "processor/operator/persistent/writer/parquet/column_writer.h"
#include "processor/result/factorized_table.h"
#include "protocol/TProtocol.h"

namespace rag3db {
namespace main {
class ClientContext;
}

namespace processor {

class ParquetWriterTransport : public rag3db_apache::thrift::protocol::TTransport {
public:
    explicit ParquetWriterTransport(common::FileInfo* fileInfo, common::offset_t& offset)
        : fileInfo{fileInfo}, offset{offset} {}

    inline bool isOpen() const override { return true; }

    void open() override {}

    void close() override {}

    inline void write_virt(const uint8_t* buf, uint32_t len) override {
        fileInfo->writeFile(buf, len, offset);
        offset += len;
    }

private:
    common::FileInfo* fileInfo;
    common::offset_t& offset;
};

struct PreparedRowGroup {
    rag3db_parquet::format::RowGroup rowGroup;
    std::vector<std::unique_ptr<ColumnWriterState>> states;
};

class ParquetWriter {
public:
    ParquetWriter(std::string fileName, std::vector<common::LogicalType> types,
        std::vector<std::string> names, rag3db_parquet::format::CompressionCodec::type codec,
        main::ClientContext* context);

    inline common::offset_t getOffset() const { return fileOffset; }
    inline void write(const uint8_t* buf, uint32_t len) {
        fileInfo->writeFile(buf, len, fileOffset);
        fileOffset += len;
    }
    inline rag3db_parquet::format::CompressionCodec::type getCodec() { return codec; }
    inline rag3db_apache::thrift::protocol::TProtocol* getProtocol() { return protocol.get(); }
    inline rag3db_parquet::format::Type::type getParquetType(uint64_t schemaIdx) {
        return fileMetaData.schema[schemaIdx].type;
    }
    void flush(FactorizedTable& ft);
    void finalize();
    static rag3db_parquet::format::Type::type convertToParquetType(const common::LogicalType& type);
    static void setSchemaProperties(const common::LogicalType& type,
        rag3db_parquet::format::SchemaElement& schemaElement);

private:
    void prepareRowGroup(FactorizedTable& ft, PreparedRowGroup& result);
    void flushRowGroup(PreparedRowGroup& rowGroup);
    void readFromFT(FactorizedTable& ft, std::vector<common::ValueVector*> vectorsToRead,
        uint64_t& numTuplesRead);
    inline uint64_t getNumTuples(common::DataChunk* unflatChunk) {
        return unflatChunk->getNumValueVectors() != 0 ?
                   unflatChunk->state->getSelVector().getSelSize() :
                   1;
    }

private:
    std::string fileName;
    std::vector<common::LogicalType> types;
    std::vector<std::string> columnNames;
    rag3db_parquet::format::CompressionCodec::type codec;
    std::unique_ptr<common::FileInfo> fileInfo;
    std::shared_ptr<rag3db_apache::thrift::protocol::TProtocol> protocol;
    rag3db_parquet::format::FileMetaData fileMetaData;
    std::mutex lock;
    std::vector<std::unique_ptr<ColumnWriter>> columnWriters;
    common::offset_t fileOffset;
    storage::MemoryManager* mm;
};

} // namespace processor
} // namespace rag3db
