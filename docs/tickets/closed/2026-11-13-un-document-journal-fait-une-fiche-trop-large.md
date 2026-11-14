# Un document journal fait une fiche trop large

- **État** : corrigé le 14 novembre 2026 (session mémoire, 96), après la décision de Lucie : une fiche par chantier, pas par moment. Ouvert le 13 novembre.
- **Famille** : produit
- **Gravité** : réponse fausse. Une fiche réunit des sujets sans rapport, et son rappel rend des sections hors de propos.
- **Atteignable en service** : non, la mémoire de Lucie n'a pas encore l'outil `archive` (M8).
- **Touche rag3weaver** : oui, l'outil `archive` du gabarit `memory` (son instruction, ou un traitement propre aux documents journaux).
- **Pour** : session mémoire, sur décision de Lucie ; relevé par l'orchestration (c7).
- **Signalé par** : [le second archivage à blanc](../../memoire/13-11-2026/03-le-second-archivage-a-blanc.md) (blanc-96-2).

## Ce qu'on voyait

Le second archivage à blanc a fait deux fiches, « Journal du 7 novembre 2026 » et « Journal du 8
novembre 2026 ». Chacune réunit les sections d'un document journal, qui parlent de chantiers
différents. Le modèle prend le document pour le sujet, alors que les sujets sont les chantiers dont
le journal parle.

## Ce qui a été décidé

Lucie, le 14 novembre 2026, en choisissant la voie 3 (la création par sujet, avec le dédoublonnage) :
**un document journal donne une fiche par chantier, pas par moment**.

Le troisième archivage à blanc avait remplacé « Journal du 7 » et « Journal du 8 » par des fiches de
moments : « Relance du 7 novembre 2026 », « Matinée du 8 novembre 2026 », « Nuit du 7 au 8 novembre
2026 ». Ce sont toujours des fiches de provenance, pas de sujet.

## Le correctif

- **L'instruction de l'archiviste** (`archive.mmd`) dit au modèle qu'un sujet est un chantier, une
  chose ou une règle, jamais une date, un jour ou un moment. Dans un journal ou un rapport de session,
  il nomme le chantier dont parle la section.
- **Un nom daté est refusé** (`archive_decided.rhai`, `nom_date`) : un nom de fiche neuve qui porte un
  mois, ou un jour de la semaine, avec un nombre. La section est dite en erreur dans le rapport, avec
  le nom et la raison, et elle reste dans le L0. Une fiche déjà là reste joignable quel que soit son
  nom.

## Les témoins

- `e2e_archiviste::un_journal_donne_une_fiche_par_chantier_pas_par_moment` : un journal de deux
  sections. Le faux modèle propose « Matinée du 8 novembre 2026 », refusé et dit, puis « Bascule au
  format 43 », qui fait sa fiche, `proposed`. Rouge sur master 5c119e436 (l'instruction ne parle pas
  de chantier), vert après.
- `e2e_archiviste::une_fiche_courante_rejointe_reste_courante` : le premier garde-fou de la voie 3,
  vert dès sa pose (un verrou, pas un correctif). L'archiviste crée en `proposed`, et seule la
  relecture de Lucie (la transition `accept`) fait une fiche `current`. Une fiche `current` qu'une
  section rejoint ne change pas d'état.
- `e2e_archiviste` : 11 sur 11, sur la lib 0c41f9fe6.

## Ce qui reste

- **Un nom de moment sans date** (« La matinée », « La relance ») passe. Si une vraie passe en montre,
  la liste s'allongera.
- **La fusion des fiches voisines par le sens** reste une piste, non codée : voir le ticket clos
  [du même sujet sous deux formulations](2026-11-13-le-meme-sujet-sous-deux-formulations-fait-deux-fiches.md).
