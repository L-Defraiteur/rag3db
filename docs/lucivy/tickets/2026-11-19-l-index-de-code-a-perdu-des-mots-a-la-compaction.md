# L'index de code a perdu des mots : la compaction du dictionnaire ne voit pas les documents en cours

- **État** : ouvert (19 novembre 2026), en cours (session lucivy).
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

## Ce qu'on suppose (témoin en cours)

`maintain` avec `compact` garde les ids que citent les `.gmap` des segments **commités**. Il refuse de tourner
pendant une fusion ou avec un segment non commité, mais un document encore dans l'indexeur n'est dans aucun
segment. S'il a réutilisé l'id d'un mot que seuls des documents supprimés portaient (l'ancienne version de son
fichier, réindexé), la compaction retire ce texte, puis le segment du document cite un id perdu.

## Comment le rejouer

- La mesure : `lucivy_core/tests/test_dictionary_composition.rs`, `where_the_lost_words_sit` (branche lucivy
  `measure-dictionary`, 9a65502), `LUCIVY_DICT_INDEX=<copie>`.
- Le témoin : `lucivy_core/tests/test_compaction_keeps_words_in_flight.rs` (branche lucivy
  `compaction-keeps-words-in-flight`, 81f58d6).

## Après le correctif

Le correctif arrête les nouvelles pertes, il ne rend pas les mots perdus. Les bases touchées doivent rebâtir leur
plein texte : c'est le rebâti sans positions déjà prévu à une fenêtre de G, sur copie d'abord.
