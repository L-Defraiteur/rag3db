# Un arrêt du backend laisse les fichiers de ses dernières écritures en retard

- **État** : corrigé le 14 novembre 2026, dans le lot qui l'ouvre (session mémoire, 96).
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

## La cause

La réaction de miroir (`Backend::monter_le_miroir`) réécrit les fichiers composés 50 ms après une
rafale d'écritures (`ReactPolicy::Debounce(50)`). `ReactorHandle::stop_within` casse sa boucle tout
de suite : ce qu'elle attendait encore (le lot à 50 ms, et ce qui restait dans sa file) était perdu.
L'ouverture régénère déjà tout depuis la base (`regenerer_les_fichiers_composes`), ce qui
rattrapait le retard, mais seulement à la réouverture suivante.

## Le correctif

`Backend::stop_reactors` régénère les fichiers composés depuis la base dès que le réacteur du miroir
est arrêté, avant que l'arrêt ne rende. La base est la vérité, et les fichiers la suivent jusqu'à
l'arrêt compris. Un miroir figé n'est pas touché, car il tient peut-être le catalogue. Le chemin
couvre `shutdown`, la fermeture par l'hôte (`POST /backend/close`) et le rechargement des
déclarations.

## Le témoin

`e2e_memoire_fichiers::un_arret_laisse_les_fichiers_a_jour_de_la_base` : une fiche réécrite et une
fiche née juste avant `shutdown`, sans attente. Leurs fichiers doivent être à jour quand l'arrêt
rend. Rouge sur master 4547f41a9 (la fiche neuve n'avait pas de fichier), vert après.
`e2e_memoire_fichiers` : 4 sur 4.

## Les preuves

À luciepc, `~/.cache/rag3weaver-build/passe-96-reelle/base/` est la base telle que la passe l'a
laissée (114 fichiers de fiche, 116 fiches en base). Les six fichiers réécrits sont datés de
14 h 51 min 28 s dans `~/memoire-longue/fiches/`, au portable.
