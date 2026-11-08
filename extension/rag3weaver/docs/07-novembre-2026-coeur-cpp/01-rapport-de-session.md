# Cœur C++ — 7 novembre 2026 : rapport de session

Reprise après la nuit du 25 au 26 octobre (lots V2 sur master, voir
`../25-octobre-2026-coeur-cpp/01-rapport-de-session.md`). Programme de c7 : nommer les worktrees
périmés, libérer le disque, atterrir ce qui est à moitié fait, puis la lib unique de la bascule 43.

## Inventaire et ménage

| ici | verdict |
|---|---|
| rag3db-moteur-v21b (projection-lecture-seule-2 → -3) | en cours, rebasé sur master, rejeu de la suite transaction |
| rag3db-moteur-lecture (lecture-en-memoire, un wip non mesuré) | périmé : worktree et bâti effacés ; branche distante à retirer |
| rag3db-moteur-tsx (moteur-tsx-cjs, dans master) | périmé : effacé, l'essai non commité gardé en patch hors dépôt |
| ~/.cache/rag3weaver-build/coeur-cpp (2,1 Go, sondes d'octobre) | effacé |
| luciepc : rag3db-colonnes + build, target-coeur-c (13 Go) | gardés, un bâti moteur et un target cargo |
| luciepc : target-coeur (4,8 Go) | effacé ; rag3db-sonde gardé le temps de la bissection et de la passe |

## Le bloquant du jour : une lecture sale

`RollbackUnderReaders.AReaderSurvivesTheRollbackOfAnotherWriter` rouge dans le rejeu du retrait
(nuit du 26), relu ce matin : un lecteur compte 924 ou 2 058 relations au lieu de 10 pendant un
COPY annulé par un autre écrivain. Rejeux : 0/40 à 04063ee06, 4/60 sur master a91f066ea, 3/40 sur
le retrait. Bissection : 6b63e4922 0/40, **f92b21d56 (V2-1b pièce 2) 2/40**, 3724accba 1/40.

Cause : `ChunkedNodeGroup::rollbackInsert(startRow = 0)` effaçait l'information de version
(`versionInfo.reset()`), et un balayage sans versions sélectionne toutes les lignes demandées ; un
lecteur qui tient le bloc et son compte de lignes d'avant (le balayage tient ses blocs depuis
V2-1b pièce 2) voyait tout le bloc : 2 048 = un bloc plein, 914 = le bloc en cours. Correctif
`b7447937d` (sur master en `42b85dc62`) : toute annulation marque ses lignes annulées puis tronque, jamais de
remise à zéro ; témoin déterministe au grain du bloc (le compte d'avant, le balayage après), le
probabiliste 60/60. Ticket du jour fermé par ce lot.

Leçon du témoin : la première forme (`isInserted` ligne par ligne) était verte sur master, parce
qu'`isInserted` est gardé par le compte de lignes du bloc ; `scan` ne l'est pas, et c'est par
`scan` que passe la lecture. Un témoin déterministe d'une course rejoue le chemin de la course.

La piste donnée par c7 pour hotfixs : la même cause peut faire des demi-arêtes LOCKS/QUERIES (un
sens dans un bloc entièrement annulé, l'autre dans un bloc partagé) ; hotfixs écrit le témoin
`LateRelTableRewrites.BothDirectionsStayEqualUnderRolledBackCopyDrains`.

## Le soir : quatre lots de plus

| lot | sur master | ce qu'il corrige |
|---|---|---|
| ordre-des-verrous | `5f0b79466` (moteur `e526cc4b6`) | l'interblocage entre la compaction des listes CSR (index puis blocs) et un lecteur qui prend sa génération (blocs puis index) : VectorSearchDuringInsertTest figé une fois sur cinq, et C7_RandomMix/Thread_Crash du banc (40/40 après) |
| instruction-en-echec | `17a094d12` | une instruction en échec ne condamne plus une transaction en lecture seule ; une interruption vaut pour la requête |
| projection | `e86d2f688` | PROJECT_GRAPH_* sous READ ONLY, et DROP_PROJECTED_GRAPH honore skip_if_not_exists (l'option du produit, ignorée, condamnait la recherche dense sous l'instantané) |
| retrait des générations | `b3bc6e479` (moteur `7cd11e5fa`) | les générations précédentes se retirent à chaque point de reprise, dans toutes les tables |

La lib unique de la bascule 43 a été bâtie depuis `17a094d12` et posée par G à 19 h 57 ; la
projection et le retrait feront la lib ordinaire de demain matin.

Leçons du soir :
- une suite qui dépasse son délai est un figement jusqu'à preuve du contraire : trois suites
  « lentes » de l'après-midi étaient l'interblocage ; la pile se prend en relançant sous
  `LD_PRELOAD=ptracer-any.so`, puis `eu-stack -p` ;
- `cmake --build --target concurrence_test` ne rebâtit pas l'extension vector : deux passes ont
  rougi sur la réouverture et ExtensionBuildIdTest pour cette seule raison ; toujours le bâti
  complet avant un banc ;
- jamais deux passes produit en même temps sur un target cargo : « failed to write fingerprint »
  n'est pas un rouge du code ;
- lire la ligne du témoin, pas la fin de la passe : la projection du 26 octobre était annoncée
  verte avec son témoin rouge ;
- un vert de hasard au banc se départage seul (SameRowsFirstCopy : 1 vert sur 11) avant de
  toucher à known_red, et le fichier va à qui a la mesure.

## Le groupe 1 : deux tickets non reproduits

Les deux tickets de la validation et de l'annulation ont reçu le témoin qu'ils demandaient, et ne
se reproduisent plus sur master ; ils se ferment avec leur témoin en garde, sans changer le moteur.

| ticket | témoin | sur master |
|---|---|---|
| 2026-10-10 validation à moitié appliquée | `HalfAppliedCommit` : un index d'essai lève à l'étape 3 de `NodeTable::commit`, après l'ajout des lignes ; deux cas, dont une suppression validée, une mise à jour, 2 100 lignes et une réouverture | `5b35a5a70` |
| 2026-10-18 annulation d'un COPY sous un tampon plein | `CopyRollbackUnderAFullBuffer` (banc) : B valide 151 250 lignes dans une autre table jusqu'aux refus, le ROLLBACK du COPY de A passe, la base reste ouverte | `15f16bcdc` |

Leçons : un témoin vert se lit par son plan avant de clore un ticket (la première forme du
second ne remplissait pas le tampon, le COPY de A tenant la table de B) ; une doublure d'index se
retire avant le point de reprise de la fermeture, qui la sérialiserait.

## À suivre

La durabilité sur faute d'entrée-sortie ou coupure (2026-10-04), témoin de la coupure d'abord par
injection de faute dans `FileInfo::writeFile` et le fsync ; puis V2-2c (format 44) et V2-3. La
lib ordinaire de G, demain matin, porte la projection et le retrait.
