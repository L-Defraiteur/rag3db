# Une réaction peut remonter une relation qu'aucun outil n'écrit, et le chargement l'accepte

- **état** : ouvert — **à faire APRÈS** `2026-10-11-aucun-noeud-ne-relie-deux-entites-gerees.md` (voir « l'ordre »)
- **gravité** : déclaration inatteignable acceptée en silence ; aucune perte de donnée
- **atteignable en service** : oui — c'est l'état du gabarit `memory` livré aujourd'hui
- **touche rag3weaver** : oui, et seulement rag3weaver

## Ce que c'est

La section `"reactions"` d'un manifeste se vérifie au chargement, et sept refus
nommés disent chacun quoi corriger (`backend.rs`,
`valider_une_reaction`). Aucun ne regarde si la relation que la réaction
**remonte** a un verbe pour l'écrire.

Une réaction dont la relation n'est écrite par aucun outil est montée, part à
chaque événement de sa source, et trouve toujours zéro ligne concernée. Elle
coûte une exécution par rafale et ne fait rien — et rien ne le dit. C'est le cas
de `review_anchored` dans le gabarit `memory` depuis `0387fe401`, trouvé non pas
au chargement mais en jouant une passe MCP.

## La recette minimale

Charger le gabarit `memory` livré : il charge sans un mot, et sa réaction est
inatteignable. C'est la recette.

## Le témoin

Aucun, et il est facile : un manifeste qui déclare une relation, une réaction
qui la remonte, aucun outil qui l'écrive — le chargement doit refuser en nommant
la relation **et** le verbe qui manque.

## La cause

`ReactTransitionNode` reçoit sa relation par **configuration** (une liaison du
manifeste), donc le nom est connu au chargement ; et les outils déclarés sont
connus au chargement aussi. Le rapprochement n'a jamais été fait parce que la
section `"reactions"` est née un jour avant son premier appelant : au moment où
les sept refus ont été écrits, aucune réaction ne tournait, donc « cette
réaction ne pourra rien faire » n'était pas encore une question.

## Ce qu'il faut pour le fermer

Un huitième refus dans `valider_une_reaction`, qui **se lit sur le manifeste
seul** : pour chaque réaction, relever les relations que ses nœuds nomment en
configuration, et refuser si aucun outil déclaré n'écrit l'une d'elles. Le refus
nomme la relation, la réaction, et dit d'ajouter le verbe.

**Ordre de grandeur : une demi-passe.** Toute l'information est déjà en main au
point de validation ; le travail est de savoir reconnaître « un outil écrit la
relation R » sans câbler de nom de nœud en dur — c'est la seule vraie question
de conception, et elle ressemble à celle que `valider_noeuds_de_reaction`
résout déjà.

## L'ordre, et pourquoi

**Ce refus doit venir APRÈS le nœud de lien générique**, pas avant. Écrit
aujourd'hui, il refuserait le gabarit `memory` **tel qu'il est livré** — la
réaction y est juste, la relation y est juste, et c'est le verbe qui manque au
dépôt entier. Un filet qui rend un gabarit livré inchargeable n'est pas un
filet, c'est une panne. Le nœud d'abord, les deux déclarations du gabarit
ensuite, le refus en dernier : à ce moment-là il n'attrape plus que de vraies
fautes.
