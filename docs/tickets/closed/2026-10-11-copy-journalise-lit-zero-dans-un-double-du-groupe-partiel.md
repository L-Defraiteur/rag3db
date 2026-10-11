# Le COPY journalisé lit 0 dans une colonne DOUBLE du groupe partiel (deux fils, une colonne tableau)

- **État** : corrigé par le cœur C++, `eae3863ab` (master `60d74a4e2`, 11 octobre 2026, nuit), avec
  son témoin `10a796166` (`JournaledCopyReadBackTest`). Le filet de rag3weaver est retiré.
- **Gravité** : perte. Les valeurs relues sont fausses, le journal les recopie, et le rejeu après un
  arrêt brutal les écrit pour de bon.
- **Atteignable en service** : oui, sans le filet. Il faut un COPY de nœuds de plus d'un groupe
  (131 072 lignes), au moins deux fils, une colonne tableau (`FLOAT[N]`, la forme des
  embarquements) et un seuil du journal qui laisse passer le COPY. Ce seuil est tiré du tampon :
  sous un tampon de plusieurs Gio, un COPY de 200 000 lignes passe journalisé.
- **Touche rag3weaver** : oui. Ses entités portent un `FLOAT[N]` à côté de leurs champs.

## Ce que c'est

Une base neuve, `Doc(id INT64 PRIMARY KEY, name STRING, score DOUBLE, vec FLOAT[4])`, puis un COPY
de 200 000 lignes avec `copy_journal_threshold` à 1 Gio et `auto_checkpoint=false`. Ensuite,
`MATCH (d:Doc) WHERE d.score <> d.id + 0.5` rend 130 à 290 lignes, toutes vers la fin
(ids d'environ 199 400 à 199 999), avec un score lu 0. Les noms sont justes. Rien ne meurt et
rien n'est rejoué : c'est le processus vivant qui lit faux.

Un `CHECKPOINT` dans le même processus rend les valeurs justes. Les données en mémoire sont donc
saines, et c'est le balayage du groupe partiel non encore écrit qui rend 0. Ce même balayage
alimente `NodeTable::logInsertedRowsToWAL` : le journal porte les zéros, et le rejeu après une mort
les écrit. Le rouge probabiliste d'`OwnerlessPagesTest.TheReplayReadsNothingBeyondTheExtent`
(ticket `2026-10-11-rejeu-d-un-copy-tue-la-colonne-score-des-dernieres-lignes-a-zero.md`) en est la
trace, une fois passé par la mort et le rejeu.

## Les combinaisons

Arbre principal, 11 octobre 2026, vers 3 h 30. Lib de master `80054b530`, trois essais par ligne.

| mode | colonne `vec` | fils | lignes | résultat |
|---|---|---|---|---|
| journalisé (seuil 1 Gio) | `FLOAT[4]` | 2 | 200 000 | **faux 3 fois sur 3** (134, 184, 241 lignes) |
| journalisé, puis `CHECKPOINT` dans le même processus | `FLOAT[4]` | 2 | 200 000 | faux avant, **juste après** |
| journalisé | `FLOAT[4]` | 1 | 200 000 | juste |
| journalisé | `STRING` | 1 ou 2 | 200 000 | juste |
| journalisé | `FLOAT[4]` | 2 | 20 000, 60 000, 130 000 | juste (sous un groupe) |
| seuil par défaut (petit tampon : repli) | toutes | toutes | 200 000 | juste |
| `force_checkpoint_on_copy=true` | toutes | toutes | 200 000 | juste |

Avec mort et rejeu (sonde `sonde.cpp`) : l'enfant vivant lit déjà faux, avec la même somme que la
base rejouée, puis passée par un `CHECKPOINT` et rouverte.

Sondes, sans gtest, liées à la lib du moteur : `~/.cache/rag3weaver-notes-A/etendue/variantes.cpp`
(`./variantes run journalckpt vec 2`) et `sonde.cpp`.

## Le filet

`Rag3dbConnection::connect` pose `CALL force_checkpoint_on_copy=true` sur chaque connexion,
c'est-à-dire le point de reprise forcé, l'ancien défaut. Coût mesuré le 10 octobre : 81 s contre
80 s sur la passe de référence, environ 1 %. C'est un changement de défaut visible, posé par
l'orchestration pendant la nuit, et Lucie peut le renverser. Le témoin
`rag3db_connection::tests::un_copy_de_plus_d_un_groupe_avec_un_tableau_garde_ses_doubles` est
rouge sans le filet et vert avec.

## Pour le fermer

Le correctif du balayage côté moteur (cœur C++), avec son témoin, la sonde verte en boucle, puis
`OwnerlessPagesTest.*` en boucle. Ensuite seulement, retirer le filet de `connect` : le témoin
rag3weaver reste et doit rester vert sans lui.

## La cause et le correctif (cœur C++)

Le groupe plein que le COPY écrit sur disque a deux segments pour la colonne `score`
([99 840, 31 232] valeurs). Avant son point de reprise, les versions de l'insertion du COPY rendent
la sélection du balayage filtrée, même quand tout est sélectionné. Le second segment porte des
exceptions ALP à ses premières positions, et `patchFloatExceptions` (`column_reader_writer.cpp`,
code repris de l'amont) passait au filtre un index absolu dans le vecteur, là où le lecteur de
pages lui donne un index relatif au balayage. Pour tout segment après le premier, aucune exception
n'était rapiécée : ces valeurs se lisaient à leur encodage ALP, 0. La relecture par la clé était
juste (un seul segment, décalage 0). Le correctif tient en une ligne : l'index relatif.

Les lignes fausses portaient les ids ~199 5xx parce que la pièce du second fil a été fusionnée dans
le groupe plein. Il fallait plus d'un groupe, et deux fils : sinon la forme n'existe pas.

## Le retrait du filet

Le témoin rag3weaver `un_copy_de_plus_d_un_groupe_avec_un_tableau_garde_ses_doubles` est vert sans
le filet sur la lib rebâtie à `60d74a4e2`. Le point de reprise forcé n'est plus posé par
`Rag3dbConnection::connect`, et la page des défauts a perdu sa ligne « filet temporaire ».
