# La variante de consigne sous contexte

15 novembre 2026, session mémoire (96). C'est le lot de la branche `cx-memoire-variante-contexte`,
en pile derrière `usage` et `query_chars`. Il répond au relevé du banc (73278ea4c), transmis par
c7 : sous contexte, le Decide de l'archiviste perd des apports et des sujets.

## Ce que le relevé a vu

- **34 « aide »** : le contexte montrait un sujet qui existe, et le modèle a répondu « rien ». Le
  sujet existe, mais la section n'y est pas rangée, et l'apport du tour est perdu.
- **22 « endort »** : des sujets entiers sont perdus, neuf au moins. La cause est dans le
  gabarit : le contexte arrivait sous le titre « Ce qui précède (déjà rangé, ne pas ranger) », et
  le modèle a pu lire que la section elle-même était rangée.

Le juge du banc s'accorde à 0,90 sur 30. Il est indulgent, parce qu'une fiche large en couvre
plusieurs précises : on garde le chiffre avec cette réserve.

## La forme

Deux paramètres d'`archive`. Leurs défauts sont inchangés, donc la F d'aujourd'hui ne bouge pas.

- **`context_title`**, nouveau : le titre posé devant `context`. Défaut : « Ce qui précède
  (déjà rangé, ne pas ranger) : ».
- **`instruction`**, qui existait : ce que l'étiquette veut dire pour le modèle.

Le banc passe les deux textes de la variante tels quels :

**`context_title`** :

> Ce qui existe déjà dans la mémoire, et ce qui précède ; la section ci-dessous n’est pas encore
> rangée :

**`instruction`** :

> Tu ranges une section d’un document sous l’une des fiches candidates. Le contexte montre ce qui
> existe déjà dans la mémoire ; il ne dit pas que la section est rangée. Si la section parle d’un
> sujet qu’une candidate porte, réponds cette candidate. Si elle parle d’un sujet que le contexte
> montre mais qu’aucune candidate ne porte, réponds create et donne dans name le nom exact de ce
> sujet tel que le contexte l’écrit : la fiche existante sera rejointe. Réponds none seulement si
> la section n’apporte rien : un accusé de réception, une politesse, une répétition sans fait
> nouveau. Un sujet est un chantier, une chose ou une règle, jamais une date, un jour ou un moment.
> En create, donne aussi dans about une phrase complète d’au moins six mots qui dit ce que la
> fiche couvre. Hors de create, name et about sont vides.

Les deux textes n'ont pas d'apostrophe droite : `instruction` est substituée entre apostrophes
dans la configuration du Decide, et la typographique (’) passe.

## Pourquoi « create avec le nom exact » rejoint

Le Decide ne peut choisir qu'une candidate, c'est-à-dire une fiche que la recherche a rappelée
pour la section. Une fiche que le contexte montre, mais que la recherche n'a pas rappelée, ne lui
est pas offerte.

Avant de créer, l'archiviste compare le nom proposé à tous les noms des fiches déjà là
(`archive_subject`, `sujets_voisins`). Un nom identique, ou voisin selon les mots, remplace la
création par la fiche existante. La section devient alors un apport de cette fiche. C'est le
chemin par lequel « rejoindre » passe quand la fiche n'est pas candidate.

## Le témoin

`e2e_archiviste::le_titre_du_contexte_se_regle_et_son_defaut_ne_change_pas`. Le titre réglé
arrive au modèle avant le contexte et la section, et remplace le titre par défaut. Sans
`context_title`, le titre par défaut reste. Le rouge a été vu à 16 h 34 : `context_title` était
un paramètre inconnu.

## Ce qui se mesure

Le banc joue la variante en F 50 k contre la F d'aujourd'hui, sur le même corpus.
L'instruction ne se croit qu'à la mesure : c'est sa raison d'être une variante, et pas un
nouveau défaut.
