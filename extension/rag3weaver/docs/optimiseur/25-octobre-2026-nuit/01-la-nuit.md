# La nuit du 25 octobre 2026 (G)

Le plan donné par c7 à 22 h 35, sur le mot de Lucie : « faut faire ce qu'il faut pour fix, même si
chantier ou réindex complet ». Le portable est en mode éco : les durées comptent 1,5 à 2 fois.

1. La pose des hôtes au hash du lot « requête entière » de la session recherche.
2. Le rebâti de la base de code de Lucie dans une base neuve (code-rag3db-neuf,
   code-rag3db-depot-neuf.rag3db), ouverte comme second backend de l'hôte, index complet.
3. Le relevé de LOCKS et des tables de liens sur la base neuve par hotfixs. À zéro écart, l'échange à
   une pose de nuit (l'ancienne en instantané locks-25-oct). Sinon, la base neuve devient le témoin
   du défaut moteur.
4. Le correctif de la porte de `poste` (ticket 2026-10-25-a-la-liberation-du-poste-…), témoin rouge
   d'abord.

## Le journal

- 22 h 23 — les deux hôtes sur 25d9bb4aa (le dense borné), lib 04063ee06. outline ajouté au
  manifeste de Lucie. Preuves vertes : search_code dense borné, et l'IDE (marge comprise).
- 22 h 35 — j'attends le hash de la session recherche pour l'étape 1.
- 22 h 56 — bâti à luciepc sur 159087582 (la requête entière, l'équipier, la montre qui dit sa cause),
  lib 04063ee06, binaire 2d01babe ; essai vert aux deux postes à 22 h 57.
- 23 h 00 — pose et réveil aux deux postes. Preuves : l'IDE vert ; « Vue d'ensemble » rend la page
  d'architecture en 4e (les dix premiers en bm25_exact). Un avertissement contradictoire (« rien de
  probant ») est rendu à la session recherche.
- 23 h 02 — étape 2 : l'index complet de la base neuve part dans un **hôte à part** (port 7992, dossier
  ~/.cache/rag3weaver-build/hote-neuf, copie figée du binaire 2d01babe, inactivité 10 h). Les poses de
  la nuit, qui redémarrent l'hôte de Lucie, l'auraient tué. 6 616 fichiers retenus. Journal :
  ~/.cache/rag3weaver/commandes/index/1792965744115-3263852.out.
- Les lots IDE d'everything declarative de la nuit seront posés ensemble, au hash de l'IDE en React.
