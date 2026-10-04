# La stèle du moteur — où l'on s'arrête d'y regarder

4 octobre 2026. Décision de Lucie :

> « Cette liste exactement définit un point d'arrêt de version pour le moteur :
> une stèle, pour savoir continuer sans y regarder encore ad vitam æternam. »

La liste : **les verrous, le chargement en masse journalisé, les écritures
parallèles**, et les défauts que les bancs ont trouvés. Quand elle est
tenue, le moteur est déclaré « assez bon » : on pose la version, et on n'y
revient que sur un défaut **constaté** dans le produit — plus par
précaution, plus pour l'améliorer.

Pourquoi : « le moteur nous aveugle sur la complexité ; c'est lui qui nous
prend beaucoup de temps à régler ; quand on l'aura bien fait, ce sera plus
simple de faire les autres sujets. » Sans point d'arrêt écrit, il absorbe
l'effort sans fin.

## 1. Ce qui doit être vrai

| # | Condition | Ce que « fait » veut dire | La preuve |
|---|---|---|---|
| 1 | **Les verrous** | le gestionnaire de verrous (V1 : ressource générique, genres ligne et index), puis A3′, A4′, V2 de la note de conception | les témoins du banc de concurrence (§6 de la note des verrous) verts |
| 2 | **Le chargement en masse journalisé** | un `COPY` n'impose plus son point de reprise ; ses lignes survivent à un arrêt brutal par le journal | arrêt brutal au milieu d'un chargement, processus neuf, comptes justes ; le premier index n'y perd plus de temps |
| 3 | **Les écritures parallèles** | plusieurs écrivains sur une même base, sans perte ni blocage, le mode allumé hors du banc | le banc de concurrence entier, vert, mode allumé ; deux agents qui écrivent en même temps dans un test du produit |
| 4 | **Plus de défaut connu qui corrompt ou qui perd** | chaque ticket de `docs/tickets/` qui touche la mémoire, la durabilité ou la justesse d'un résultat est fermé | le ticket fermé avec son témoin au banc |

Ce que la stèle **n'exige pas** : les défauts de confort (un message, un cas
de syntaxe contourné, une lenteur sans perte), les correctifs d'amont non
reproduits, toute optimisation. Ils restent en tickets et n'empêchent pas de
poser la version.

## 2. Où l'on en est (4 octobre, 17 h)

- **1, les verrous** : conçus (`docs/3-octobre-2026-15h47/01-note-de-conception-les-verrous.md`),
  témoins écrits au banc ; V1 pas commencée.
- **2, le chargement journalisé** : la demi-page est écrite
  (`extension/rag3weaver/docs/3-octobre-2026-23h31/coeur-cpp/03-le-point-de-reprise-de-copy.md`) ;
  trois voies — alléger le point de reprise (en cours), une fenêtre de
  chargement initial, journaliser pour de bon (la voie que la stèle demande).
- **3, les écritures parallèles** : T0, A2, A5, A5 bis livrées ; le mode
  reste éteint hors du banc ; la suite attend les verrous.
- **4, les défauts** : la corruption de mémoire à la recréation d'un index a
  sa cause et son correctif (`1ea49837f`) ; une quarantaine de tickets
  ouverts, **pas encore triés** entre « bloque la stèle » et « confort ».

## 3. Ce qui reste à faire pour que la stèle soit utilisable

1. **Trier les tickets** : une ligne par ticket du moteur — bloque la stèle
   (mémoire, durabilité, résultat faux) ou non. À faire par la session du
   cœur C++, relu par le banc.
2. **L'ordre** : les verrous, puis les écritures parallèles, puis le
   chargement journalisé — ou le chargement d'abord, puisqu'il est sur le
   chemin des 90 s du premier index. À trancher.
3. **Poser la version** quand les quatre conditions sont tenues : une
   étiquette git, la date au journal des chantiers, et la règle « on n'y
   revient que sur un défaut constaté ».

## 4. Ce qui n'attend pas la stèle

Un agent seul sur un dépôt marche déjà : l'interface des fiches de contexte
et un premier utilisateur extérieur ne dépendent ni des verrous ni des
écritures parallèles.
