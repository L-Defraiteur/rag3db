# Cœur C++ : évaluation du travail de Codex (16-17 novembre 2026)

Écrite le 18 novembre à la reprise, au mot de Lucie (« chaque session évalue la pertinence de ce
qu'a fait Codex »), par la session « cœur c++ ». Elle juge ce que Codex a fait dans le périmètre du
cœur, à partir de la passation `docs/coeur-cpp/16-11-2026/01-passation-pour-codex.md`.

## 1. La passe TSan du 16, lue et jugée

Six tours de `TwentyWriters.DdlEveryMinute` sous 200 Mio, sur master `64eee2e1f`, finis à 12 h 38
(`~/.cache/rag3db-moteur-notes/dict-tsan/` à luciepc). 111 avertissements, aucun SIGSEGV, **aucun
rapport ne nomme `DictionaryChunk` ni `StringChunkData`** : TSan n'a pas vu le défaut du
dictionnaire. Ce qu'il a vu, par famille :

| famille | rapports | lecture |
|---|---|---|
| `RelTable::getNumTotalRows` / `RelTable::reserveRelOffsets` | 43 / 41 | des compteurs de relations lus et réservés sans verrou commun ; connus de longue date, à ticket |
| `CatalogEntry::setTimestamp` / `getTimestamp` | 16 / 1 | l'horodatage écrit au COMMIT hors du verrou du `CatalogSet` ; 8 octets alignés, non déchirés sur x86-64 (ticket de B1) |
| **`NodeTable::addColumn` / `NodeTable::getColumn`** | 2 / 1 | **neuf** : un ADD COLUMN agrandit le vecteur `columns` (et libère son ancien tampon) pendant qu'un lecteur y lit en préparant son balayage (`NodeTableScanState::setToTable`). Une lecture après libération possible, donc un plantage possible. Les lecteurs ne prennent pas le verrou de schéma. **Ticket ouvert** : `2026-11-18-un-add-column-realloue-les-colonnes-d-une-table-sous-un-lecteur.md` |
| `NodeGroup::findChunkedGroupIdx`, `ChunkedNodeGroup::getStartRowIdx`, `GroupCollection::appendGroup` / `clear` | 2 / 2 / 1 / 1 | la liste des groupes de lignes lue pendant qu'un écrivain ou le point de reprise la change ; à lire avec le ticket précédent |
| `Checkpointer::canAutoCheckpoint` | 1 | lecture d'un compteur du journal ; bénin à première lecture |

Ce que je n'avais pas vu le 16 et que Codex a relevé : le tour 2 est rouge sur
`catalog-as-expected` (le CREATE_VECTOR_INDEX refusé par le pool plein, et pourtant son index revu
après la réouverture). Ticket de Codex, porté sur master avec cette page :
`2026-11-16-le-scenario-tsan-revoit-un-index-dont-la-creation-a-ete-refusee.md`.

## 2. Ce que Codex a touché, et le verdict

**Sur master, dans le périmètre du cœur : rien.** Aucun des 76 commits arrivés entre `927941a67a` et
la reprise ne touche `src/`, `test/`, `extension/vector`, `cmake/` ni `docs/coeur-cpp/`. Les 8 fichiers
de `docs/tickets/` qu'ils touchent sont des tickets fermés d'autres familles (garde, ties, pont).
Tout le travail du cœur est **en branches** `cx-coeur-*`, sur l'entrée C1 de la passation.

| branche ou objet | ce que c'est | verdict |
|---|---|---|
| `cx-coeur-dictionary-append-delivery-2` : témoin `70ee2719f` (`DictionaryAppendException`), correctif `888c81825` (`src/storage/table/dictionary_chunk.cpp`) | un **vrai défaut** neuf : dans `DictionaryChunk::appendString`, les octets de la chaîne sont publiés avant l'agrandissement des offsets ; si celui-ci est refusé (pool plein), la dernière chaîne existante s'allonge (« gamma » devient « gammadelta »). Témoin rouge isolé, correctif minimal (réserver les deux tampons avant toute publication), cinq cas ciblés verts | **utile, à garder** : à reposer sur master et à pousser par la voie C++ transférable, après la passe entière (c'est mon lot suivant). Il ne corrige pas le SIGSEGV (ci-dessous) ; il corrige une valeur fausse |
| le SIGSEGV reproduit dans `StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops`, avec son core symbolisé (`cx-coeur-dictionary-append-atomic`, pages 15, 21, 24) | un **test beaucoup plus petit** que `DdlEveryMinute` qui meurt de la même famille de pile, et la pile exacte : `Hashtable<DictionaryEntry>::_M_find_tr` ← `DictionaryChunk::appendString` ← **`StringChunkData::write`** ← `Column::checkpointSegment`, donc une écriture pendant le point de reprise et non `finalize`. Reproduit avec le correctif des offsets : ce correctif n'est pas la cause | **utile, à garder** : c'est le meilleur point d'entrée pour C1. Requalifie mon entrée C1 (section 3) |
| les deux rouges de sa qualification globale (page 11) : `StringRewriteAcrossAddColumn…` avec `wrongRows=228`, et `CheckpointOpenReader.OpenReaderOnRelsIsNotWaitedFor` (`[plan]`, le point de reprise fini avant la lecture) | deux constats, identiques sur la base sans le correctif (comparés octet pour octet) : ils ne viennent pas du correctif | **utile, à garder** : tickets portés sur master avec cette page (`…releve-228-anomalies.md`, `…finit-avant-la-lecture-censee-l-observer.md`) |
| tickets d'outillage de ses pilotes (`…le-confinement-efface-le-drapeau-de-la-sonde-de-chaines`, `…le-controle-du-dictionnaire-compare-un-compteur-a-lui-meme`, `…la-commande-de-decompression-du-core-utilise-une-option-refusee`, `…le-cas-de-chaines-isole-se-termine-sur-sigsegv-sans-resultat`) | des défauts de ses propres pilotes de passe, ou des doublons d'un constat déjà porté | **inutile pour le cœur** : restent dans leur branche |
| l'outillage de passe (`docs/coeur-cpp/16-11-2026/outils/c1-offsets/`, `outils/core-ro/` : contrôleurs Python, lanceurs, reçus, superviseurs, quotas par place) et les 24 pages de recettes, d'admissions et de reçus | une mécanique d'exécution lourde, propre à sa session (unités confinées, quiescence, empreintes) ; nos passes passent par `poste lourd` et la garde | **inutile pour le cœur, sans nuisance** : rien sur master, ne pas fusionner |
| six archives `.tar.gz` de preuves versionnées dans `cx-coeur-dictionary-append-atomic` | des journaux bruts dans git | **à ne pas fusionner** : leur place est le cache de luciepc |
| la lecture de ma branche `dictionary-index-after-string-2` (« ne pas pousser 256 non prouvé ») | la même conclusion que la mienne | juste |

**Verdict d'ensemble.**
1. Rien de nuisible n'a atteint master ; rien n'est à retirer.
2. Un vrai défaut neuf, prouvé rouge puis vert et prêt à livrer : l'ajout refusé qui change la dernière chaîne.
3. Le meilleur apport pour C1 : un petit test qui reproduit le SIGSEGV, et la pile exacte, qui désigne `StringChunkData::write` pendant le point de reprise.
4. Trois rouges neufs relevés et ticketés (228 anomalies, plan du lecteur ouvert, index refusé revu).
5. Le coût : une mécanique de pilotes et de reçus très lourde, et une session arrêtée sur un refus de plateforme avant la cause.

## 3. Ce que ça change à la page du 16

- **C1** passe de « en cours, lire le TSan » à : TSan lu, sans rien sur le dictionnaire. On repart du
  petit test `StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops` (plus court et
  plus fréquent que `DdlEveryMinute`) et de la pile `StringChunkData::write` ← `checkpointSegment`. La
  question devient : qui écrit dans un `StringChunkData` pendant `checkpointSegment`, et quelle vue de
  chaîne il tient. La piste « ligne comptée avant sa chaîne » reste écartée, et le correctif des
  offsets de Codex n'est pas la réponse.
- **Une entrée neuve, avant C1** : livrer le correctif des offsets (le témoin `70ee2719f` puis
  `888c81825`, reposés sur master), sous le ticket
  `2026-11-16-un-ajout-au-dictionnaire-refuse-change-la-derniere-chaine.md`.
- **Une entrée neuve, plantage** : la course `addColumn` / `getColumn` (ticket du 18).
- **Trois entrées neuves** : les tickets des 228 anomalies, du plan du lecteur ouvert et de l'index
  refusé revu, à trier par gravité (le premier peut être une réponse fausse).

## 4. Le blocage de la plateforme sur C1

Ce qui a été refusé, d'après la page et le ticket de l'orchestration de Codex
(`origin/cx-orchestration-reprise-16nov-r1`,
`docs/orchestration/tickets/2026-11-16-la-session-coeur-est-bloquee-par-le-controle-de-la-plateforme.md`) :

- **qui** : la plateforme de Codex, sur la session Codex du cœur (`01a1cb5e-…`), en état systemError ;
- **le message exact** : « This content was flagged for possible cybersecurity risk. » ;
- **la tâche en cours** : l'analyse en lecture seule de la propriété des vues de chaînes jusqu'au point
  de reprise du dictionnaire, après un SIGSEGV dont le core venait d'être lu par gdb ;
- **ni fichier ni ligne** ne sont nommés par le refus. Ce n'est donc pas un fichier du dépôt qui a été
  refusé. Le plus probable est la forme du travail : lecture de cores, gdb, adresses et pointeurs
  sauvages, outillage de confinement et de processus. Un classifieur peut prendre cela pour de
  l'exploitation de mémoire. Ce n'est qu'une lecture, qu'aucune ligne du refus ne confirme.

L'orchestration de Codex n'a pas contourné ce refus : aucune reprise, reformulation ou délégation. Je
ne le contourne pas non plus. Je ne reprends rien de la session Codex refusée. **Ce qui
reste à Lucie** : savoir que la plateforme de Codex refuse ce genre d'analyse de plantage, et décider
si C1 reste chez la session Claude du cœur (c7 me l'a rendu à la reprise) ou si elle veut autre chose.
Je poursuis C1 de mon côté, au mot de c7, par les voies du dépôt (témoins, passes, piles). Je n'ouvre
aucun core hors de ce que nos passes produisent déjà.
