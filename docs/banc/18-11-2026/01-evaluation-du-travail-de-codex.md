# Le travail de Codex dans le périmètre du banc : évaluation (18 novembre 2026)

Demande de c7 à la reprise, au mot de Lucie : « chaque session évalue la pertinence de ce qu'a fait Codex ». Codex a
tenu le rôle du banc du 16 novembre, 11 h, au 17 novembre, 21 h 08. Cette page juge ses commits de master dans
le périmètre du banc et ses branches `cx-banc-…`, contre nos règles : code neuf en anglais, rouge d'abord, artefact
prouvé avant tout chiffre, défaut connu porté par un ticket sur master.

Relevé fait le 18 novembre, en lecture seule : `git log 927941a67a..origin/master -- docs/banc`, les branches
`origin/cx-banc-*`, sa passation (`origin/cx-banc-lucivy-indexed-ro-20261116:docs/banc/17-11-2026/99-passation-pour-claude.md`)
et le jeu figé à luciepc.

## 1. Le tableau

| où | quoi | verdict | pourquoi |
|---|---|---|---|
| master, 6 commits de `docs/banc` (38146af549 → 99fef92ce6) et `docs/banc/16-11-2026/18-ro-classificateur-raw-vertex.md` | relectures du travail d'ED sur le jeton Vertex : témoin, union, candidat | **utile, à garder** | Le correctif lui-même est sur master par ED (936f13daa2 le témoin, 4b32256693 le correctif : `connect_llm` garde la `TokenSource`). Ces relectures ne prouvent rien seules, elles bornent ce qui a été joué. |
| master, mon ticket `2026-11-16-un-modele-vertex-declare-garde-son-jeton-d-une-heure.md` | état mis à jour : « correctif proposé qualifié CPU et faux HTTP seulement ; union et intégration NON QUALIFIÉES » | **utile, à garder** | C'est exact, et c'est bien dit : le ticket reste ouvert. Il se fermera sur une passe réelle de plus d'une heure sans 401 : F300 rejouée (entrée 1). |
| `cx-banc-embedding-rate`, 675cf02000 | `tests/bench_embedding_rate.rs` et son entrée `[[test]]` (feature `daemon`) : faux service à 1 000 caractères/s par `EmbedDaemon` et `CallbackEmbedder`, client `DaemonEmbedder` de l'hôte, lots par `stable_batches`, plafond de 200 caractères avec des textes non ASCII, mesure laissée vide pour le rouge | **utile, à garder** | C'est le pas 2 de l'étalonnage, exactement dans la forme de la page du 16 (§ 8) : anglais, témoin non ignoré, mesure posée à vide. **Son rouge n'a pas été observé**, aucune trace ne le montre : le pas 2 est commencé, pas fait. Je reprends depuis cette tête. |
| `cx-banc-oracle-migration`, 15 commits (df02f39a06 → 9bcac1558c) | pages 04 à 07 du 16 : la recette de l'oracle des copies finales A, B et B′ (bm25, vector, tous signaux, tolérances, sous-modules et non suivis, preuve du lecteur) ; une relecture sur copie des preuves de fin de la bascule ; un ticket `2026-11-16-oracle-des-copies-finales-des-trois-bases.md` | **utile mais à requalifier** | La recette reprend nos leçons du 16 (fichiers non suivis, tolérance, signaux exigés) et refuse les anciens lanceurs. **Rien n'est joué** : aucune parité sur les copies finales. Le ticket n'est pas sur master. Ses chiffres de fin (97 247 vecteurs, dette 0) sont une lecture de copie, à reprouver au rejeu. |
| `cx-banc-corpus-public-20261116`, 12 commits | corpus publics multilangages figés (miroir, licences, tirage déterministe, attentes de 12 sources) et quatre tickets de défauts d'analyse (upb généré, propriétés TypeScript, contrainte générique Rust, template C++ qui contamine un namespace) | **utile mais à requalifier** | Ce n'est pas une entrée de ma liste. Les défauts sont précis, mais **aucun des quatre tickets n'est sur master** : un défaut connu sans ticket posé n'existe pas pour les autres. À faire relire par codeparsers, puis à poser en docs seules. |
| `cx-banc-lucivy-indexed-ro-20261116`, 66 commits | relectures en lecture seule pour lucivy (admission du corpus FS, `indexed: false`), Archi, la publication commune, Vertex ; un audit du cache de contexte Vertex ; sa passation | **utile mais à requalifier** | Aucun test n'est joué par le banc, et il le dit : ce sont des relectures de preuves d'autres porteurs. Quatre tickets sur sept ne sont pas sur master (FS de lucivy, témoin documentaire Vertex d'Archi, ledger de sonde, cache Vertex). La valeur dépend de leurs porteurs. **La lisibilité est faible** (§ 4). |
| `cx-banc-contexte-initialisation`, `-placement-explicite`, `-publication-12nov`, `-residence-appels`, `-residence-nocturne`, `-resultats-k10`, `-suivi-k10` | branches des 12 et 13 novembre, d'avant cette période | **historique, à ne pas fusionner** | La passation du 13 novembre le disait déjà : ne pas fusionner `cx-banc-residence-appels` entière. Rien à retirer, rien à reprendre sans un nouveau besoin. |

**Rien d'inutile ni de nuisible** n'est trouvé :
- aucun commit sur master hors docs dans le périmètre du banc ;
- aucune preuve repeinte en vert ;
- le jeu figé de l'étalonnage est intact : `~/.cache/rag3db-banc-notes/etalonnage/set.json` à luciepc, sha256
  `458f83f0e8877cd3c585eb6b3a54e0ef4b42c0e15cd4e1a72439854326acaa3d`, inchangé depuis le 16 à 10 h 11 ;
- les copies A, B et B′ (`banc-parite/migration-*-20261116`) sont en place.

## 2. Le verdict d'ensemble

1. Codex a respecté les règles qui comptent : identité gmail, aucune attribution, aucun force push, aucune preuve
   repeinte, rien d'annoncé vert sans l'avoir joué.
2. Dans ma liste, il a commencé le seul pas utile au bon endroit : le témoin de l'étalonnage, bien formé, mais
   sans l'avoir vu rouge.
3. L'essentiel de son temps est allé à des relectures pour d'autres porteurs et à un corpus public non demandé :
   utiles si leurs porteurs les reprennent, sans effet tant que leurs tickets restent sur des branches.
4. Huit tickets dorment sur des branches non posées. C'est la seule entorse réelle : un défaut connu se porte sur
   master.
5. Ses écrits sont précis mais denses au point d'être difficiles à reprendre (§ 4). Le travail est récupérable,
   pas lisible tel quel.

## 3. Ce que ça change à la page du 16 (`02-passation-pour-codex.md`)

- **Entrée 6, l'étalonnage** : le pas 1 est fait par le banc (jeu intact). Le pas 2 est commencé par Codex, sur
  `cx-banc-embedding-rate`, tête 675cf02000, rouge non observé. **Le banc reprend depuis cette tête** : voir le
  rouge, puis écrire la mesure, puis le vert, puis la mesure ignorée.
- **Entrée 1, F300 et N rejouée** : intacte. Le correctif du jeton Vertex sur master permet de rejouer F300 au-delà
  d'une heure, ce qui qualifiera aussi le ticket Vertex.
- **Entrées 2, 3, 4, 7 et 8** : intactes, aucun commit.
- **Le rejeu de la parité sur la copie finale** : la recette de Codex (`cx-banc-oracle-migration`, page 04) est une
  bonne base. Elle se joue au mot de c7, avec le code de master d'aujourd'hui (lib 59ac9c86bc, sans
  `RAG3WEAVER_MOTEUR_ANCIEN`).
- **Les tickets à poser** : les huit tickets des branches. Je pose ceux du banc (`oracle-des-copies-finales…`,
  `upb-gen…`, `cache-de-contexte…`) après relecture. Je signale les autres à leurs porteurs : codeparsers,
  lucivy, Archi, CP.

## 4. La lisibilité de sa passation

- **Ce qui aide** : les chemins, les hashes et les branches sont exacts, et vérifiables un par un. Les frontières
  sont nettes (ce qui n'est pas qualifié est dit tel), et chaque « non joué » est écrit.
- **Ce qui gêne** :
  - des phrases télégraphiques collées (« CPU v4 réellement2+3PASS », « 11UNKNOWN non attribués ») ;
  - des sigles sans définition (RO, FIN, PREPA SOURCE, D6, CP, C9) ;
  - des hashes en série sans dire à quoi ils servent ;
  - aucun « ce que ça veut dire pour Lucie ».
- Une session neuve met longtemps à en tirer l'état. La page du 16 du banc, par entrées avec « à faire, preuve,
  taille », se reprenait mieux. Pour la prochaine passation : une ligne d'état par entrée, d'abord ; les preuves
  ensuite.
