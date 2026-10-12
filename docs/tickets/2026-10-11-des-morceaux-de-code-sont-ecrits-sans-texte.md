# Des morceaux de code sont écrits sans texte : 20 % de code-rag3db

- **État** : ouvert (11 octobre 2026) — l'arbre principal le prend (cause, témoin, correctif, réécriture des morceaux touchés)
- **Gravité** : réponse fausse (une recherche ou un usage ne voit pas le passage, sans le dire jusqu'aux défenses de `40f24a50b`) ; et blocage (la passe de vecteurs bouclait dessus, le gel de code-lourd)
- **Atteignable en service** : oui, tout index de code
- **Touche rag3weaver** : oui (l'écriture des morceaux, à trouver)

## Ce que c'est

Sur une copie de l'index `code-rag3db-depot` (le dépôt rag3db, prise à 20 h 24, lib
`e71c85b551cf`), comptée par F (`e2e_mesure_index_entier::les_morceaux_a_texte_vide`) :
**26 092 `Scope_Chunk` sur 128 732 ont `_text` nul ou vide, soit 20 %**. Ce n'est pas quelques
lignes abîmées : il y en a des centaines par gros fichier (`lucivy_core/src/sharded_handle.rs` 301,
`tree-sitter-scss/src/parser.c` 211, `luciole/src/runtime.rs` 185, `rag3weaver/src/catalog.rs` 141,
`dialect.rs` 94…).

**Le texte a existé** : les 26 092 portent tous un `_text_hash` qui est celui d'un vrai texte,
aucun celui d'une chaîne vide (`e2e_mesure_index_entier::les_morceaux_vides_ont_le_hash_d_un_texte`).
Le texte est donc perdu entre le hachage et l'écriture en base.

Ni codeparsers ni le découpage ne créent un morceau vide (relu par codeparsers : un champ vide est
sauté, le découpeur ne rend aucun morceau blanc ; record_nodes.rs:1450-1496, chunker.rs:110-121).

## Ce qu'il a coûté

- **Le gel de code-lourd** (luciepc, quatre fois, autour de 18-20 %) : l'embarquement sautait un
  morceau sans texte sans le marquer, la réclamation le reprenait à chaque passe ; quand les
  morceaux dus qui restaient étaient tous vides, `run_index` bouclait sans fin (WAL qui grossit,
  aucun appel au service). Défenses dans `40f24a50b` : un morceau sans texte n'est jamais dû, une
  passe sans progrès s'arrête nommée.
- **Des faux négatifs** : ces passages ne sont ni cherchables par leur texte ni embarqués. Depuis
  `40f24a50b`, l'index le dit (journal de fin, `index_states.chunks_without_text`, en-tête des
  lectures).

## Pistes, à prouver

- Le chargement en masse (CSV, `cellule_csv`, record_nodes.rs:695-729) : un `\` non mis entre
  guillemets sous `escaped_newlines=true` (relevé par codeparsers). Le compte par gros fichier va
  dans ce sens ; la comparaison des `\` faite sur les fichiers ne vaut rien (les décalages des
  morceaux portent sur le texte du scope, pas du fichier).
- Témoin en cours (codeparsers) : `sharded_handle.rs` seul, par le chemin de masse et par
  `ingest_code`, sur deux bases neuves ; le compte des vides de chaque côté.

## Ce qu'il faut pour le fermer

La cause prouvée par un témoin rouge (un fichier passé en masse rend des morceaux sans texte), le
correctif, et **la réécriture des morceaux touchés** : la montre ne les reverra pas (le fichier n'a
pas changé) ; l'index doit lancer de lui-même la réécriture des morceaux sans texte quand leur
compte est non nul. La question de fond, gardée pour Lucie : un morceau sans texte doit-il exister
(un scope sans embarquement mais avec ses relations) ou ne jamais naître.

## Ce qui ne reproduit pas (sondes de codeparsers, 11 octobre au soir)

Sonde `sonde_morceaux_vides.rs` (worktree `rag3db-vide`, non commitée ; bases gardées sous
`~/.cache/rag3weaver-build/sonde-vides-*`), lib du même bâti que l'hôte de luciepc
(`rag3db-build-6df47a543…`). Colonnes comptées : `Scope_Chunk`, `_text` nul, `_text` vide, vides
avec hash. **Aucun vide, nulle part** :

- `sharded_handle.rs` seul, en masse (`sync_source`) puis paquet par paquet (`ingest_code`) : 351 morceaux ;
- base mémoire : `lucivy_core` (1 757), `ld-lucivy` (28 703), le dépôt entier (130 228) ;
- base disque, fermeture et réouverture : `ld-lucivy` sans vecteurs puis avec vecteurs (embarqués
  pendant l'ingestion), le dépôt entier sans vecteurs ;
- une masse arrêtée à 320 fichiers sur 1 147 puis reprise, avec réouvertures ;
- `edit_file` sur `sharded_handle.rs` (ligne remplacée à longueur égale, ligne insérée en tête), puis
  resynchronisation de la source ;
- la passe de rattrapage (`embarquer_le_retard` en boucle, 57 passes, puis réouverture) et la
  réclamation seule répétée mille fois sur les mêmes lignes.

Une passe propre ne perd donc rien : ni la masse, ni le SET de réclamation, ni celui du vecteur, ni
le point de reprise. Restent hors des sondes : **l'hôte lui-même** (sessions concurrentes, montre
et réactions, rebâti du plein texte à l'ouverture après une génération interrompue), **un vrai
service d'embarquement lent**, **les arrêts brutaux** et le rejeu du journal, et **la copie prise au
vol** (celle que F a comptée a été prise pendant que l'hôte tenait la base). La sonde par l'hôte,
avec un faux service lent et une session qui interroge pendant l'index, est la suivante (codeparsers,
12 octobre au matin).

La réclamation seule reproduit en revanche **la boucle** : sans le service, elle rend sans fin ses
propres lignes (512 000 en 8,7 s) — ce que les défenses de l'arbre principal arrêtent.

## Le chemin réel, par l'hôte, est sain (12 octobre, nuit)

Codeparsers, avec le témoin de bout en bout de l'orchestration : binaire release de master
`8610eaec6` contre la lib du poste (`6df47a54…`), un hôte à part, `code-tree add` d'un worktree de
rag3db sur une base neuve, l'outil `index` par une session MCP avec le vrai service `granite-278m`
(régime doux, 1 369,5 s). Pendant l'index, la session a lu (`search_code`, `usages`) et renommé un
symbole sur disque, et la montre a réindexé. Compté ensuite, base fermée : **`Scope_Chunk` 102 469,
`_text` nul 0, vide 0 ; `File_Chunk` 6 303, vides 0.**

Le chemin d'aujourd'hui ne crée donc pas de morceau vide en une histoire propre. Les vides de la
copie de F et de code-lourd viennent de **leur histoire** : des binaires plus anciens (code-lourd a
été indexé par `a3c1ca98f`, puis par le binaire suivant), des `kill -9`, et une copie prise au vol.
La copie de code-lourd que Lucie prend au réveil, base fermée, dira si ses vides ont un parent et un
hash ; l'outil de comptage est la sonde de codeparsers (`compter_une_base`), versée au ticket.

Ce qui reste à faire pour fermer : établir l'origine sur cette copie ; si c'est un défaut d'un
binaire ancien déjà corrigé, le dire ici ; dans tous les cas, **la réécriture des morceaux sans
texte** d'une base qui en a (l'index la lance quand le compte est non nul), puisque la montre ne les
reverra pas.
