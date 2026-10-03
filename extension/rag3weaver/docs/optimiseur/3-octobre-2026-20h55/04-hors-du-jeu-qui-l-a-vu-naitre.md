# Les modèles de décision : hors du jeu qui l'a vu naître

**3 octobre 2026, session « optimiseur ».** Suite de
[l'exploration](03-chercher-la-bonne-maniere-de-s-en-servir.md), toujours sans
conclusion de produit. Trois questions : GLiNER tient-il dans la forme
« rangement » ? La forme à 15 sur 16 tient-elle sur un jeu qui n'est pas de ma
main ? Pourquoi les textes déjà rangés font-ils baisser le résultat ?

**La réponse aux deux premières est non. Ni la forme à 15 sur 16, ni le seuil
de 0,07 ne tiennent hors du jeu qui les a vus naître ; et GLiNER ne tient pas
non plus.** Le document 03 disait « rien ne garantit que la meilleure forme
d'ici le reste sur d'autres textes » : elle ne le reste pas. C'est le
résultat de cette passe, et il corrige deux phrases du 03 (le « 6 sur 6 » de
GLiNER et « le seuil tient »), qui reposaient sur six cas chacune.

JevK5-4B en Q8 sur la carte de l'autre poste, GLiNER sur CPU. Jeux, scripts et
sorties dans `banc/`.

## 1. Un jeu qui n'est pas de ma main : le journal des chantiers

**Comment il est bâti** (`banc/batir_journal.py`, sans aucune étiquette
posée à la main) : les entrées de `docs/journal-des-chantiers.md` sont
rangées sous leur titre de section. Cinq sections ont au moins quatre
entrées : « Branches ouvertes » (10), « Ce qui est dans git, et ce qui n'y
est pas » (4), « Reste à faire sur du travail déjà fusionné ou poussé » (8),
« Hors de ce dépôt » (4), « Bugs connus, non corrigés » (22).

- **Rejoindre** (32 cas) : une entrée, sa section parmi les options ; le
  « sujet neuf proposé » est l'en-tête de l'entrée.
- **Créer** (15 cas) : une entrée dont la section est retirée des options ;
  le sujet neuf proposé est le titre de cette section.

Formulations, critères et gabarits **tels quels**, sans rien régler.

| forme, sur le journal | rejoint le bon (32) | crée à tort | range ailleurs | neufs : crée (15) | fusionne à tort |
|---|---|---|---|---|---|
| l'alternative, deux textes par sujet, « créer » en dernier — **le 15 sur 16 du document 03** | **13** | 16 | 3 | 12 | 3 |
| la même, « créer » en premier | 17 | 8 | 7 | 8 | 7 |
| l'alternative, noms seuls | 5 | 26 | 1 | 13 | 2 |
| « quel sujet va à ce texte », noms seuls | 11 | 12 | 9 | 9 | 6 |
| « quel sujet va à ce texte », deux textes par sujet | 1 | 31 | 0 | 10 | 5 |

**Aucune forme ne tient.** La meilleure rejoint une entrée sur deux, et celle
qui rejoint le plus fusionne à tort près d'un texte neuf sur deux.

Deux choses distinguent ce jeu du mien, et je ne sais pas laquelle pèse :

- **Les sujets sont des états, pas des thèmes.** « Bugs connus, non
  corrigés » et « Reste à faire » ne se distinguent pas par ce dont parle le
  texte, mais par où en est le travail. C'est un rangement plus dur, et
  peut-être pas celui que le nœud aura à faire.
- **Le sujet neuf proposé est l'en-tête de l'entrée**, donc ses propres
  mots : « L'index vectoriel peut laisser une ligne injoignable dès sa
  construction » contre « Bugs connus, non corrigés ». Le neuf gagne.

## 2. Le contrôle à 48 paires du banc de la mémoire

La session de la mémoire longue a porté son jeu de contrôle de 6 à 48 paires
(master `5d1a3189b`) : six faits, six redites dites autrement, deux
contradictions ; la vérité est donnée par le scénario, personne ne l'a
étiquetée. Les listes sont recopiées mot pour mot de son script
(`banc/banc4.py`) ; critère, options et seuil du document 02, sans réglage.

| | sur mes 20 paires (document 02) | sur les 48 paires du banc |
|---|---|---|
| AUC de P(même) | 0,97 | **0,82** |
| vraies « même » au-dessus du seuil 0,07 | 4 sur 5 | 4 sur 6 |
| fusions à tort au-dessus du seuil | 0 | **6 sur 40**, aucune sur les 2 contradictions |
| le verdict à quatre choix dit « même » pour une redite | 3 fois sur 5 | **0 fois sur 6** (« complète » 3, « neuve » 2, « contredit » 1) |
| une paire sans rapport est dite « neuve » | 4 sur 5 | 26 sur 40 |

**Le seuil ne tient pas.** Le document 03 disait qu'il tenait, sur les six
premières paires : c'était six paires. Les redites du banc sont des
paraphrases lointaines de titres très courts (« dire qu'on prend le
compilateur » pour « annoncer cargo avant de le prendre ») ; mes paires
étaient des phrases complètes. P(même) y va de 0,03 à 0,31 pour les vraies et
monte à 0,13 pour une paire sans rapport : les deux distributions se
recouvrent.

Ce qui tient quand même : les deux contradictions reçoivent un P(même) très
bas (0,03 et 0,05), donc aucune n'est prise pour une redite.

## 3. GLiNER dans la forme « rangement »

Mêmes 24 textes et dix sujets que JevK5, ligne à ligne.

| forme | GLiNER : rejoint le bon (16) | crée à tort | range ailleurs | neufs : crée (8) | fusionne à tort | latence CPU | JevK5, forme la plus proche : rejoint le bon |
|---|---|---|---|---|---|---|---|
| formulation 1, noms, neuf en dernier | 2 | 9 | 5 | 8 | 0 | 68 ms | 12 |
| noms, neuf en premier | 2 | 10 | 4 | 8 | 0 | 54 ms | — |
| noms et descriptions | 4 | 7 | 5 | 7 | 1 | 91 ms | 13 |
| descriptions et deux textes | 5 | 3 | 8 | 3 | 5 | 232 ms | 8 |
| « aucun de ces sujets » au lieu du neuf nommé | 7 | 3 | 6 | 6 | 2 | 55 ms | 12 |
| dix sujets | 2 | 11 | 3 | 6 | 2 | 68 ms | 10 |
| formulation 2 (« join … » / « create … »), créer en dernier | 5 | 6 | 5 | 6 | 2 | 110 ms | 15 |
| formulation 2, créer en premier | 7 | 3 | 6 | 4 | 4 | 113 ms | 14 |

Sur le journal : 0 sur 32, il crée à chaque fois.

**GLiNER ne tient pas.** Le « 6 sur 6 » du document 03 venait de six cas
faciles, où le texte était presque le nom d'un sujet. Dès que le sujet neuf
proposé reprend les mots du texte, il le choisit ; et quand il ne crée pas, il
range ailleurs une fois sur deux. Il n'a pas de critère : la seconde
formulation ne peut lui être dite que par les mots des étiquettes.

## 4. Pourquoi les textes déjà rangés font-ils baisser le résultat ?

**Un cas de près.** Le texte « Dans un clone neuf, poser l'adresse
personnelle en local avant le premier commit », quatre sujets existants dont
« identité git », et le sujet neuf proposé « adresse des commits ».

| | « identité git » (lettre A) | le neuf (lettre E) | jetons |
|---|---|---|---|
| noms et descriptions | **0,81** | 0,07 | 241 |
| + deux textes déjà rangés par sujet | 0,30 | **0,69** | 422 |

L'option du bon sujet, dans le second cas, se lit : « identité git — Quelle
identité signe les commits et sous quel compte on pousse. Texts already filed
under it: «L'identité git globale du poste est l'adresse professionnelle :
vérifier user.email dans tout clone neuf.» ; «Pas de trailer d'attribution
dans les messages de commit.» ». Elle parle bien du même sujet, et même du
clone neuf. Le modèle bascule pourtant vers « adresse des commits ».

**Les variantes contrôlées**, sur les 16 textes à ranger (forme « quel sujet
va à ce texte », le neuf en dernier) :

| ce qu'on donne aux sujets existants | rejoint le bon | probabilité moyenne donnée au neuf | jetons |
|---|---|---|---|
| nom et description | 13 | 0,20 | 247 |
| + deux textes déjà rangés | 8 | 0,48 | 431 |
| + une phrase neutre de même longueur, sans contenu | 10 | 0,33 | 339 |
| + les textes pour le bon sujet seulement | 9 | 0,40 | 278 |
| + les textes pour les autres sujets seulement | 10 | 0,40 | 400 |
| + les deux textes les plus proches du texte à ranger | 9 | 0,43 | 424 |
| + les mêmes textes, annoncés comme « For example, among many other texts » | 11 | 0,22 | 435 |
| nom et description, le neuf allongé d'une phrase | 10 | 0,30 | 271 |
| deux textes, le neuf allongé d'une phrase | 7 | 0,49 | 455 |
| deux textes, le neuf au milieu de la liste | 6 | 0,58 | 431 |

Ce que cela dit, hypothèse par hypothèse :

- **La longueur, en partie.** Une phrase neutre sans contenu fait déjà perdre
  trois textes (13 → 10). Tout ce qui allonge les options profite au neuf.
- **Des extraits mal choisis : non.** Prendre les textes les plus proches du
  texte à ranger n'aide pas (9).
- **La lettre du neuf qui s'éloigne : non.** Le rapprocher (au milieu) fait
  pire (6).
- **La manière de les annoncer : oui, nettement.** « Texts already filed
  under it » se lit comme une définition du sujet par ses textes : le texte
  à ranger n'est aucun d'eux, donc il va ailleurs. Dire que ce ne sont que
  des exemples parmi d'autres ramène la probabilité du neuf de 0,48 à 0,22
  et rend trois textes.
- **Et un effet que la longueur n'explique pas** : donner des textes au seul
  bon sujet fait baisser aussi (9). Plus un sujet est décrit précisément,
  plus il paraît étroit.

Je n'en tire pas de règle : ces écarts sont de un à cinq textes sur seize.

## Ce que cette passe change à ce que disaient les documents 02 et 03

| ce qui était écrit | ce que cette passe en dit |
|---|---|
| 02 : JevK5 sépare « même » du reste, AUC 0,97, seuil 0,07 sans fausse fusion | vrai sur mes 20 paires ; sur les 48 du banc, AUC 0,82 et 6 fausses fusions sur 40 |
| 03 : « le seuil tient », sur six paires | il ne tient pas sur 48 |
| 03 : une forme rejoint 15 fois sur 16 sans fusion à tort | sur le journal, la même forme rejoint 13 fois sur 32 |
| 03 : GLiNER range sans faute un texte parmi dix sujets (6 sur 6) | 2 à 7 sur 16 dès qu'un sujet neuf est proposé |
| 03 : la façon de poser la question compte plus que le modèle | **confirmé, et plus fort** : elle déplace le résultat de 1 à 17 sur 32 |
| 02 : aucune contradiction prise pour une redite | confirmé sur les deux contradictions du banc |

## Où cela laisse la question

- **Rien de ce qui a été mesuré ne tient sur trois jeux à la fois.** Chaque
  forme a son jeu où elle marche.
- **Le sujet neuf proposé est le point faible commun** : quand son nom
  reprend les mots du texte, JevK5 comme GLiNER le choisissent. Or c'est ce
  qu'un agent proposera naturellement.
- **Le jeu du journal n'est peut-être pas le bon juge** : ses sujets sont des
  états d'avancement. Un jeu de vrais sujets thématiques, rangés par
  quelqu'un d'autre que moi, manque toujours.

## Ce qui reste à essayer

- Ne pas montrer le nom du sujet neuf : « aucun de ces sujets » fait mieux
  que le neuf nommé, pour GLiNER (7 contre 2) comme pour JevK5 à contexte
  égal (12 contre 8).
- Poser la question sans le neuf, et lire la création dans la faiblesse du
  meilleur score, avec un seuil par critère : cela retire le biais, au prix
  d'un seuil qu'on vient de voir fragile.
- Les 48 paires du banc avec les mémoires complètes (le fait et son
  pourquoi) plutôt que les titres seuls.
- Le jeu thématique qui manque.
