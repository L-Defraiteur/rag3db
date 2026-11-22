# Un arrêt pendant la repasse de l'analyse attend la fin du fil

- **État** : ouvert (22 novembre 2026, 21 h 13)
- **Famille** : produit
- **Gravité** : moyenne. Un hôte dont une base repasse son analyse à l'ouverture met environ 41 s à sortir après un
  TERM, deux fois de suite ; moins de 20 s le matin, avant le lot `repass-on-open`. C'est au-delà du seuil d'arrêt,
  et une relance de service attend d'autant.
- **Atteignable en service** : oui. Relevé par G à la pose de 21 h 2x, à chaque arrêt pendant une repasse.
- **Touche rag3weaver** : oui (l'arrêt de l'hôte, les fils de fond d'un catalogue, `code_sync::sync_source`)
- **Pour** : session codeparsers (c7, 22 novembre 2026)
- **Signalé par** : G (relevé à la pose), c7

## Constat

Depuis `repass-on-open` (PR 516), un backend dont les lignes `File` portent une autre version de l'analyse relance son
index entier en fond à l'ouverture. À l'arrêt, l'hôte arrête les fils de fond de chaque catalogue et **les attend**
(`catalog_threads::stop_and_join`, au prochain paquet de chacun). Pendant une repasse, ce prochain paquet arrive tard :
l'hôte a mis environ 41 s à sortir, deux fois de suite.

## Ce qu'il faut

- **Un TERM pendant une repasse arrête le fil au prochain paquet**, et l'hôte sort en moins de 10 s. Le journal de
  l'index dit déjà qu'une indexation arrêtée « reprend là » à la relance.
- **La repasse reprend à la prochaine ouverture** : les lignes non repassées portent toujours l'ancienne version, et
  `a_l_ouverture` la relance (rien à garder de plus).
- **Témoin rouge d'abord** : une base ouverte en repasse, un arrêt de l'hôte demandé pendant la passe, la sortie
  mesurée ; puis la reprise à la réouverture.

## Pistes à vérifier

- Où le fil de la synchronisation regarde son drapeau d'arrêt (`SourceSyncOptions.stop`) pendant une repasse : par
  fichier, par paquet, ou seulement entre les phases (nœuds, relations, plein texte, vecteurs).
- Si l'attente vient d'une phase longue qui ne lit pas le drapeau : la dérivation des relations, le plein texte du code
  après les relations, ou la publication de la génération.
