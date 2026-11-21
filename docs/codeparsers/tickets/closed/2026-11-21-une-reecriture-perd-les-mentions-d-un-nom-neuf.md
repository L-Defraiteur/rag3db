# Une réécriture perd les mentions d'un nom neuf, et garde la vieille ligne des autres

- **État** : corrigé (21 novembre 2026), branche `codeparsers/new-name-mentions-r`. Deux causes, deux correctifs ; voir « Ce que l'enquête a établi » en fin de ticket
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

## Ce que l'enquête a établi (21 novembre 2026, au soir)

L'hypothèse de ce ticket (l'ordre du drain écrit une `MENTIONS` avant son `Symbol`) est **contredite** par la lecture
du code : les symboles d'une ingestion sont écrits tout de suite, par un graphe à eux, avant la mise en file des liens.
L'enquête a trouvé deux défauts distincts.

1. **L'ordre des groupes de liens n'était pas déterministe.** C'est la cause des arêtes de ce ticket (`from_file`,
   `BatchLongest`, `Tokenizer`, la vieille ligne de `map_err`).
   - **Le mécanisme** : le drain groupe ses liens par relation et par jeu de propriétés, et parcourait les groupes dans
     l'ordre d'une `HashMap`, dont la graine change à chaque table. Une paire venue dans deux groupes prenait la valeur
     du groupe écrit en dernier.
   - **La preuve** : deux réécritures identiques ont donné 733 332 puis 733 344 `MENTIONS`, sans le prototype comme
     avec. Le prototype des occurrences n'y était pour rien.
   - **Le correctif** : un `BTreeMap`, par relation puis par propriétés. Le témoin est
     `e2e_drain_order::two_identical_drains_write_the_same_links`, rouge sur master (deux valeurs en seize drains).
2. **Les rendez-vous d'une fermeture repliée partaient d'un scope jamais écrit.**
   - **Le mécanisme** : `fold_lambdas` replie une fermeture ou une compréhension dans le scope nommé qui la porte, mais
     laissait ses rendez-vous (`pending` et ses neuf listes) sous la clé de la fermeture. Le drain écartait donc, sans
     un mot, 26 544 `MENTIONS` sur la réécriture des 419 fichiers.
   - **La mesure** : 17 991 de ces liens portaient un nom que le scope englobant mentionne déjà. Les 8 553 autres
     étaient ses locales, lues dans une compréhension, et le lien de la fermeture vers son parent. **La perte réelle
     est nulle** ; les déplacer vers le scope aurait ajouté de faux liens.
   - **Le correctif** : ces rendez-vous sont écartés, comme l'est déjà la hiérarchie de la fermeture. Le témoin est
     `e2e_new_name_mentions::a_mention_inside_a_closure_belongs_to_the_scope_that_holds_it`, rouge sur master (cinq
     liens sans bout).
3. **Le silence, en plus des deux défauts.** Un lien écarté faute de bout est maintenant compté et dit : avec un
   exemple au journal du drain, dans `Catalog::take_dangling_links` et à côté de `failed` dans les rapports
   d'ingestion et de réécriture (`dangling_links`). Les témoins sont dans `e2e_dangling_links`. Le compte n'est pas
   encore visible dans `daemon_status` : voir le ticket
   [`2026-11-21-le-compte-des-liens-sans-bout-n-est-pas-dans-daemon-status`](../2026-11-21-le-compte-des-liens-sans-bout-n-est-pas-dans-daemon-status.md).
