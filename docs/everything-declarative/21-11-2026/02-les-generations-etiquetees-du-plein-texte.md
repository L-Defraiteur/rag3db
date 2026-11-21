# Les générations étiquetées du plein texte : quel commit, quelle génération

21 novembre 2026, nuit. Sessions everything declarative et lucivy, à la demande de c7. **Une décision pour Lucie** :
deux propositions, en fin de page, avec notre recommandation.

## Ce qui existe aujourd'hui dans rag3weaver

- **Le commit du plein texte d'un paquet**, sur le chemin des versions : à chaque paquet validé, les fichiers du plein
  texte sont synchronisés (`sync_generation`), puis une génération de stockage est marquée des deux côtés. Elle est
  d'abord marquée dans le dossier, puis dans la méta `fts_generation:<index>` de la base, sous la même transaction que
  les lignes (`catalog.rs`, vers la ligne 9905).
- **Cinq commits simples, hors de ce chemin** : le tampon d'un lot et le geste des non-candidats
  (`appliquer_le_tampon`), le rebâti par lots (`rebuild_fts_step`), la réindexation d'une entité (`reindex`), le
  rebâti d'une cellule et celui d'une table.
- **Un écart entre la base et le dossier** (la base nomme N, le dossier est à N + k, le cas du 810) se règle aujourd'hui
  en jetant le plein texte et en le rebâtissant (`fts_directory.rs` : « on ne garde pas la génération N−1 pour y
  revenir, on rebâtit »).
- **La génération d'une couche** (le rang publié que lit le catalogue) se publie à la fin d'une écriture de couche,
  par `publier_la_generation` (`history_layers`).

## Ce qu'apporte lucivy 4.4.0 (la part de lucivy)

Conçue pour la marque par paquet (`docs/lucivy/20-11-2026/02-conception-lucivy-4-4-la-voie-b.md`), selon la
précision de Lucie : « la base ne garde que la dernière génération validée et celle en cours ».

- `commit_tagged(g)` : un commit qui étiquette l'état du dossier. Il refuse un `g` inférieur ou égal au courant : `g`
  est croissant et unique par index (un index, c'est une table d'une cellule).
- Chaque shard porte `lucivy-generation:<g>` : l'état du dossier se lit sans écrire.
- `revert_to(g)` : le retour d'un coup à une génération gardée.
- `release_below(g)` : le ramasse-miettes relâche ce qui est sous `g`. Juste après le COMMIT durable, il reste au plus
  deux générations.
- `commit_keeping_generation()` : un commit qui change des documents sans créer de génération.
- Sur un index étiqueté, `commit()` simple est refusé ; `maintain` et `compact` refusent s'il reste des documents non
  commités.
- **Coût de `commit_tagged`** : un commit ordinaire, plus, par shard, l'attente du pli du dictionnaire, la copie d'un
  méta de quelques Ko et de ses stats, et cinq petits JSON. Il ne dépend pas du nombre de documents ; il n'est pas
  encore mesuré sur 100 000. Le disque gardé est de quelques Mo par paquet, au pire un index entier pendant une fusion
  complète (+850 Mo pour le dépôt), le temps d'un paquet.

## Les deux propositions

**(A) La génération du paquet** (lucivy ; ED s'y range) :
1. `g = fts_generation:<index>`, le numéro de paquet, déjà croissant, par index, validé avec les lignes ;
2. le commit lucivy du paquet devient `commit_tagged(g suivant)`, avant `sync_generation` puis
   `mark_next_generation`, qui rend ce même `g`, persisté avec les lignes ;
3. `release_below(g)` après le COMMIT durable ;
4. à l'ouverture, un dossier au-dessus de la base se remet à la génération de la base par `revert_to`, **sans
   rebâti** ; un dossier en dessous, ou une génération non gardée, garde le rebâti ;
5. les cinq commits hors version passent à `commit_keeping_generation()`.

**(B) Le rang publié de la couche** (la première lecture de c7) : `g` est le rang publié, étiqueté par
`publier_la_generation` ; le retour arrière d'une couche remet le plein texte à `g` d'un coup.

## Pourquoi (A)

- **(B) ne tient pas la contrainte de lucivy** : plusieurs couches (les fourches) partagent un même plein texte par
  table, et leurs rangs ne sont, pour lui, ni croissants ni uniques. `commit_tagged` les refuserait.
- **(B) ferait grossir le disque sans borne** : garder l'état publié de chaque couche, pour y revenir, oblige à
  retarder `release_below`. Ce n'est pas l'usage prévu.
- **(A) règle le défaut réel** : l'écart entre la base et le dossier après un arrêt brutal, qui se règle aujourd'hui par
  un rebâti complet (minutes sur le dépôt), se règle en un `revert_to`.
- **Le retour d'historique d'une couche** reste ce qu'il est aujourd'hui : un rebâti, ou une décision à part si on le
  veut instantané un jour.

## Ce que Lucie tranche

1. **(A) ou (B)** ; notre recommandation : **(A)**.
2. **Qui fait le lot** : la page de la voie B le confiait à l'arbre principal. Il touche l'écriture par paquet du
   catalogue (`catalog.rs`) et l'ouverture du plein texte (`fts_directory.rs`), avec le pointeur 4.4.0 de lucivy.
3. **Quand** : la 4.4.0 n'est pas publiée (le tag est au mot de Lucie).
