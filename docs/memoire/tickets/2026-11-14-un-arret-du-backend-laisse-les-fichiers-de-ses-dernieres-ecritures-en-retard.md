# Un arrêt du backend laisse les fichiers de ses dernières écritures en retard

- **État** : ouvert, le 14 novembre 2026.
- **Famille** : produit
- **Gravité** : réponse fausse, rattrapée. Entre l'arrêt et la réouverture, le dossier des fichiers ne dit pas ce que la base porte. Une copie ou une lecture faite à ce moment voit des fiches manquantes ou périmées.
- **Atteignable en service** : oui, par tout arrêt d'un backend qui vient d'écrire des entités à fichiers (la mémoire : fiches, apports, sources).
- **Touche rag3weaver** : oui, l'écriture des fichiers d'entité (`files_root`) et l'arrêt du backend (`op: shutdown`).
- **Pour** : session mémoire, ou la session qui tient l'écriture des fichiers d'entité.
- **Signalé par** : session mémoire (96), à la première passe de l'archiviste sur la mémoire de Lucie, le 14 novembre 2026.

## Ce qu'on voyait

La passe a joué 52 documents, un processus `rag3weaver-backend <manifeste>` par document, chacun
fermé par `{"op":"shutdown"}`. À la fin, la base portait 116 fiches, mais `memoire-longue/fiches/`
n'en avait que 114 :

- 2 fichiers manquaient : « Scripts d'évaluation de la précision du rappel » et « Durée du rappel
  synchrone », créées par le dernier document ;
- 4 étaient périmés : « hooks-essai », « Classeur local du rappel », « Fiches glissées au rappel »
  et « Contrôle du juge de rappel ».

À la réouverture par l'hôte du portable (14 h 51 min 28 s), l'hôte a réécrit ces six fichiers depuis
la base, ainsi que `manifeste.yaml`.

## Ce qui reste à savoir

- Si l'écriture des fichiers est différée (un lot, une file) et si `shutdown` la coupe, ou si elle
  ne suit qu'une partie des écritures d'une même transaction.
- Un témoin rouge d'abord : un backend de mémoire qui crée deux fiches puis reçoit `shutdown`, et
  dont le dossier `fiches/` doit porter les deux à la sortie.

## Les preuves

À luciepc, `~/.cache/rag3weaver-build/passe-96-reelle/base/` est la base telle que la passe l'a
laissée (114 fichiers de fiche, 116 fiches en base). Les six fichiers réécrits sont datés de
14 h 51 min 28 s dans `~/memoire-longue/fiches/`, au portable.
