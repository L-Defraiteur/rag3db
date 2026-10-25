# À la libération du poste, une mesure et des travaux lourds entrent ensemble

- **État** : ouvert, **une hypothèse lue dans le code, pas un défaut constaté** (corrigé le 25 octobre 2026
  à 23 h 25). Le cas de 22 h 26 n'en était pas une instance : le relevé s'était fié aux heures de fin
  des journaux. Les `.ninja_log` datent le départ des deux bâtis lourds à 22 h 27 m 06 s, la seconde où
  la mesure `temoin-1` (partie vers 22 h 26 m 20 s) a rendu le poste : la porte a tenu. Reste
  l'hypothèse ci-dessous (flock n'ordonne pas les réveils). Elle ne devient un défaut que si le témoin
  la reproduit ; sinon, le ticket se ferme avec la preuve de dizaines de rejeux. Pas prioritaire.
- **Gravité** (si l'hypothèse se vérifie) : réponse fausse en silence. Une mesure qui a couru sous charge rend un chiffre qui ne
  vaut rien, et rien ne le dit.
- **Atteignable en service** : oui, à luciepc, dès que plusieurs sessions attendent le poste
  ensemble. C'est l'usage ordinaire du soir.
- **Touche rag3weaver** : oui (`extension/rag3weaver/scripts/poste`, la porte).

## Ce que c'est

La règle de `poste` : une mesure est exclusive et prioritaire. Dès qu'une mesure attend, plus aucun
travail lourd n'entre ; un lourd qui attend ne part qu'après elle. Le relevé du 25 octobre à 22 h 26
(une mesure et deux bâtis lourds entrés ensemble) s'est révélé faux : voir l'État.

La porte aujourd'hui (scripts/poste, l. 363 à 380) :
- une mesure prend `porte.lock` en exclusif et le garde, puis `poste.lock` en exclusif ;
- un lourd prend `porte.lock` en exclusif, le rend aussitôt, puis prend `poste.lock` en partagé.

Rien n'ordonne donc les réveils à la libération. `flock` n'est pas une file : un lourd qui attendait
la porte peut la traverser avant que la mesure ne la prenne. Plusieurs lourds peuvent aussi partager
`poste.lock` alors que la mesure, qui tient déjà la porte, attend encore son exclusif. Le mécanisme
exact du 22 h 26 reste à reconstituer par le témoin.

## Le témoin (à écrire, rouge d'abord)

Avec un dossier de verrous à lui (`$V` d'essai) :
1. un lourd tenu ;
2. une mesure et un lourd en attente ;
3. libération du premier lourd.

Attendu : la mesure passe seule ; le lourd en attente ne démarre qu'après la fin de la mesure. Chaque
travail note son entrée et sa sortie, et le témoin juge que les intervalles ne se chevauchent pas.
Le cas se rejoue plusieurs fois, l'ordre des réveils n'étant pas fixe.

## Pour le fermer

Un correctif de la porte qui donne la priorité à une mesure qui attend : par exemple un jeton
« mesure en attente » posé avant toute attente de la mesure, et vérifié par un lourd après la porte,
au moment de prendre `poste.lock`. Le témoin doit être vert sur des dizaines de rejeux.
