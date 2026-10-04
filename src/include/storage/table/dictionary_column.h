#pragma once

#include <unordered_map>

#include "dictionary_chunk.h"
#include "storage/table/column.h"
#include "storage/table/column_chunk_data.h"
#include "storage/table/string_chunk_data.h"

namespace rag3db {
namespace storage {

class DictionaryColumn {
public:
    // Le nom du refus de lire une chaîne dont les décalages lus dans le dictionnaire sont
    // incohérents (fin avant le début, ou fin au-delà des données). Une garde de mémoire,
    // active en Release ; le message porte les nombres.
    static constexpr const char* DICTIONARY_OFFSETS_OUT_OF_ORDER =
        "Dictionary offsets out of order";

    // Un indice de chaîne au-delà du dictionnaire de son segment. Refusé avant de dimensionner
    // quoi que ce soit d'après lui : la plage d'indices à relire commande une réservation.
    static constexpr const char* DICTIONARY_INDEX_OUT_OF_RANGE = "Dictionary index out of range";

    DictionaryColumn(const std::string& name, FileHandle* dataFH, MemoryManager* mm,
        ShadowFile* shadowFile, bool enableCompression);

    void scan(const SegmentState& state, DictionaryChunk& dictChunk) const;
    // Offsets to scan should be a sorted list of pairs mapping the index of the entry in the string
    // dictionary (as read from the index column) to the output index in the result vector to store
    // the string.
    template<class Result>
    void scan(const SegmentState& offsetState, const SegmentState& dataState,
        std::vector<std::pair<DictionaryChunk::string_index_t, uint64_t>>& offsetsToScan,
        Result* result, const ColumnChunkMetadata& indexMeta) const;

    // Ajoute au dictionnaire d'un bloc les chaînes du segment désignées par leurs indices
    // (distincts), et rend pour chacun l'indice de sa chaîne dans ce dictionnaire.
    std::unordered_map<DictionaryChunk::string_index_t, DictionaryChunk::string_index_t>
    scanToChunk(const SegmentState& offsetState, const SegmentState& dataState,
        std::vector<DictionaryChunk::string_index_t> indicesToScan,
        DictionaryChunk& dictChunk) const;

    DictionaryChunk::string_index_t append(const DictionaryChunk& dictChunk, SegmentState& state,
        std::string_view val) const;

    bool canCommitInPlace(const SegmentState& state, uint64_t numNewStrings,
        uint64_t totalStringLengthToAdd) const;

    Column* getDataColumn() const { return dataColumn.get(); }
    Column* getOffsetColumn() const { return offsetColumn.get(); }

private:
    void scanOffsets(const SegmentState& state, DictionaryChunk::string_offset_t* offsets,
        uint64_t index, uint64_t numValues, uint64_t dataSize) const;
    void scanValue(const SegmentState& dataState, uint64_t startOffset, uint64_t endOffset,
        common::ValueVector* resultVector, uint64_t offsetInVector) const;

    static bool canDataCommitInPlace(const SegmentState& dataState,
        uint64_t totalStringLengthToAdd);
    bool canOffsetCommitInPlace(const SegmentState& offsetState, const SegmentState& dataState,
        uint64_t numNewStrings, uint64_t totalStringLengthToAdd) const;

private:
    // The offset column stores the offsets for each index, and the data column stores the data in
    // order. Values are never removed from the dictionary during in-place updates, only appended to
    // the end.
    std::unique_ptr<Column> dataColumn;
    std::unique_ptr<Column> offsetColumn;
};

} // namespace storage
} // namespace rag3db
