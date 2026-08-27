# 05 — La réputation des abstractions : ce que l'usage prouve, et ce qu'il ne prouve pas

Source : `inspi-chatgpt.md`, apporté par Lucie le 27 août.

Ce n'est pas une idée neuve dans la maison — le
[doc 49](../23-aout-2026-20h33/49-vision-le-catalogue-comme-graphe-outils-tags-memoire.md)
a déjà `GraphTool` / `GraphVersion` / `Invocation` et la règle **on promeut sur
preuve, pas à l'opinion** ([doc 51](../23-aout-2026-20h33/51-vision-le-chaos-controle.md)).
Mais l'échange apporte quatre choses que le doc 49 n'a pas, et il contient deux
pièges qu'il faut nommer avant qu'ils entrent dans le code.

## 1. Ce que ça ajoute, et qui vaut d'être pris

### 1.1 Séparer popularité et confiance

Le critère de promotion du doc 49 est un seul chiffre :

```cypher
WHERE n >= 5 AND echecs = 0
```

Et **c'est exactement le piège de l'inertie** : une fois promu, un graphe est
trouvé en premier, donc utilisé davantage, donc il reste promu. Le premier
arrivé gagne, et rien ne peut plus le déloger — « ce magnifique mécanisme grâce
auquel l'humanité fait encore tourner des macros Excel de 2004 ».

Quatre lectures, pas une : **pertinence**, **confiance**, **usage**, **récence**.
Le nombre d'usages n'est pas une preuve de qualité, c'est une preuve de
visibilité.

### 1.2 La politique de sélection dépend du contexte

Un point que le doc 49 n'a pas du tout : **le critère de choix n'est pas le
même selon ce qu'on fait**. Action dangereuse → la confiance prime. Prototypage
→ pertinence et récence. Backend métier → validation humaine exigée.

Et on a déjà la machine pour l'exprimer : `WorkDomain` et `Selector`
([doc 05 du 26](../26-aout-2026-20h29/05-origine-cellule-domaine.md)) compilent
un contexte en `FilterCondition`. Une politique de sélection est un domaine de
travail de plus, pas un mécanisme nouveau.

### 1.3 Les avis sont des données cherchables

Le doc 49 stocke des `Invocation` : il sait **combien** et **si ça a marché**.
Il ne sait pas **pourquoi**. Pouvoir demander « pourquoi ce graphe a mauvaise
réputation ? » et recevoir *deux échecs sur schéma nullable, un rollback après
migration, « ne marche que si customer_id existe »* — ça, c'est neuf, et c'est
la moitié la plus utile.

**Et ça ne demande aucun vocabulaire nouveau.** Un avis d'agent sur un graphe,
c'est un `Message` dans une `Conversation`, écrit par un `Participant`, daté,
cherchable, avec le fuseau qui connaît ses règles — tout ça a été livré ce
matin. Il manque une relation, `ABOUT`, vers la `GraphVersion` jugée. C'est
tout.

### 1.4 La diversité des contextes est un signal à part

Cent quatre-vingts succès dans un seul projet valent moins que vingt succès
dans six projets : le second a montré qu'il **généralise**, le premier a montré
qu'il tourne. Le doc 49 compte ; il ne diversifie pas.

## 2. Les deux pièges

### 2.1 `confidence_score` stocké se contredit lui-même

La fiche proposée finit par `confidence_score`, et le paragraphe suivant dit
qu'il faut « bien séparer popularité et confiance ». **Les deux ne peuvent pas
être vrais en même temps** : agréger, c'est reperdre la séparation qu'on vient
de faire, et un agrégat rangé quelque part est un verdict qui survit à ses
raisons.

La règle qu'on applique depuis trois jours vaut ici mot pour mot :

> Une projection d'un état ne ment pas ; un verdict archivé, si.

C'est ce qui a été fait pour le bloc d'attentes ([doc 04](04-la-session-tient-l-invite.md)),
et pour la même raison. Donc : **les quatre lectures sont stockées, l'agrégat
ne l'est jamais** — il est calculé au moment de choisir, avec les poids de la
politique en cours. Deux politiques différentes classent différemment, et
c'est le comportement voulu.

### 2.2 `rating: 5/5` est de l'opinion, précisément ce qu'on refuse

Un agent qui note son propre run à 5/5 s'auto-évalue, et une auto-évaluation
est le signal le plus faible dont on dispose — c'est même la définition de ce
que le doc 51 interdit : *qu'on promeuve à l'opinion*.

Ce qui est fort, dans la liste, c'est **tout ce que personne ne tape** :

| Signal | D'où il vient | Pourquoi il est fort |
|---|---|---|
| le run a fini | `RunFinished { ok }` | personne ne le déclare |
| il y a eu un rollback | l'`undo` du catalogue | coûteux, donc jamais accidentel |
| le résultat a servi en aval | la trace du tour suivant | observé, pas rapporté |
| le même appel a été refait autrement juste après | la trace, encore | **un aveu d'inutilité que personne n'écrira jamais** |

La dernière ligne est celle qui vaut le plus cher et qu'on a déjà sous la main.
Un agent qui rappelle le même outil avec d'autres arguments dans la foulée dit,
sans le dire, que le premier résultat ne lui a rien apporté.

Donc : garder les `notes` — elles portent le *pourquoi*, elles sont
cherchables, elles sont la §1.3. **Jeter l'étoile**, ou à tout le moins lui
interdire d'entrer dans la confiance.

## 3. Ce que l'échange ne résout pas, et qu'il faut ajouter

### 3.1 Séparer les scores ne suffit pas contre l'inertie

Nommer le problème n'est pas le résoudre. Si la sélection prend toujours le
mieux noté, **rien d'autre n'accumule jamais de preuve**, et la séparation des
scores ne change rien au résultat : le premier arrivé gagne toujours, juste
avec quatre colonnes.

Il faut une **part d'exploration explicite** : de temps en temps, essayer ce
qui n'a pas encore fait ses preuves. Et ça tombe bien, la boucle du doc 49 §6
— *chercher, composer si besoin, garder ce qui a marché* — a déjà la place :
elle compose quand rien ne convient. Il suffit qu'elle essaie aussi, parfois,
quand quelque chose conviendrait.

Le coût est borné et connu : un run de graphe raté est bon marché, et il est
tracé — donc l'exploration **produit** de la preuve au lieu d'en consommer.

### 3.2 Un échec doit être attribué

« Deux échecs sur schéma nullable » — un graphe qui échoue parce que le schéma
a changé n'est pas un mauvais graphe. Sans attribution, la réputation punit le
messager.

C'est très exactement la famille de défauts qu'on a corrigée deux fois cette
semaine : le masque HNSW et le filtre vectoriel sur un champ de parent avaient
tous deux **un symptôme attribué à la mauvaise couche**. On a de quoi faire
mieux : `NodeRun` dit quel nœud a échoué, et `Invocation` pointe une version
exacte.

### 3.3 La réputation appartient à une cellule

Un graphe de confiance chez l'un ne l'est pas chez l'autre. Le doc 49 filtre
déjà `_org` / `_project` dans ses requêtes, et **ça doit le rester** : une
réputation globale est une fuite inter-locataires sous forme de réputation.
Ce qui peut se partager à travers une organisation est déjà cadré — c'est
`ExportableStats` et la question du partage entre projets, non tranchée
(doc 05 du 26, §10.4).

### 3.4 Dédupliquer, c'est fusionner, et une fusion ne se défait pas

`auth`, `authentication`, `login-system`, `user-auth` : le cross-encoder dit
qu'ils se ressemblent. Il ne dit pas que ce sont **le même concept** — ce
peuvent être deux couches d'un même système, et les confondre coûte plus cher
que de les laisser séparés.

La règle qu'on s'est donnée pour l'identité des fichiers vaut ici :
**proposer, ne pas fusionner.** Le reranker produit un candidat ; quelque chose
confirme — un humain, un accord de plusieurs modèles, ou l'usage. Le doc 49 §9
nomme déjà la dérive sémantique d'un tag et l'opération de fusion ; c'est le
même sujet, et la même prudence.

## 4. La forme que je proposerais

Rien de nouveau dans le vocabulaire — c'est la contrainte du
[doc 13 §8](../25-aout-2026-18h58/13-la-session-comme-graphe.md), et elle est
saine.

- **Stocké** : les faits. `Invocation` (existe), `RunFinished`, les rollbacks,
  la cellule, le domaine de travail, la version exacte.
- **Stocké** : les avis, comme `Message` dans une `Conversation`, reliés
  `ABOUT` à une `GraphVersion`. Le texte, pas la note.
- **Jamais stocké** : le score. Quatre lectures dérivées à la demande —
  pertinence, confiance, usage, récence — et un agrégat calculé **au moment de
  choisir**, avec les poids du domaine en cours.
- **Explicite** : la part d'exploration, et le fait qu'un échec soit attribué.

Et la question ouverte n°1 du doc 49 — *qui a le droit de promouvoir ?* —
devient plus facile à trancher quand on a séparé les lectures : on peut
répondre **différemment selon le domaine**, ce qui était impossible avec un
seul chiffre.

## 5. Une réserve, pour finir

L'excerpt la formule lui-même, en plaisantant : *aucun risque de créer une
bureaucratie algorithmique avec des étoiles et des réputations*. La plaisanterie
est le vrai risque.

Un système de réputation ajoute une couche qu'il faut ensuite déboguer, et dont
les erreurs sont **silencieuses** : un bon graphe mal noté ne proteste pas. La
protection est celle qu'on s'applique depuis le début — que chaque lecture soit
**dérivée de faits qu'on peut aller relire**, et qu'aucun verdict ne survive à
ses raisons. Tant que « pourquoi ce graphe a mauvaise réputation » a une réponse
qui cite des runs datés, le mécanisme se débogue. Le jour où la réponse est un
nombre, il ne se débogue plus.
