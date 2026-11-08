# Deux témoins d'e2e_lectures rouges sur master depuis le soir du 8 novembre

- **État** : fermé — rouge d'environnement, aucun commit fautif (8 novembre 2026, 21 h 40,
  everything declarative ; lecture de l'orchestration) : la saturation du service creux de luciepc
  par les passes concurrentes
- **Famille** : produit
- **Gravité** : test rouge sur master — il bloque toute passe qui joue e2e_lectures, ou apprend à
  passer outre
- **Atteignable en service** : non par un commit ; un service creux saturé ralentit le surlignage
  pour tout le monde, et le surlignage le dit déjà (« surlignage creux indisponible »)
- **Touche rag3weaver** : oui

## Ce que c'est

e2e_lectures, joué seul sur master **199f4d75c** à luciepc (8 novembre, vers 21 h 15, pression
E/S avg60 0,08, sous cbc06c3e1 + MOTEUR_ANCIEN), fait 22 verts et 2 rouges :

- `e2e_lectures::le_creux_se_surligne_par_le_chemin_de_l_hote` (e2e_lectures.rs:1350) — le
  surlignage remonte un avertissement au lieu d'aucun : « Une écriture de l'index est en cours : la
  réponse porte sur l'état validé, d'avant elle. La consigne de cohérence ne s'applique pas
  pendant cette écriture… ». Au premier passage, sous un bge-m3 saturé, c'était « surlignage creux
  indisponible (le service creux n'a pas répondu en 1000 ms) ».
- `e2e_lectures::un_outil_de_lecture_ne_prend_jamais_le_mutex_du_catalogue`
  (e2e_lectures.rs:411, appelé de :444) — un appel d'outil après l'index d'une base neuve répond
  `{"error":"connection error: …/code.rag3db is already open for writing in this process — 3
  référence(s) à l'instance précédente tenue(s) après 10s : un catalogue, un index plein texte ou
  une tâche de fond la retient encore","ok":false}`.

Les deux rouges sont les mêmes sur la pile de l'atelier (gabarits-vivants-3, rebasée sur
4f54e473a) : ils ne viennent pas d'elle. e2e_lectures était **24/24** dans la passe complète du
juge par pas, sur 9bae60786, à 18 h 44.

## Les candidats

Les commits de code arrivés sur master entre 9bae60786 et 4f54e473a :
9b6b535d3 et 2313398cc (l'identité du premier commit de git_init), 3d968fb98 (l'aperçu sur un
graphe de clic vide), e16cd5c56 et 4fd994dd8 (l'accueil en dépôts), 521ade9d8 et 8a81ed38a (la fin
du premier index), 6044f5ae2 et 9c20d6773 (l'avancement sur le flux), 4f54e473a (le juge par pas).
« 3 références à l'instance précédente tenues après 10 s » fait penser à un fil qui garde un `Arc`
du backend ou du catalogue après sa fermeture.

## Ce que la bissection a dit

`git bisect run` d'e2e_lectures seul entre 9bae60786 (bon, 24/24 à 18 h 44) et 4f54e473a, à
luciepc : 4fd994dd8, 2313398cc et feffea716 (un commit de docs seules, juste après 9bae60786) sont
tous mauvais, et seul `le_creux_se_surligne_par_le_chemin_de_l_hote` y rougit. Un commit de docs ne
casse pas un témoin : le rouge suit l'état du service, pas le code. Le témoin du mutex est vert à
chaque pas : son rouge du rejeu d'avant était la concurrence de deux e2e sur la même machine.

## La cause

Le service bge-m3 de luciepc (7879, le creux et le dense de ces témoins) était tenu à 99 % du
moteur de card2 depuis 21 h par le test de charge d'ingestion de la passe de recherche
(`e2e_charge_ingestion`, deux connexions, un flot continu) ; s'y ajoutaient la passe de la pile de
l'atelier et sa bissection, qui jouaient e2e_lectures en parallèle. Mesures de l'orchestration :
3,4 s par texte à 21 h 20, 24 ms à 21 h 36, le test de charge fini.

## La preuve

Carte libre, à 21 h 37 : e2e_lectures seul sur **9bae60786**, 24/24 en 16 s ; sur **f16628f8b**
(la pile de l'atelier sur master), 24/24 en 16 s.

## Ce qui reste

Rien à corriger dans le code. Deux leçons pour les passes : ne pas jouer en parallèle deux suites
qui chargent le même service d'embarquement (une passe et sa bissection, ou un test de charge), et
lire l'état des services (`fdinfo` de la carte, le temps par texte) avant de nommer un rouge de
master.
