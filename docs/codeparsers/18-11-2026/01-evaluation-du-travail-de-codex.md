# codeparsers — évaluation du travail de Codex (16 au 17 novembre 2026)

Demande de Lucie, portée par c7 le 18 novembre à 11 h 3x : chaque session évalue la pertinence de ce que Codex a
fait dans son périmètre pendant l'arrêt (du 16 à 11 h au 17 à 21 h 08). Périmètre de codeparsers : le sous-module
`extension/rag3weaver/codeparsers` (dépôt `L-Defraiteur/codeparsers`), `docs/codeparsers/`, et la résolution des
arêtes de `extension/rag3weaver/src/code.rs`. Base de la relecture : master `99fef92ce6`, le dépôt codeparsers
relu (`git fetch`), la passation de Codex
(`git show "origin/cx-codeparsers-publication-snapshot-cause-fix:docs/codeparsers/17-11-2026/99-passation-pour-claude.md"`).

## Ce qui a changé sur master et dans le dépôt codeparsers

- **Master rag3db : rien dans le périmètre.** Des 76 commits de `927941a67a..99fef92ce6`, aucun ne touche le
  sous-module, `docs/codeparsers/` ni `code.rs`. Le pointeur reste `bd13385`, `REGLES_DE_L_ANALYSE` vaut 7.
- **Dépôt codeparsers : master n'a pas bougé** (`bd13385`). Deux branches neuves : `cx-rust-stop-words` et
  `cx-language-corpus-probe`.
- **Dix-neuf branches rag3db `cx-codeparsers-*` du 16 et du 17**, en une pile (chaque branche reprend la
  précédente) et une branche à part (`language-corpus`). Les branches `cx-codeparsers-*` du 11 au 13 novembre
  (require, bilan, passation du week-end) sont d'une session Codex d'avant l'arrêt : hors de cette évaluation.

## 1. Le tableau

| branche ou commit | ce que c'est | verdict |
|---|---|---|
| rag3db `cx-codeparsers-rust-stop-words`, `-2`, `-3` | mon lot (5) des mots vides de Rust repris tel quel (`e4d66995f`, `b1d978aef`, `2b9c382c4`) ; `-3` pointe sur `cx-rust-stop-words` | **utile, à garder** : c'est la base de la reprise |
| codeparsers `cx-rust-stop-words` (`ecc8194`, `34c74e4`) | les données neuves des mots vides renommées en anglais (règle de Lucie du 16), la recette du générateur précisée | **utile, à garder** |
| rag3db `cx-codeparsers-rust-red-1` : `3388464f6` | deux témoins e2e en anglais : un `std::env::var(…)` et un `var(…)` du projet sur **une même ligne**, dans les deux ordres, avec une indentation UTF-8 | **utile, à garder** : ils montrent un vrai défaut de ma règle (elle jugeait toutes les occurrences `::var` de la ligne et masquait l'appel du projet) |
| rag3db `cx-codeparsers-rust-red-1` : `c5fc9c4b6` | la règle du chemin lit la racine **à la position exacte de la mention** (`reference_path_root`, colonne en octets sur la ligne d'origine), avec son test unitaire ; remplace ma recherche sur toute la ligne | **utile, à garder** : meilleure que ma forme ; rouge consigné (`9e7caecc9`), mais **aucune passe complète** ensuite |
| rag3db `cx-codeparsers-rust-probe-baseline`, `-rust-fix-1`, `-protocol-report` | la sonde `sonde_marque_nom.rs` étendue de 216 lignes (profil `RUST_PROBE_PROFILE`, deux phases), un pilote Python, `compare.py`, un relevé historique de 1 530 lignes (`historical-var-pairs.json`), des protocoles | **utile mais à requalifier** : l'idée (rejuger les paires `var` historiques) sert la mesure du lot ; les pilotes, contrôleurs et protocoles ne sont pas du code de produit et ne vont pas sur master. Reprendre au plus la comparaison, dans la sonde existante |
| rag3db `cx-codeparsers-rust-relay`, `-rust-relay-report`, `-publication-contract`, `-publication-core`, `-publication-window-red`, `-publication-capacity`, `-publication-guard-red`, `-publication-guards-grouped`, `-publication-guards-report`, `-publication-guards-fixture-pin`, `-publication-guards-fix`, `-publication-guards-correction`, `-publication-cause-report`, `-publication-snapshot-cause-fix` | un « relais de publication » Python (`scripts/publication_relay.py`, 1 568 lignes ; `publication_loader.py`) pour faire passer un « ledger de sonde » d'un poste à l'autre avec accusés de réception, coupons « GO », gardes d'horloge et de nettoyage ; un ticket sur branche (« la publication du ledger de sonde attend un ack sans relais autonome ») | **inutile, à ne pas fusionner** : absent de ma liste du 16 et de tout ticket d'avant ; une infrastructure inventée pour faire tourner une sonde à distance, qui n'est pas un besoin de codeparsers ; jamais verte (63 cas « NONJOUÉS » selon sa propre passation) ; elle a consommé la journée du 17. Les branches restent comme archive (aucune suppression de branche distante) |
| rag3db `cx-codeparsers-language-corpus` | un banc multi-langages sur corpus publics (32 382 fichiers), ses scripts sous `docs/codeparsers/16-11-2026/language-corpus/`, **sept tickets neufs** (Svelte qui n'initialise pas sa grammaire, champs des méthodes C# qui confondent noms et types, un worker qui déborde sa pile sur un fichier OpenSSL, le compteur de scopes du fichier à zéro, cinq fichiers à la borne du banc, l'oracle aux façades, les relations sur ces projets) ; son pointeur vise `1b35cfb` d'une branche d'outil | **utile mais à requalifier** : les tickets nomment de vrais défauts du parseur, à relire un par un et à porter sur master avec leur preuve ; le banc lui-même n'était pas dans ma liste ; **le pointeur ne doit pas atterrir** (il vise un outil, pas un correctif) |
| codeparsers `cx-language-corpus-probe` (`be2ebe3`…`877cb4a`) | `examples/language_corpus_probe.rs` (521 lignes), l'outil du banc | **utile mais à requalifier** : un outil de mesure ; à garder sur sa branche tant qu'un ticket ne le demande pas sur master |

## 2. Verdict d'ensemble

- Le 16 au matin, Codex a repris mon lot (5) au bon endroit et l'a amélioré : la règle du chemin à la position exacte et ses deux témoins sont meilleurs que ma forme. C'est à garder.
- Il a ensuite dérivé : une sonde en deux phases, puis, le 17, un relais de publication de 1 568 lignes que personne n'a demandé, jamais vert, hors du produit.
- Le banc multi-langages a trouvé de vrais défauts du parseur (Svelte, C#, OpenSSL), qui méritent leurs tickets sur master.
- Rien n'a été cassé : master et le dépôt codeparsers n'ont reçu aucun commit de mon périmètre, et les règles sont tenues (gmail, aucune attribution, aucun force push).
- Coût : la journée du 17 est perdue pour le produit ; le lot (5) n'est toujours pas passé ni poussé.

## 3. Ce que ça change à ma page du 16

- **L'entrée 1 repart de `cx-codeparsers-rust-red-1`** (mon lot, les données en anglais de `cx-rust-stop-words`, la
  règle par position de Codex et ses deux témoins), reposée sur master. Restent la sonde avant/après, la doc,
  `REGLES_DE_L_ANALYSE = 8` et la passe complète, comme écrit.
- **Une entrée neuve, après l'entrée 2** : relire les sept tickets de `cx-codeparsers-language-corpus`, porter sur
  master ceux qui tiennent, avec leur preuve (Svelte, C#, OpenSSL d'abord : ce sont des défauts du parseur).
- **La pile du relais de publication** ne reprend pas. Elle reste en branches, comme archive.

## 4. Lisibilité de sa passation

Exacte sur l'état (ce qui n'est pas qualifié est dit), mais **difficile à lire** : un jargon propre (« GO », « CP »,
« coupons », « ledger », « RO TOTALE »), des empreintes SHA en série, des phrases sans espace après les nombres, et
aucune réponse simple à « qu'est-ce que c'est, à quoi ça sert ». Une session neuve n'y trouve pas où reprendre le
produit : le lot (5) n'y est cité qu'en passant.
