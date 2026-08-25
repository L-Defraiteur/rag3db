# lucivy — nuit du 24 au 25 août : parité navigateur, fusion en arènes, compaction

Session lucivy, branche `wip/publication-3.0.0`. Suite du doc 44 (numéroté 41 chez lucivy — renuméroté ici, collision avec la passation 41). Tout est
mesuré ; le journal détaillé est dans `lucivy/docs/25-08-2026/01-journal-nuit.md`.

## 1. Parité navigateur / natif : acquise

Panel de 21 requêtes (contains strict/relaxed, split, startsWith, term,
phrase, fuzzy d1/d2, regex, parse simple/booléen, filtre extension, champ
path, sans résultat) sur les **15 440 fichiers** kernel indexés dans le
navigateur : **20/21 identiques** au natif (comptes, top-10 ordonné, scores
à 1e-4, nombre de spans), la 21e est un ex æquo (382 docs au même score sur
le champ `path`, fenêtre top-10 arbitraire). Harnais : `playground/parity_*`
+ `lucivy_core/tests/test_playground_parity.rs`.

Ce qu'il a fallu pour y arriver, en plus du doc 41 : les handles paresseux
**épinglent** les octets d'un fichier supprimé tant qu'ils vivent
(sémantique unlink — un searcher tenait des segments qu'une fusion venait de
remplacer ; mmap masque ça en natif), et le panel tourne depuis le worker
(`lucivy_open` + `lucivy_search`) sur l'index OPFS.

## 2. Fusion v3 en arènes (natif, vous en profitez directement)

`merge_segments_v3` n'alloue plus par entrée : arène de textes, table
d'intern en adressage ouvert comparée contre l'arène, postings plats
bucketés par ordinal (les buckets arrivent triés par construction),
lecteurs `for_each_entry` sans allocation, remap de docs en table dense.
Sortie **octet pour octet identique** (tests de merge, pipeline, vérités
terrain). Natif i686, fusion `content` de 14 segments / 650 k tokens :
**643 → 406 ms** (boucle 436 → 145, assemblage 172 → 226 à cause des 650 k
`String` que le type de sortie impose encore).

## 3. Compaction (nouvelle API)

`ShardedHandle::compact(max_docs)` → `IndexWriter::compact` → planifié
**dans l'acteur** `segment_updater` (il seul sait quels segments une fusion
tient ; un plan fait dehors se faisait refuser par une cascade de la
policy). Deux robustesses au passage, qui vous concernent : la préparation
d'un lot de fusions est atomique (une 2e op refusée laissait la 1re « en
fusion » pour toujours), et un refus **répond** au demandeur au lieu de le
laisser paniquer (« actor died without replying »).

Mesure, 15 440 fichiers, compaction à 10 k docs/segment : 4 fusions,
16,2 s, **294 → 21 `.sfx`, 5 642 → 4 449 Mo (−21 %)**. Le FST partage les
suffixes (`sfx` 2 215 → 1 613, `bytemap` 568 → 425, `termtexts` 306 → 230) ;
les postings ne bougent pas (`word_sfxpost` 750, `sfxpost` 559). Après un
chargement en masse, appelez-la : moins de segments = moins de FST à ouvrir
par requête.

## 4. Le mur du navigateur, chiffré

Même compacté, l'index v3 fait **11× le texte** (4,4 Go pour 400 Mo).
Requêtes dans le navigateur : 4-14 s **en release comme en debug** — ce
n'est pas le CPU, c'est la relecture de sidecars depuis OPFS à chaque
requête (le cache de fichiers de 768 Mo ne peut pas tenir 2,5 Go de FST).
Deux chantiers seulement changent ça : le format (les dérivés
`bytemap`/`word_sfxpost` sont recalculables ; postings compressibles) et des
lecteurs **par plage** pour les 2,5 Go de fichiers indexés par ordinal, le
`.sfx` restant résident. Pour vous côté natif : mmap + page cache absorbent
tout ça, rien à faire ; mais le ratio 11-15× est aussi ce que vos deltas et
vos blobs transportent.

## 5. Outillage disponible

`?corpus=<tar.gz>` (indexation directe d'une archive servie),
`?open=<index>` (réouverture OPFS), `?compact=N`, `?cache=<MB>`, `?noopfs`,
`?verbose` ; `LUCIVY_MERGE_CONCURRENCY`, `LUCIVY_FILE_CACHE_BYTES`,
`LUCIVY_WRITER_HEAP`/`_THREADS` ; `LUCIVY_WASM_DEBUG=1` pour un build
symbolisé ; hook de panic + allocateur qui journalisent la pile dans le ring.

## Post-scriptum (≈01:05)

`compact` attend maintenant un index calme avant de planifier et refait
des tours tant que le nombre de segments baisse (une cascade de la policy
peut tenir des segments au moment du plan — en wasm, avec une fusion à la
fois, elle les tenait *tous*). Natif, 15 440 docs : **285 → 15 `.sfx`,
5 580 → 4 339 Mo, 43 s** (`8b58881`). Rejeu navigateur en cours.
