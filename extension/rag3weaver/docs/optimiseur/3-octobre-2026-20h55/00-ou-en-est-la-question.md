# Un modèle de décision pour la mémoire longue : où en est la question

**3 octobre 2026, session « optimiseur ».** Synthèse des cinq documents de ce
dossier ([01](01-les-modeles-de-decision-verifies-a-la-source.md) la
vérification à la source, [02](02-les-modeles-de-decision-mesures.md) la
première mesure, [03](03-chercher-la-bonne-maniere-de-s-en-servir.md) la
formulation, [04](04-hors-du-jeu-qui-l-a-vu-naitre.md) les jeux qui ne sont
pas de notre main, [05](05-trois-pistes-sur-trois-jeux.md) le dernier tour).
Elle ne recommande rien pour le produit : elle dit ce qui est établi, ce qui
ne l'est pas, et ce qu'on ne sait pas.

La question de départ : un nœud qui reçoit un critère en clair, une chose et
des candidats, et rend un choix dans une liste fermée avec une probabilité.
Premier usage : ce sujet neuf est-il un sujet existant, ou faut-il le créer ;
cette mémoire neuve redit-elle, complète-t-elle ou contredit-elle une mémoire
existante.

## Ce qui tient sur tous les jeux

- **Les modèles existent et sont utilisables sans rien entraîner.** Six
  dépôts vérifiés, tous sous Apache-2.0, tous avec un point de contrôle prêt.
  Un seul des encodeurs se déclare multilingue (Laya) ; JevK5 se dit anglais
  et lit pourtant le français.
- **La liste fermée et le déterminisme sont acquis.** JevK5 met 99,9 % de sa
  probabilité sur les lettres offertes ; deux passes rendent les mêmes
  chiffres. Sur carte, une décision prend 230 ms ; sur CPU, deux secondes.
- **Ranger un texte parmi des sujets existants est un problème d'ordre, et
  nous l'avons déjà.** Le cosinus de granite, sur le texte complet des sujets
  (nom, description, deux textes rangés), met le bon sujet en premier 14 fois
  sur 16, 19 sur 32 et 22 sur 30 sur trois jeux, aussi bien ou mieux que
  JevK5.
- **Un ordre n'est pas un verdict.** Le cosinus note une contradiction aussi
  haut qu'une redite (0,85 et 0,83). JevK5 ne prend aucune contradiction pour
  une redite, sur les deux jeux qui en contiennent.
- **La façon de poser la question compte plus que le modèle.** Sur les mêmes
  cas, le résultat va de 2 à 15 sur 16, et de 1 à 17 sur 32, selon la forme.
  L'ordre des options, la longueur des options, la manière d'annoncer un
  exemple déplacent des verdicts.
- **Les petits encodeurs (Laya, GLiNER) ne jugent pas**, dans notre usage
  comme dans celui de leurs auteurs.

## Ce qui ne tient sur aucun

- **Décider qu'il faut créer un sujet.** Trois voies essayées, sur trois
  jeux. Le sujet neuf nommé avec les mots du texte est choisi à tort ; rendu
  anonyme (« aucun de ces sujets »), il n'est presque jamais choisi ; lu dans
  la faiblesse du meilleur score, il ne se sépare pas (AUC de 0,53 à 0,80).
- **Un seuil qui se transporte.** Le seuil sans fausse fusion vaut 0,50 pour
  un critère et 0,07 pour un autre ; réglé sur vingt paires, il laisse passer
  6 fusions à tort sur 40 sur un jeu qu'il n'a pas vu.
- **« Variante ».** Huit de mes quinze étiquettes sont contredites avec
  constance sous seize formulations.
- **Une forme qui gagne partout.** Celle qui rejoint 15 fois sur 16 sur mon
  jeu rejoint 13 fois sur 32 sur le journal.

## Ce qu'on ne sait pas

- **Ce que cela donne sur de vrais sujets de la base.** Mes jeux sont écrits
  par moi ; le journal range par état d'avancement ; le jeu public est d'un
  autre domaine. Le jeu qui manque est thématique, du nôtre, et rangé par
  quelqu'un d'autre.
- **Si le verdict sur les mémoires tient.** 85 % sur mes vingt paires ; sur
  les 48 du banc, cinq redites sur six reconnues quand on montre le pourquoi,
  trois sur six sinon. Deux contradictions seulement ont été jouées hors de
  ma main.
- **Si la création doit être décidée par un modèle.** La proposition de la
  mémoire longue prévoit le défaut « demander toujours » et une création
  réparable ; rien de mesuré ici ne permet de faire mieux que ce défaut.
- **Ce que vaudrait un modèle plus gros** (JevK5-9B, Kev) ou la même mesure
  avec un critère écrit par quelqu'un qui connaît mieux ces modèles.
- **L'accord avec un jugement humain** : aucune étiquette de ces mesures n'a
  été relue.

## Les chiffres qui portent cette synthèse

| | résultat | document |
|---|---|---|
| verdict à quatre choix sur les mémoires, JevK5, mes 20 paires | 85 % | 02 |
| même verdict sur les 48 paires du banc, titres seuls / avec le pourquoi | 3 / 5 redites reconnues sur 6 | 04, 05 |
| ranger parmi les existants, cosinus sur textes complets, trois jeux | 14/16, 19/32, 22/30 | 05 |
| même chose, JevK5 | 12/16, 15/32, 20/30 | 05 |
| textes neufs reconnus avec « aucun de ces sujets », trois jeux | 3/8, 2/15, 3/12 | 05 |
| AUC de la création lue dans le meilleur score, le meilleur modèle de chaque jeu | 0,79 ; 0,74 ; 0,80 | 05 |
| seuil 0,07 sur 48 paires non vues | 4 redites sur 6, 6 fausses fusions sur 40 | 04 |
| latence d'une décision JevK5, carte / CPU | 230 ms / 1 955 ms | 03 |

Jeux, scripts et sorties brutes : `banc/`.
