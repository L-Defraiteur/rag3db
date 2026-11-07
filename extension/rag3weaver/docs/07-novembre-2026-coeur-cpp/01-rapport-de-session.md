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

## À suivre

Le retrait des générations (v22-retrait-2, à rebaser et rejouer sur le correctif), puis la lib
unique de la bascule 43 par l'arbre principal, bâtie depuis master avec le correctif.
