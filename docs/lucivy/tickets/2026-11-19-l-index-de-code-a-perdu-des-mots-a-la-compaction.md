# L'index de code a perdu des mots : la compaction du dictionnaire ne voit pas les documents en cours

- **État** : en cours (19 novembre 2026, soir). Le sursis d'un cycle (PR lucivy #50) est un filet, pas le correctif : la perte est reproduite avec lui.
- **Gravité** : justesse. Un document dont un mot est perdu ne sort plus pour ce mot, en exact comme en flou. Rien
  ne le signale à la recherche.
- **Atteignable en service** : oui. C'est mesuré sur une copie de l'index de code de la base du dépôt (tablette, le
  19 novembre à 12 h 08).
- **Touche rag3weaver** : oui, par la maintenance nocturne du plein texte (`fts_maintenance`, avec compaction) qui
  tourne pendant qu'un index écrit.
- **Famille** : produit
- **Pour** : lucivy

## Ce qui se passe

`check_dictionary` sur la copie de `Lucivy_Scope` : dans chaque shard, des identifiants que des segments vivants
citent n'ont plus de texte dans aucune génération du dictionnaire.

| champ | mots perdus (4 shards) |
|---|---:|
| content | 4 748 (989, 1 318, 632, 1 809) |
| docstring | 2 213 |
| name | 283 |

Tous les ids perdus sont dans la plage de la génération compactée de leur shard, vers sa fin (2,0 à 2,2 millions sur
2,24) : ils sont récents. Dans chaque shard, un segment de 530 à 620 documents, sans suppression, en cite la plupart
(936 sur 989 dans le shard 0) ; le grand segment fusionné cite le reste.

## Ce que l'enquête a établi (19 novembre 2026)

- **Les pertes suivent chaque compaction nocturne.** Les instantanés de la tablette du 16 novembre (10 h 22) et du
  18 novembre (15 h 29) les avaient déjà, avec les mêmes ids anciens. Après chaque compaction (16 nov. à 03:30,
  19 nov. à 09:09, maintenance nocturne rattrapée au réveil de l'hôte), des ids créés peu avant elle, 15 000 à
  40 000 sous le maximum de la génération compactée, n'ont plus de texte. Un segment récent d'environ 1 000
  documents, presque sans suppression, les cite.
- **Quatre pistes réfutées**, chacune par un témoin dans `lucivy_core/tests/test_compaction_keeps_words_in_flight.rs`
  ou par une lecture :
  1. un document non commité qui réutilise un id : `compact_dictionaries` tient le verrou de l'écrivain et refuse
     avec des documents non commités (depuis la 4.3.8) ;
  2. la table des créations en attente non vidée après un commit sans paire neuve ;
  3. une compaction seule sur une copie réelle (index du 18) : 0 mot perdu ;
  4. deux handles sur le même dossier : la maintenance nocturne de l'hôte prend le même catalogue et le même handle
     que l'indexation (`daemon_host.rs`, sous le slot en écriture).
- La perte demande donc un segment hors de la vue de la compaction au moment où elle tourne. La cause exacte n'est
  pas nommée.

## Le correctif : le sursis d'un cycle (PR lucivy #50)

La compaction ne jette plus un texte mort. Elle l'écrit dans une génération en sursis, lue comme les autres. La
compaction suivante garde les textes du sursis qu'un `.gmap` cite de nouveau et **journalise chacun avec le segment
qui le cite** : c'est la cause, qui sera nommée à la prochaine occurrence. Elle laisse partir les autres.
`check_dictionary` compte à part les ids en sursis (`in_reprieve`). Témoin
`test_failpoints_a_compaction_that_misses_a_segment_loses_no_word` (suite `failpoints`, un point de faute cache un
segment à la compaction) : 7 mots perdus sans le correctif, 0 avec.

## La perte reproduite (19 novembre 2026, soir)

Dans la CI de la PR de version 4.3.12 (lucivy #51), sur un runner à 4 cœurs, le test de charge
`rewriting_files_while_the_maintenance_compacts_loses_no_word` (`lucivy_core/tests/test_compaction_keeps_words_in_flight.rs`)
a perdu **152 mots**, sursis compris. En 4 compactions, 196 textes sont revenus du sursis, cités par 8 segments
différents. Les compactions ratent donc des segments **sans cesse**, et un segment raté deux fois de suite perd ses
mots. La 4.3.12 est suspendue.

**Piste en cours de preuve** : la compaction lit les champs de chaque segment par son méta (`reader.sfx_fields()`).
Un segment fraîchement fusionné dont le méta ne porte pas encore ses champs ne serait pas lu, alors que
`check_dictionary` passe par le schéma. Branche lucivy `debug-missed-segments` : la compaction y journalise, sous
`LUCIVY_DEBUG_COMPACTION`, les segments commités, ceux du gestionnaire, et par segment ses champs et ses `.gmap`
lus. Le test tourne en boucle sous `taskset -c 0-3` jusqu'au rouge.

Ensuite : un témoin ciblé (un segment fusionné au méta sans champs, juste avant la compaction, perd ses mots), puis
le correctif de la cause. Le sursis reste comme filet.

## Ce qu'il reste

- Le correctif de la cause, puis la 4.3.12 au mot de Lucie, puis le pointeur dans rag3weaver. La compaction est coupée sur les hôtes depuis le
  19 à 17 h 44 (G) ; elle revient avec le pointeur.
- Le rebâti du plein texte des bases touchées rend les mots perdus : avec le profil léger, puis `check_dictionary`.
- La sonde de nuit, par le chemin de rag3weaver (branche rag3db `probe-lost-words`) ; et la lecture du journal de
  l'hôte après la prochaine compaction, pour y trouver la ligne « a text in reprieve is cited again ».

## Comment le rejouer

- La mesure : `lucivy_core/tests/test_dictionary_composition.rs`, `where_the_lost_words_sit` (branche lucivy
  `measure-dictionary`, 9a65502), `LUCIVY_DICT_INDEX=<copie>`.
- Le témoin : `lucivy_core/tests/test_compaction_keeps_words_in_flight.rs` (branche lucivy
  `compaction-keeps-words-in-flight`, 81f58d6).

