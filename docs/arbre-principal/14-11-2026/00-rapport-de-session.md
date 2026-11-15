# Arbre principal, le 14 novembre 2026 (et la nuit du 15) : le rapport de session

## Ce qui a atterri sur master

| Lot | Master | Ce qu'il fait |
|---|---|---|
| La parente d'une tâche suivie | 912ea2084 | la parente déclarée d'une tâche entre aux suivies ; une déclaration écartée ne s'écrase jamais |
| Le court-circuit du masque, puis 1b | 194a4e572 | sans filtre ni vue ni orpheline, la recherche à versions ne masque rien (406 s → 12 s sur 24 requêtes, réponses identiques) ; la migration 1b vers les versions, par lots bornés, sans appel au modèle |
| Le crochet en `Weak` | a76f36a08 | le crochet de test du point de reprise échoué ne retient plus sa base (le rouge de l'hôte depuis le 13) |
| Les suivies (`tracked`) | 844d01247 | un seul objet générique pour les couches d'un dépôt : la racine, les sources à sélecteur git, la pile, le flux, l'identifiant stable des couches, le branchement dans l'état et l'atelier |
| Le défaut `cited` d'une source `refs:` | 4eb274033 | un motif large ne s'écrit qu'à la citation |

Les pages : `01-1b-sur-la-copie-de-code-rag3db-depot.md`, `02-les-suivies-tracked.md`,
`03-2a-prime-3-ce-que-les-suivies-apportent.md`. Les tickets ouverts : le tri d'une jointure trop grande
(souhait pour le cœur), l'aide des tests qui panique sans service (pour embarquement), la recherche à
versions qui masque tout sans filtre (fermée par le court-circuit, la dépendance à 2a′-2 dite).

## Reprendre

**Aucun worktree de code n'est vivant** : tous les lots ont atterri. Seul reste le worktree de docs.

- `~/.cache/rag3weaver-build/wt-docs-73`, au portable, détaché. Il sert aux docs de la session :
  `git fetch -q origin master && git switch -q --detach origin/master`, puis l'écriture, puis un commit par
  chemins (`git commit -- <chemins>`) et `git push origin HEAD:master`.

**Les copies de bases**, au portable sous `~/.cache/rag3weaver-build/1b-copie/` :
- `2026-11-14-depuis-avant-835d5298f/` : la copie de travail de `code-rag3db-depot`, jamais ouverte ;
- `migree-20261114-1641/` : **la preuve de 1b**, aussi à luciepc au même chemin (md5 identiques). Elle ne
  s'ouvre que par une copie reflink ;
- `verif-20261114/`, `avant-20261114/`, `requetes/` : les copies des vérifications, à effacer quand la
  bascule des bases réelles sera faite.

**Ce qui attend, et sur quoi** :
1. **2a′-3** (l'écriture des couches par les objets git, la position hors de l'identité) attend la mesure
   d'ED de (3a) et (3a′) sur la copie migrée, puis sa page. Le défaut, tant que la mesure n'a pas parlé,
   est le doublon au bord haut d'un trou, compté (page 03).
2. **La bascule des bases de Lucie** (versions, rang 0 et position, en une seule fenêtre) attend trois
   choses : la répétition sur copies, le ticket « une recherche à versions prend des Gio » fermé, et la
   durée de validation que Lucie dira. Ensuite viendra le retrait de l'ancien index (environ 5 Go par
   base).
3. **L'aide des tests** (ticket pour embarquement) : jusqu'à son correctif, `e2e_estimate` se joue à
   luciepc sans `RAG3WEAVER_EMBED_SERVICE`.

## Le ménage du 15 novembre, 1 h 2x

**Retirés par la garde**, administrations et fichiers non suivis archivés dans
`~/.cache/rag3weaver-build/archives/menage-arbre-principal-15nov/` :
- **au portable** : `wt-1b`, `wt-masque`, `wt-crochet`, `wt-suivies`, `arbre-crochet-push`,
  `arbre-refs-cited-push` et `arbre-suivies-push` ;
- **à luciepc** : `wt-1b-pile`, `wt-suivies` et `arbre-1b-push` ;
- **les targets** : `target-1b`, `target-masque`, `target-crochet`, `target-suivies` et
  `target-suivies-garde`, plus `target-1b-pile-luciepc` et `target-suivies-luciepc`.

**Retirées en local seulement**, dont la tête était poussée à l'identique ou déjà dans master : les
branches `arbre-principal/*` des étapes de la pile.

**Les branches distantes restent** (on n'en efface pas). Parmi elles, `arbre-principal/refs-cited-pile`
(dac27a8e2) est **inutile** : poussée par erreur pendant un rebase en conflit, elle porte l'ancienne tête du
lot `cited`. La bonne était `refs-cited-pile-2`, atterrie en 4eb274033. Les têtes de pile intermédiaires
(`*-pile`, `*-pile-2`, `*-pile-3`) sont toutes atterries.
