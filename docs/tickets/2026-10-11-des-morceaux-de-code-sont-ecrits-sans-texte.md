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
