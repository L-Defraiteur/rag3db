# Implementation : WithFreqsAndPositionsAndOffsets dans izihawa-tantivy

> Ajouter le stockage des offsets caracteres (offset_from, offset_to) dans les postings, comme Lucene `DOCS_AND_FREQS_AND_POSITIONS_AND_OFFSETS`.

## Base path

```
packages/rag3db/extension/tantivy/izihawa-tantivy/
```

## Decisions d'architecture prises

- **Pas de nouveau OffsetSerializer** : on reutilise `PositionSerializer` (meme format bitpacked blocks de 128)
- **Interleaving** : offset_from_delta et offset_to_delta sont interleaves dans un seul stream `(from_0, to_0, from_1, to_1, ...)` → 2 * term_freq valeurs par doc
- **Fichier `.offsets` separe** : nouveau `SegmentComponent::Offsets`, CompositeFile par champ comme `.pos`
- **TermInfo etendu** : ajout `offsets_range: Range<usize>` (byte range dans `.offsets`)
- **Format arena du recorder** : `delta_doc, (position+1, offset_from+1, offset_to+1)*, 0 (END)` — le +1 permet d'utiliser 0 comme sentinel
- **Tests compat v6/v7 ignores** : le format binaire de TermInfo et TermInfoBlockMeta a change

## Progression

### 1. IndexRecordOption — FAIT

**Fichier :** `src/schema/index_record_option.rs`

- [x] Variant `WithFreqsAndPositionsAndOffsets` avec `#[serde(rename = "offsets")]`
- [x] `has_freq()` : nouveau variant dans le bras `true`
- [x] `has_positions()` : nouveau variant dans le bras `true`
- [x] `has_offsets()` : nouvelle methode, true seulement pour `WithFreqsAndPositionsAndOffsets`
- [x] `downgrade()` : tous les cas pour le nouveau variant (min de self et other)

**Aussi modifie :**
- `src/postings/per_field_postings_writer.rs` : dispatch `WithFreqsAndPositionsAndOffsets => TfPositionAndOffsetRecorder`
- `src/postings/skip.rs` : match arm ajoute (meme comportement que WithFreqsAndPositions pour l'instant)

### 2. Recorder trait + TfPositionAndOffsetRecorder — FAIT

**Fichier :** `src/postings/recorder.rs`

- [x] Trait `Recorder` : ajout `record_position_with_offsets(position, offset_from, offset_to, arena)` avec default → `record_position`
- [x] `BufferLender` : ajout `buffer_offsets_from`, `buffer_offsets_to`, methode `lend_all_with_offsets()`
- [x] `TfPositionAndOffsetRecorder` : nouveau recorder
  - Arena format : `delta_doc, (position+1, offset_from+1, offset_to+1)*, 0`
  - `record_position()` : fallback sans offsets (dummy 0,0)
  - `record_position_with_offsets()` : stocke les 3 valeurs
  - `serialize()` : extrait position_deltas + offset_from_deltas + offset_to_deltas, appelle `write_doc_with_offsets`

### 3. PostingsWriter pipeline — FAIT

**Fichiers :**
- `src/postings/postings_writer.rs`
  - [x] Trait `PostingsWriter` : ajout `subscribe_with_offsets(doc, pos, offset_from, offset_to, term, ctx)` avec default → `subscribe`
  - [x] `SpecializedPostingsWriter<Rec>` : impl `subscribe_with_offsets` qui appelle `recorder.record_position_with_offsets`
  - [x] `index_text()` : appelle maintenant `subscribe_with_offsets` avec `token.offset_from as u32` et `token.offset_to as u32`
- `src/postings/per_field_postings_writer.rs`
  - [x] Dispatch `WithFreqsAndPositionsAndOffsets => SpecializedPostingsWriter::<TfPositionAndOffsetRecorder>`
  - [x] Meme chose pour `JsonPostingsWriter`

### 4. TermInfo + format binaire — FAIT

**Fichier :** `src/postings/term_info.rs`

- [x] Ajout champ `offsets_range: Range<usize>`
- [x] `SIZE_IN_BYTES` : 4 * u32 + 3 * u64 = 40 bytes (etait 28)
- [x] `BinarySerializable` : serialize/deserialize offsets_start(u64) + offsets_len(u32)
- [x] `offsets_num_bytes()` : nouvelle methode

**Fichier :** `src/termdict/fst_termdict/term_info_store.rs`

- [x] `TermInfoBlockMeta` : ajout `offsets_offset_nbits: u8`
- [x] `SIZE_IN_BYTES` : +1 byte (4 nbits au lieu de 3)
- [x] `num_bits()` : somme des 4 nbits
- [x] `deserialize_term_info()` : extraction offsets_start/end via bitpacking
- [x] `bitpack_serialize()` : ecriture offsets_range.start
- [x] `flush_block()` : calcul offsets_end_offset, delta-encoding, ecriture end offset

**Fichier :** `src/termdict/sstable_termdict/mod.rs`

- [x] `TermInfoValueReader::load()` : lecture offsets_start + offsets_num_bytes par terme
- [x] `TermInfoValueWriter::serialize_block()` : ecriture offsets_range.start + len

### 5. SegmentComponent + fichier .offsets — FAIT

**Fichier :** `src/index/segment_component.rs`

- [x] Ajout `SegmentComponent::Offsets`
- [x] Array statique : 9 elements (etait 8)

**Fichier :** `src/index/index_meta.rs`

- [x] Extension `.offsets` pour `SegmentComponent::Offsets`

**Fichier :** `src/space_usage/mod.rs`

- [x] Match arm `Offsets => PerField(positions layout)`

### 6. Serialisation complete (ecriture disque) — FAIT

**Fichier :** `src/postings/serializer.rs`

- [x] `InvertedIndexSerializer` : ajout `offsets_write: CompositeWrite<WritePtr>`, ouverture `.offsets` dans `open()`
- [x] `new_field()` : passe `offsets_write.for_field(field)` au `FieldSerializer`
- [x] `close()` : ferme `offsets_write`
- [x] `FieldSerializer` : ajout `offsets_serializer_opt: Option<PositionSerializer>` (reutilise PositionSerializer)
- [x] `create()` : cree le offsets serializer si `has_offsets()`
- [x] `current_term_info()` : track offsets_start
- [x] `write_doc_with_offsets()` : interleave (from_delta, to_delta) et ecrit via `write_positions_delta`
- [x] `close_term()` : flush offsets serializer, track `offsets_range.end`
- [x] `close()` : ferme offsets serializer

### 7. SegmentReader — FAIT

**Fichier :** `src/index/segment_reader.rs`

- [x] Ajout champ `offsets_composite: CompositeFile`
- [x] Constructeur sync : ouvre `.offsets` (fallback CompositeFile::empty si absent)
- [x] Constructeur async (quickwit) : idem

### 8. Deserialisation (lecture disque) — FAIT

**Fichier :** `src/index/segment_reader.rs`

- [x] `inverted_index()` : passe `offsets_composite` au `InvertedIndexReader` (sync + async)
- [x] Fallback `FileSlice::empty()` si pas d'offsets pour le champ

**Fichier :** `src/index/inverted_index_reader.rs`

- [x] Champ `offsets_file_slice: FileSlice` dans `InvertedIndexReader`
- [x] `new()` : parametre `offsets_file_slice` ajoute
- [x] `empty()` : `offsets_file_slice: FileSlice::empty()`
- [x] `read_postings_from_terminfo()` : quand `has_offsets() && offsets_num_bytes() > 0`, ouvre un `PositionReader` pour les offsets
- [x] `read_postings_from_terminfo_async()` : idem version async
- [x] Passe le `PositionReader` offsets au `SegmentPostings::from_block_postings`

**Fichier :** `src/postings/segment_postings.rs`

- [x] Champ `offsets_reader: Option<PositionReader>` dans `SegmentPostings`
- [x] `empty()` : `offsets_reader: None`
- [x] `from_block_postings()` : parametre `offsets_reader` ajoute
- [x] `offsets()` : lit les offsets interleaves (2 * term_freq valeurs), de-interleave et accumule les deltas en `Vec<(u32, u32)>`
- [x] Calcul de read_offset : `2 * (position_offset + tf_before_cur)` — chaque position genere 2 offset values

**Fichier :** `src/postings/postings.rs`

- [x] Trait `Postings` : ajout `fn offsets(&mut self, output: &mut Vec<(u32, u32)>) {}` avec default vide
- [x] `impl Postings for Box<dyn Postings>` : delegation `offsets()`

**Aussi modifie :**
- `src/postings/block_segment_postings.rs` : 3 appels `from_block_postings` mis a jour (3eme arg `None`)
- `src/postings/mod.rs` : test round-trip `test_offsets_round_trip` ajoute

### 9. Skip list — REPORTE

**Fichier :** `src/postings/skip.rs`

Le skip pour les offsets n'est pas critique pour le MVP. Le skip actuel track deja le tf_sum qui permet de naviguer dans `.pos`. Pour `.offsets`, on pourrait ajouter un champ similaire mais ce n'est pas bloquant : on peut calculer l'offset dans `.offsets` a partir du tf_sum existant (2 * tf_sum valeurs interleaved).

A faire plus tard si necessaire pour les performances de seek.

## Pipeline de donnees (mis a jour)

### Ecriture (FAIT)

```
Token (offset_from, offset_to, position)    <- tokenizer-api/src/lib.rs
  |
  v
PostingsWriter::index_text()                <- src/postings/postings_writer.rs
  |  Appelle subscribe_with_offsets(doc_id, position, offset_from, offset_to, term)
  v
TfPositionAndOffsetRecorder::record_position_with_offsets()
  |  Arena: delta_doc, (pos+1, from+1, to+1)*, 0
  v
TfPositionAndOffsetRecorder::serialize()    <- src/postings/recorder.rs
  |  Extrait: position_deltas, offset_from_deltas, offset_to_deltas
  v
FieldSerializer::write_doc_with_offsets()   <- src/postings/serializer.rs
  |
  +---> PostingsSerializer (.idx)           doc_ids + term_freqs bitpacked
  +---> PositionSerializer (.pos)           position deltas bitpacked blocks 128
  +---> PositionSerializer (.offsets)       interleaved (from_delta, to_delta) bitpacked blocks 128
```

TermInfo par terme :
```
{ doc_freq, postings_range, positions_range, offsets_range }
```
Stocke dans le term dictionary (FST bitpacked ou SSTable).

### Lecture (FAIT)

```
InvertedIndexReader::read_postings_from_terminfo()
  |  Ouvre les slices via term_info.postings_range, positions_range, offsets_range
  |
  +---> BlockSegmentPostings (.idx)        doc_ids + term_freqs
  +---> PositionReader (.pos)              position deltas bitpacked blocks 128
  +---> PositionReader (.offsets)           interleaved offset deltas bitpacked blocks 128
  |
  v
SegmentPostings
  |
  +---> positions() -> Vec<u32>            <- existant (cumul des deltas)
  +---> offsets() -> Vec<(u32, u32)>       <- NOUVEAU
          read_offset = 2 * (position_offset + tf_before_cur)
          lit 2 * term_freq valeurs interleaved
          de-interleave + cumul : (cum_from, cum_to) par token
```

## Strategie de delta-encoding pour les offsets

Choix : **interleaving** dans un seul stream PositionSerializer.

```
Doc avec 3 tokens:
  Token 0: offset_from=0,  offset_to=4
  Token 1: offset_from=5,  offset_to=7
  Token 2: offset_from=8,  offset_to=15

Delta-encoding (chaque stream independamment):
  offset_from_deltas: [0, 5, 3]    (0, 5-0=5, 8-5=3)
  offset_to_deltas:   [4, 3, 8]    (4, 7-4=3, 15-7=8)

Interleaved dans .offsets:
  [0, 4, 5, 3, 3, 8]  (from_0, to_0, from_1, to_1, from_2, to_2)
```

6 valeurs pour 3 tokens (2x). Bitpacked en blocks de 128 comme les positions.

## Fichiers modifies (resume)

| Fichier | Statut | Nature du changement |
|---------|--------|---------------------|
| `src/schema/index_record_option.rs` | FAIT | Nouveau variant + methodes |
| `src/postings/recorder.rs` | FAIT | Nouveau trait method + TfPositionAndOffsetRecorder |
| `src/postings/postings_writer.rs` | FAIT | subscribe_with_offsets + index_text passe offsets |
| `src/postings/per_field_postings_writer.rs` | FAIT | Dispatch nouveau recorder |
| `src/postings/term_info.rs` | FAIT | offsets_range + serialisation |
| `src/termdict/fst_termdict/term_info_store.rs` | FAIT | offsets_offset_nbits bitpacking |
| `src/termdict/sstable_termdict/mod.rs` | FAIT | Reader/writer offsets |
| `src/index/segment_component.rs` | FAIT | SegmentComponent::Offsets |
| `src/index/index_meta.rs` | FAIT | Extension .offsets |
| `src/space_usage/mod.rs` | FAIT | Match arm Offsets |
| `src/postings/serializer.rs` | FAIT | InvertedIndexSerializer + FieldSerializer offsets |
| `src/index/segment_reader.rs` | FAIT | offsets_composite ouverture |
| `src/postings/skip.rs` | FAIT (minimal) | Match arm (meme que positions) |
| `src/compat_tests.rs` | FAIT | Tests v6/v7 ignores |
| `src/indexer/segment_writer.rs` | FAIT | Tests: offsets_range: 0..0 |
| `src/termdict/tests.rs` | FAIT | Tests: offsets_range: 0..0 |
| `src/index/inverted_index_reader.rs` | FAIT | offsets_file_slice + read_postings ouverture |
| `src/postings/segment_postings.rs` | FAIT | offsets_reader + methode offsets() |
| `src/postings/postings.rs` | FAIT | Trait offsets() method |
| `src/postings/block_segment_postings.rs` | FAIT | from_block_postings 3eme arg (tests) |
| `src/postings/mod.rs` | FAIT | Test round-trip test_offsets_round_trip |

## Tests

- 984/984 tests lib passent (7 ignores dont 2 compat format)
- tantivy_fts crate compile sans erreur
- Test round-trip `test_offsets_round_trip` : indexe "hello world" et "abc be be be abc" avec `WithFreqsAndPositionsAndOffsets`, relit positions ET offsets, verifie les valeurs exactes

## Prochaines etapes

1. **ContainsQuery** : utiliser les offsets pour la validation des separateurs — EN COURS
2. **Merge support** : verifier que le merger gere correctement les offsets lors des fusions de segments
3. **Skip list offsets** : ajouter un tracking separe pour les offsets dans le skip reader (optimisation seek)

## Propagation offsets — reste a faire

Les offsets sont stockes/lus au niveau `SegmentPostings` et `Box<dyn Postings>`, mais pas encore propages a travers les couches union utilisees par `RegexPhraseWeight` et `AutomatonPhraseWeight`.

Fichiers a modifier :

| Fichier | Quoi | Statut |
|---------|------|--------|
| `src/postings/loaded_postings.rs` | `LoadedPostings` : stocker les offsets en memoire au `load()`, impl `offsets()` | A FAIRE |
| `src/query/union/bitset_union.rs` | `BitSetPostingUnion` : impl `offsets()` (meme pattern que `append_positions_with_offset`) | A FAIRE |
| `src/query/union/simple_union.rs` | `SimpleUnion` : impl `offsets()` (meme pattern que `append_positions_with_offset`) | A FAIRE |
| `src/query/phrase_query/phrase_scorer.rs` | `PostingsWithOffset` : propager `offsets()` du posting wrappe | A FAIRE |

Chaine actuelle :
```
SegmentPostings    → offsets()  OK
Box<dyn Postings>  → offsets()  OK (delegation)
LoadedPostings     → offsets()  MANQUANT
BitSetPostingUnion → offsets()  MANQUANT
SimpleUnion        → offsets()  MANQUANT
PostingsWithOffset → offsets()  MANQUANT
```

Une fois la propagation faite, `ContainsScorer` pourra lire les offsets directement depuis les posting lists au lieu de re-tokeniser le texte stocke.
