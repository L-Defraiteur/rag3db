#pragma once

#include "hnsw_config.h"
#include "storage/table/list_chunk_data.h"

namespace rag3db {
namespace catalog {
class TableCatalogEntry;
class NodeTableCatalogEntry;
} // namespace catalog

namespace main {
class ClientContext;
}

namespace vector_extension {
template<typename T>
concept VectorElementType = std::is_floating_point_v<T>;
using metric_func_t = std::function<double(const void*, const void*, uint32_t)>;

struct HNSWIndexUtils {
    enum class RAG3DB_API IndexOperation { CREATE, QUERY, DROP };

    // Le nom du refus de chercher dans un index, ou de le recréer sans l'avoir retiré, quand
    // il est au catalogue mais que la table ne le porte plus : le rejeu du journal a écrit
    // dans la table pendant que l'extension n'était pas chargée. L'appelant le reconnaît à
    // ce fragment, retire l'index (DROP_VECTOR_INDEX) et le rebâtit.
    static constexpr const char* INDEX_BEHIND_ITS_TABLE = "is behind its table";

    // Deux gardes de mémoire, actives en Release. Un décalage de nœud hors du tableau des
    // « déjà visités » d'une recherche, ou hors du graphe en mémoire d'une construction,
    // écrivait hors de son bloc (corruption de tas, ticket du 4 octobre 2026). Chacune a son
    // fragment, pour savoir laquelle a parlé ; le message porte le décalage et la taille.
    static constexpr const char* OFFSET_BEYOND_VISITED_SET = "is beyond the visited set";
    static constexpr const char* OFFSET_BEYOND_IN_MEM_GRAPH = "is beyond the in-memory graph";
    [[noreturn]] static void throwOffsetBeyondVisitedSet(common::offset_t offset,
        common::offset_t size);
    [[noreturn]] static void throwOffsetBeyondInMemGraph(common::offset_t offset,
        common::offset_t numNodes);

    static bool indexExists(const main::ClientContext& context,
        const transaction::Transaction* transaction, const catalog::TableCatalogEntry* tableEntry,
        const std::string& indexName);

    static bool validateIndexExistence(const main::ClientContext& context,
        const catalog::TableCatalogEntry* tableEntry, const std::string& indexName,
        IndexOperation indexOperation,
        common::ConflictAction conflictAction = common::ConflictAction::ON_CONFLICT_THROW);

    static catalog::NodeTableCatalogEntry* bindNodeTable(const main::ClientContext& context,
        const std::string& tableName);

    static void validateAutoTransaction(const main::ClientContext& context,
        const std::string& funcName);

    static metric_func_t getMetricsFunction(MetricType metric, const common::LogicalType& type);

    static void validateColumnType(const catalog::TableCatalogEntry& tableEntry,
        const std::string& columnName);

    static std::string getUpperGraphTableName(common::table_id_t tableID,
        const std::string& indexName) {
        return "_" + std::to_string(tableID) + "_" + indexName + "_UPPER";
    }
    static std::string getLowerGraphTableName(common::table_id_t tableID,
        const std::string& indexName) {
        return "_" + std::to_string(tableID) + "_" + indexName + "_LOWER";
    }

    template<typename T>
    static T* getEmbedding(const storage::ColumnChunkData& embeddings, common::offset_t offset,
        common::length_t dimension) {
        auto& listChunk = embeddings.cast<storage::ListChunkData>();
        return &listChunk.getDataColumnChunk()->getData<T>()[offset * dimension];
    }

private:
    static void validateColumnType(const common::LogicalType& type);
};

} // namespace vector_extension
} // namespace rag3db
