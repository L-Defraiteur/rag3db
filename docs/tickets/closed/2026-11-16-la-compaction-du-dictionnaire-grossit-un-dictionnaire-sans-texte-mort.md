# La compaction du dictionnaire grossit un dictionnaire qui n'a aucun texte mort

- **État** : fermé (16 novembre 2026), sans défaut : mesuré par lucivy le jour même, voir « La mesure ». La
  compaction rend le dictionnaire d'un index neuf des mêmes documents, à 0,25 % près. C'est la disposition d'avant,
  une génération par commit, qui était 3 % plus petite, et non la compaction qui grossit.
- **Gravité** : faible. Les réponses ne changent pas. Mais une maintenance qui n'a rien à retirer fait grossir le
  dictionnaire de 3,7 % : `maintain` doit rendre au moins ce qu'il a trouvé, jamais davantage.
- **Atteignable en service** : oui, chaque nuit. La maintenance nocturne de rag3weaver (`fts.maintenance`, 03:30)
  appelle `maintain` avec `compact` sur toutes les tables de plein texte.
- **Touche rag3weaver** : non (lucivy seule), mais c'est l'hôte de rag3weaver qui en paie les octets.
- **Famille** : produit
- **Pour** : lucivy

## Ce qui se passe

La première maintenance nocturne a tourné le 16 novembre 2026 à 03:30 sur les bases de Lucie, sans aucun refus.
Lu dans `daemon_status`, à la clé `fts_maintenance` de l'état d'index de chaque backend. Sur les tables qui
avaient des textes morts, le dictionnaire baisse comme prévu ; `Scope` de l'atelier passe ainsi de 559,0 à 409,8 Mo.

Une table fait l'inverse : `Lucivy_Source` de la mémoire longue
(`~/.local/share/rag3weaver/memoire-longue.rag3db.fts/Lucivy_Source`, 147 documents, de gros textes).

| `Lucivy_Source` | avant | après |
|---|---:|---:|
| textes du dictionnaire (`dictionary_texts`) | 513 210 | 513 210 |
| octets du dictionnaire (`dictionary_bytes`) | 70 192 162 | **72 825 395** (+3,75 %) |
| octets de l'index (`index_bytes`) | 96 720 657 | 98 636 021 |
| documents vivants, orphelins purgés | 147, 0 | 147, 0 |

Il n'y avait aucun texte à retirer, et le dictionnaire grossit de 2,6 Mo, ce qui fait toute la hausse de
l'index. Sur `Symbol`, l'autre table du même relevé qui garde tous ses textes, il baisse un peu (23 088 644 →
23 060 266). La hausse n'est donc pas systématique.

## Ce qu'on suppose (à vérifier, rien n'est prouvé)

- **La table des ids des textes** (`src/suffix_fst/termtexts_v3.rs`, `ids_section`) coûte 8 octets par plage d'ids
  contigus gardés. Si la compaction renumérote ou garde les ids en plages plus morcelées qu'avant, la table grossit
  même quand aucun texte ne part.
- **Ou bien le dictionnaire est réécrit à neuf**, alors qu'il était fait de générations repliées : la forme
  diffère (FST, `.pidx`) sans que le contenu change.

## Ce qu'il faut faire

1. Mesurer d'abord, sur une **copie**, jamais sur l'original : `cp -a --reflink=auto` du dossier
   `Lucivy_Source`, l'hôte à l'arrêt ou le backend fermé (`POST /backend/close`). Ensuite `index_report` avant et
   après `compact_dictionary` (`LUCIVY_REPORT_INDEX`, `test_index_report`), en détaillant par fichier
   (`.sfx`, `.termtexts`, section des ids, `.pidx`, `.gmap`).
2. Corriger selon la cause :
   - si rien n'est mort (`DeadTextsReport` à zéro texte retiré), `compact_dictionary` ne réécrit rien et le dit
     dans son rapport ;
   - si des textes sont morts, la réécriture ne doit pas morceler les plages d'ids.
3. Ne pas toucher : le format des fichiers (ce serait une 4.4), l'API de `maintain` et `index_report`, la garde
   `max_purge_ratio`.

## Comment le prouver

- **Rouge d'abord**, dans `lucivy_core/tests/test_maintain.rs` : un index sans aucun supprimé (ou mieux, la forme
  qui reproduit le cas, retrouvée par la mesure 1), puis `maintain` avec compaction. Exiger
  `dictionary_bytes après <= dictionary_bytes avant`. Le voir rouge sur main avant le correctif.
- Puis `cargo test --release -p lucivy-core` et `cargo test --lib` (ld-lucivy) en entier, verts.
- Puis la mesure 1 rejouée sur la copie : l'après n'est pas plus gros que l'avant.

## La mesure (16 novembre 2026, à luciepc, lucivy main 4.3.9)

Sur une copie de `Lucivy_Source`, prise après la maintenance de la nuit, sans ouvrir l'original. Les segments
nommés par chaque `meta.json` sont complets.

1. **La maintenance est idempotente.** Deux `maintain` avec compaction à la suite (`test_maintain`,
   `maintain_an_index`) : 98 636 021 → 98 636 021 octets, 513 210 → 513 210 textes, 4 segments, zéro tour de fusion.
2. **La hausse se reproduit sur un index neuf.** Les 147 documents réécrits par `test_light_profile`
   (`LUCIVY_PROFILE_VARIANTS=positions`) :

| les 147 documents | segments | générations | `.sfx` | `.termtexts` | `.pidx` | dictionnaire |
|---|---:|---:|---:|---:|---:|---:|
| un commit par document, comme les écrit la mémoire | 14 | 22 | 62,38 Mo | 7,87 Mo | 0,47 Mo | 70,7 Mo |
| le même, après `maintain` avec compaction | 4 | 4 | 64,39 Mo | 7,85 Mo | 0,56 Mo | 72,8 Mo |
| un seul commit | 4 | 4 | 64,23 Mo | 7,79 Mo | 0,56 Mo | 72,6 Mo |

**Lecture.** La compaction ramène le dictionnaire à une génération par shard, la forme d'un index écrit d'un coup.
Elle ne laisse rien de plus : `.termtexts` est égal, et `.sfx` est à 0,25 % de l'index neuf. Ce sont les petites
générations d'une écriture document par document qui tiennent les mêmes suffixes en 3 % de moins dans leurs `.sfx`.
Ne rien réécrire quand aucun texte n'est mort garderait ces 22 générations, que chaque requête parcourt une à une.
On ne le fait donc pas.

**Ce qui reste, en curiosité, pas en défaut** : pourquoi plusieurs petites FST de suffixes pèsent moins qu'une seule
qui tient les mêmes clés. C'est à regarder seulement si le poids du dictionnaire redevient un sujet.

Scripts et journaux à luciepc, `~/.cache/rag3weaver-build/mesure-dico-source/` (`idem.sh`, `neuf.sh`, `parts.sh`).
