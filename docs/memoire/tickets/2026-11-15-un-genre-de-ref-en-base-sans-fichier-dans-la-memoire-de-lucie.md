# Un genre de Ref en base sans fichier, dans la mémoire de Lucie

- **État** : ouvert, le 15 novembre 2026. Rien n'est touché sur la mémoire de Lucie sans sa décision : c'est sa base.
- **Famille** : produit
- **Gravité** : écart entre la base et son dossier. Il est sans perte connue, mais la question de la vérité n'est pas tranchée.
- **Atteignable en service** : oui. `daemon_status` le dit : « dossier ≠ base : 1 écart(s) ».
- **Touche rag3weaver** : oui, le miroir des fichiers (`regenerer_les_fichiers_composes`, qui ne régénère que les fichiers composés) et l'écriture des fichiers simples d'une entité (`RefType`, `genres/<nom>.md`).
- **Pour** : session mémoire (96), puis Lucie pour la décision.
- **Signalé par** : `daemon_status` de l'hôte du portable, dans la nuit du 14 au 15 novembre 2026, vu par la session mémoire.

## Ce qu'on voyait

`daemon_status`, backend `memoire-longue` : `files.by_entity.RefType = [{kind: row_without_file, uuid: 69e7d6f0-2a72-f3ea-2907-13d0a53f2668}]`, `gaps: 1`, `regenerated: 0`.

Le dossier `~/memoire-longue/genres/` porte cinq fichiers : `agent`, `alias`, `doc`, `handle` et `same_as`. Ce sont les mêmes dans les trois instantanés du 14 novembre (13 h 46, 17 h 16, 18 h 20). L'écart est donc antérieur aux deux passes de l'archiviste de ce jour-là.

Le miroir corrigé à l'arrêt (`27f5c21a1`) ne régénère que les fichiers **composés** (les fiches) : un fichier simple comme celui d'un genre n'est pas rattrapé.

## La question à trancher

- **Le miroir régénère aussi les fichiers simples** depuis la base. La base est la vérité, comme pour les fiches, et le fichier manquant revient.
- **Ou le dossier est la vérité** pour les genres. La ligne est alors un orphelin, à retirer, après avoir lu ce qu'elle porte.

Avant de trancher, il faut lire cette ligne sur une **copie** : son nom, sa description, qui l'a écrite et quand. Puis dire à Lucie ce qu'elle est.
