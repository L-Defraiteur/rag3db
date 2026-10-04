# Brouillon de vision — des curseurs ouverts, et le contexte comme une vue

4 octobre 2026. **Brouillon** : rien n'est décidé ni codé. Deux idées de
Lucie, la seconde plus lointaine que la première :

> « Et si les agents avaient des curseurs ouverts sur des fichiers, comme un
> vrai dev, qu'ils peuvent fermer quand ils veulent, et tant qu'ils ne le font
> pas ils voient le curseur actif et les relations dans leur contexte ? »

> « Peut-être que le contexte serait vraiment à manager totalement
> différemment de ce qui se fait actuellement. Il faudrait expérimenter, mais
> peut-être qu'un jour ce ne serait plus un simple fil d'historique d'outils,
> mais une vue optimisée de contexte, en permanence. Il faudrait d'abord
> faire des essais intermédiaires qui ne vont pas directement là-dessus. »

Ce document complète les visions du produit code
(`../3-octobre-2026-20h37/01`), l'exploration des relations
(`../3-octobre-2026-20h16/01`) et la vision des mémoires
(`../4-octobre-2026-00h09/01` et `02`).

## 1. Le trou que cela comble

Un agent qui lit un fichier en garde une **copie** dans son contexte. Cette
copie vieillit sans le dire : un autre agent édite, lui-même édite, la
synchronisation passe, et il raisonne sur un texte qui n'existe plus. Il
relit par précaution, ou il édite à l'aveugle et l'édition échoue.

Un développeur, lui, a des onglets ouverts : ce qu'il regarde est vivant, et
il sait toujours sur quoi il est.

## 2. Le curseur

Un **curseur** est une adresse tenue ouverte par l'agent : un scope (une
fonction, un type), une plage, un fichier. Tant qu'il est ouvert, le harnais
en tient l'état et le montre.

| Ce qu'un curseur montre | D'où ça vient | État |
|---|---|---|
| où il est : chemin, lignes, signature du scope | le catalogue | livré |
| sa fraîcheur : inchangé, ou « a changé depuis l'ouverture », et par qui | l'ingestion émet ce qu'elle change | livré côté événement |
| ses relations : appelants, dépendants, tests — quelques lignes, comptées | `usages`, `impact`, section « Avant d'éditer » | livré, lié à un appel d'outil |
| les fiches de mémoire qui parlent de lui | les liens de la vision des mémoires | proposé |
| qui d'autre a un curseur dessus | un registre des curseurs par session | à faire |

Ce qu'un curseur ne montre **pas** : le corps du fichier. Le corps se lit par
un outil ; le curseur dit s'il faut le relire.

**Les gestes** : ouvrir, déplacer, fermer, lister. Rien d'autre.

**C'est une adresse de la vision des mémoires.** Un curseur est une
référence (`row:Scope/…`, une plage, `path:…`) qui vit le temps de la
session ; une fiche de mémoire est la même référence, durable. À la
fermeture, le harnais peut proposer d'en garder une note — jamais l'imposer.

## 3. Ce que cela donne

- **Une vue vivante au lieu d'une copie morte** : moins de relectures de
  précaution, moins d'éditions sur un contenu périmé.
- **Les relations sans les demander**, là où l'agent travaille, et seulement
  là.
- **Un état qui survit à la compression du contexte** : les curseurs sont
  tenus par le harnais et rendus de nouveau, pas résumés.
- **Un signal pour l'archiviste** : les curseurs ouverts disent sur quoi
  l'agent travaille sans qu'il le déclare — le fil de travail, et les fiches à
  rappeler.
- **La présence entre agents** : un curseur visible des autres est un verrou
  d'intention, qui annonce sans bloquer.

## 4. Les points durs

- **Le coût en contexte.** Le souci permanent : ne pas injecter sans cesse.
  Un bloc à budget fixe, rendu en fin de prompt (le préfixe reste en cache),
  avec l'en-tête et quelques lignes de relations par curseur ; ne rendre de
  nouveau que ce qui a changé.
- **L'agent oublie de fermer.** Un plafond, le coût affiché (« 4 curseurs,
  ~700 jetons »), et le plus ancien fermé d'office en le disant.
- **Le grain.** Le scope plutôt que le fichier : c'est là que sont les
  relations. Un fichier sans scope (un md, une configuration) reste un
  curseur de fichier.
- **Le bruit des relations.** Un curseur montre en permanence ce que la
  section « Liens » montre une fois : les fausses arêtes (variables locales
  Rust, conteneur `mod tests`) y pèseraient plus lourd. À corriger d'abord.
- **Hors de notre chat.** Dans un harnais qui ne laisse qu'ajouter du
  contexte (Claude Code, `../4-octobre-2026-00h09/02`), pas de bloc épinglé :
  on n'aurait qu'une approximation, une injection quand un curseur change.

## 5. L'idée plus lointaine : le contexte comme une vue

Aujourd'hui, le contexte d'un agent est un **journal** : tout ce qui a été
dit et fait, dans l'ordre, jusqu'à ce qu'il déborde et qu'on le résume. Ce
que l'agent sait de l'état du monde est éparpillé dans ce journal, en copies
de dates différentes.

L'autre forme : le contexte est une **vue**, recalculée à chaque tour à
partir d'un état tenu hors du modèle —

| Zone de la vue | Ce qu'elle porte | Ce qui la tient |
|---|---|---|
| la tâche | le but, ce qui reste, ce qui est décidé | le fil de travail (mémoire) |
| l'espace de travail | les curseurs ouverts, leur fraîcheur, leurs relations | le registre des curseurs |
| ce qui vient de se passer | les derniers tours, tels quels | le journal, sur une fenêtre courte |
| ce qui a été appris | des fiches, rappelées par ce qu'on touche | l'archiviste |
| l'historique | accessible par un outil, pas présent | le journal complet |

Le journal ne disparaît pas : il devient une source qu'on interroge, pas le
contexte lui-même. Les curseurs sont la première zone de cette vue ;
l'archiviste (L1, L2, L3) en est une autre. Les deux visions se rejoignent
ici.

Ce qu'on ne sait pas, et que seuls des essais diront :

- si un modèle travaille aussi bien sur une vue recalculée que sur son propre
  fil — il est entraîné sur des fils ;
- ce que coûte une vue qui change à chaque tour (le cache de préfixe est
  perdu sur ce qui bouge) ;
- ce qu'on perd en ôtant un résultat d'outil que l'agent croyait avoir sous
  les yeux.

## 6. Les essais intermédiaires

Du plus petit au plus engageant ; chacun se juge seul, et aucun n'oblige au
suivant.

| # | Essai | Ce qu'il apprend | Ce qu'il demande |
|---|---|---|---|
| 1 | **La fraîcheur seule** : après un outil, dire « ce que tu as lu de X a changé depuis » | si les copies périmées sont un vrai coût | le crochet après outil (livré) et un registre de ce qui a été lu |
| 2 | **Les curseurs** : `open` / `close`, un bloc en fin de prompt, relations et fraîcheur | si un état épinglé évite relectures et éditions ratées | un registre par session, le rendu, un plafond |
| 3 | **Un résultat d'outil périmé se replie** : la lecture d'un fichier rouvert depuis est remplacée par une ligne (« lu au tour 12, remplacé au tour 20 ») | si l'on peut ôter du fil sans perdre l'agent | la maîtrise de l'assemblage du prompt (notre chat) |
| 4 | **La zone « tâche »** : but et reste à faire tenus hors du fil, rendus à chaque tour | si l'agent tient mieux un travail long | le gabarit *fil de travail* de la mémoire |
| 5 | **La vue** : fenêtre courte de tours + zones, le reste par outil | si la vue remplace le fil | tout ce qui précède |

**Comment juger** : des passes d'agent réelles (Gemini, comme pour le produit
code), sur les mêmes tâches avec et sans. Compter les relectures du même
fichier, les éditions ratées sur un contenu périmé, les appels à `usages` et
`impact`, les jetons par tâche, et la réussite. Une tâche longue au moins,
qui traverse une compression : c'est là que la différence doit se voir.

## 7. Ce qui attend un choix de Lucie

1. Lancer l'essai 1 (petit, sans risque) dès qu'une session est libre, ou
   attendre que les fausses arêtes soient corrigées.
2. Le grain du curseur : scope d'abord (recommandé), ou fichier d'abord.
3. Les curseurs visibles entre agents dès le début, ou plus tard avec les
   écritures parallèles.
