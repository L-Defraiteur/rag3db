# Une réécriture perd les mentions d'un nom neuf, et garde la vieille ligne des autres

- **État** : ouvert (21 novembre 2026)
- **Famille** : produit
- **Gravité** : moyenne. Après une réécriture, `usages` d'un nom apparu dans ce lot manque des sites ; dans les mêmes
  scopes, des sites d'un nom déjà connu gardent leur ancienne ligne. Rien ne le signale.
- **Atteignable en service** : oui, à chaque synchronisation qui apporte un nom que la base n'a jamais vu, utilisé
  dans un scope déjà indexé.
- **Touche rag3weaver** : oui (ingestion des `MENTIONS`, drain des liens)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, à la mesure du coût des occurrences (lot 2 de l'intra-scope)

## Constat

On réécrit 419 fichiers par la sonde d'écriture (`history_layers::tests::sonde_ecriture_en_couche`, mode ordinaire) :
de l'arbre indexé de la copie de G (`repetition-versions-code-rag3db-depot-20261119-194739`) vers master 726676b9ea.
La sonde est sur la branche jamais déposée `codeparsers/occurrences-cost-probe` (3bc114d837), lib 4753938c9d.

Les mêmes fichiers sont réécrits deux fois, sur deux reflinks neufs, une fois sans et une fois avec le prototype des
occurrences (`RAG3WEAVER_PROTO_OCCURRENCES`). Le prototype ajoute des arêtes `OCCURS` Scope → Symbol et n'écrit
aucune `MENTIONS`. Résultat : **733 332 `MENTIONS` sans, 733 344 avec**, reproduit deux fois (le 21 novembre à 6 h 52
et à 18 h). L'écart compte 4 arêtes retirées et 16 ajoutées. Toutes touchent des noms qui apparaissent dans ce lot :
`Tokenizer`, `from_file` et `BatchLongest`, du crate `tokenizers`, arrivés avec le comptage des jetons.

| fichier | scope | sans le prototype | avec |
|---|---|---|---|
| `src/burn_bge_m3_embedder.rs` | `from_bytes`, `BurnBgeM3Embedder` | `map_err` ligne 146 ; pas de `from_file` ni de `BatchLongest` | `from_file` 133, `map_err` 134, `BatchLongest` 136 |
| `src/burn_granite_embedder.rs` | `from_bytes`, `GraniteEmbedder` | `map_err` ligne 109 ; pas de `from_file` ni de `BatchLongest` | `from_file` 99, `map_err` 100, `BatchLongest` 102 |
| `src/tokens.rs` | `counter` | ni `Tokenizer` ni `from_file` | `Tokenizer` 77, `from_file` 77 |
| deux scripts Python sous `docs/` | `tok` | pas de `from_file` | `from_file` |

La passe « avec » a raison : ses lignes sont celles de master. La passe sans le prototype, c'est-à-dire le chemin
normal, garde donc des `MENTIONS` périmées : les mentions des noms neufs manquent, et l'arête vers `map_err` reste à
la ligne de l'ancienne version du fichier.

## Hypothèse, à prouver par le témoin

C'est l'ordre du drain. Une `MENTIONS` vers un `Symbol` que ce lot crée (un nom neuf) partirait avant que la ligne du
symbole soit posée. Le chemin COPY l'écarte alors comme un lien sans bout (`absents`, `copier_les_liens`), sans
erreur. Ce qui touche aussi `map_err` reste à lire : l'arête de la paire existante passe par la mise à jour
(`a_merger`) et devrait prendre la nouvelle ligne.

Le prototype met en file des liens vers `Symbol` (`mettre_en_file_les_liens`, puis `devoir(Symbol)`). Il change
ainsi l'ordre ou la complétude du drain. L'écart n'est donc pas du bruit du prototype : le prototype masque le défaut.

## Témoin attendu

Une base indexée, puis la réécriture d'un fichier dont un scope existant utilise un nom que la base n'a jamais vu.
Après le drain :
- la `MENTIONS` vers ce nom est là ;
- les `MENTIONS` du scope vers des noms connus portent leurs nouvelles lignes ;
- le compte de liens sans bout de la tranche est nul.

Il faut le voir rouge sur master avant le correctif.
