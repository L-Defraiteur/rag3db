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
- 23 h 06 — la session mémoire a fini la migration de la fiche de Lucie (memoire-longue rouverte sur
  son nouveau manifeste, instantané migration-fiche-25-oct).
- 23 h 21 — l'index complet de la base neuve est fini, en 19 min : 6 508 fichiers, 68 585 scopes,
  214 438 relations, 116 573 vecteurs ; aucune ligne d'échec ; « plein texte vérifié : autant de
  documents que de lignes (4 entités) ». L'hôte à part est arrêté proprement à 23 h 21 m 42 s.
  Base neuve : 1 066 627 072 octets, plus 648 Mo de plein texte (la vivante : 1,2 Go et 707 Mo).
- 23 h 22 — étape 3 : la base neuve est donnée à hotfixs pour le relevé de LOCKS et des tables de liens.
- 23 h 28 — bâti sur 81fdc2f40 (l'IDE en React vues 1 et 2, le repli de la marge 400813249, le rendu
  entier c4472d61a), lib 04063ee06, binaire a4df1b50 ; essai vert aux deux postes.
- 23 h 31 — pose et réveil aux deux postes. Témoin de l'IDE vert ; /ide/ et la page du chat servent la
  page React ; le sha256 de app.js est celui du bundle aux deux postes.
- 23 h 40 — la page du chat est rendue en Chrome sans tête piloté par CDP, sur l'hôte vivant : en-tête « Chat de
  l'atelier · backends/code », « modèle : google/gemini-3.8-flash », conversation et champ de saisie, console
  sans erreur. Une remarque à everything declarative : le Markdown des réponses s'affiche brut.
- 23 h 46 — verdict de hotfixs sur la base neuve : 0 écart sur les 18 tables de liens (LOCKS : 506 relations
  dans chaque sens ; QUERIES 21 806 ; MENTIONS 396 818…). Échange annoncé pour 23 h 55 aux sessions.
- 23 h 55 — étape 3, l'échange. L'hôte s'arrête proprement (la vivante n'a plus de .wal). L'instantané
  locks-25-oct est vert (base 1 261 416 448 octets, fts 1 097 029 794, copies identiques), avec RETOUR.sh ;
  les fichiers d'origine sont déplacés dans instantanes/locks-25-oct/originaux/, rien n'est effacé. La base
  neuve prend le nom canonique (fichier, .fts, .extensions) ; le manifeste de Lucie est inchangé, et
  rag3db-neuf.json est renommé en .fait-locks-25-oct.
- 23 h 55 m 23 s — réveil vert : même binaire a4df1b50, lib 04063ee06, MainPID 3559518, jeton inchangé.
  35 fichiers en retard à l'ouverture, plus aucun à 23 h 58 ; fts_counts concorde (autant de documents que
  de lignes dans les quatre entités) ; le journal ne montre aucune erreur. La journée de montre de hotfixs
  part de là.
- Pour la pose du matin : f5edfdb6c et 45308bce9 (la première recherche après l'ouverture), 7c7a69578 (vue
  3), c10dabb71 (vue 4) ; à suivre, le Markdown du chat, les surlignages et « rien de probant ».
- 0 h 36 — bâti à luciepc sur 11d350001 (bb0f4d9c0 : les vues React 1 à 4 et le Markdown du chat ; la recherche :
  la poignée, « rien de probant », les surlignages), lib 04063ee06, binaire 8f4c923e ; essai vert aux deux postes
  à 0 h 38.
- 0 h 44 — pose et réveil aux deux postes (annoncés à 0 h 39). Preuves : le témoin l_ide_d_un_vrai_hote est
  vert en 9,6 s ; le chat est rendu par CDP sur l'hôte vivant, avec le Markdown (strong 2, listes 3, li 7,
  code 11, plus aucun astérisque), l'en-tête et le modèle, et la console est propre.
- La suite, d'après c7 : une pose en fin de file vers 2 h 30, sur son hash, puis le matin la bascule 43 par la
  procédure de l'arbre principal.

