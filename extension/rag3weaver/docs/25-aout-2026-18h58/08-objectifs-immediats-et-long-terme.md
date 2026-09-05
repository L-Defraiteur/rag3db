# 08 — Objectifs : immédiats, moyen terme, long terme

25 août 2026, minuit. Ce qu'on attaque dans l'ordre, et pourquoi cet ordre.
La feuille de route complète est `../vision_roadmap_09_2026/06` ; ceci est
la pile de travail telle qu'elle se présente après ce soir.

## Immédiat — la prochaine session

1. ~~**Des `enum` dans les schémas d'outils.**~~ **Fait le 25 au soir** :
   directive `%% choices:` dans les fiches (`@targets`, `@relations`, ou
   une liste close), résolue contre le catalogue vivant quand la fiche part
   vers le modèle (`GraphToolBox::tool_defs`), et refus `bad_choice` à
   l'appel avec la liste des valeurs admises. Le détail de `relation`
   nomme aussi les extrémités (`DEFINED_IN (Scope→File)`). Puis, après
   minuit, généralisé : les `choices` vivent sur `ConfigParam`, déclarés
   par les nœuds, et une fiche en **hérite par câblage** de ses `$var`, à
   travers les niveaux d'imbrication ([10](10-parametres-de-config-entrees-du-graphe.md)).
   Reste à mesurer sur Gemini si `search_expand` devient prenable.
   L'intention d'origine : cibles (`Scope`, `File`,
   `Product`…) et relations (`CONSUMES`, `DEFINED_IN`…) réelles, tirées du
   catalogue au moment où les `ToolDef` sont générés — un modèle ne peut pas
   inventer `HAS_SIGNALS` si le schéma ne le permet pas, et le 0,5 B l'a
   fait. C'est aussi ce qui rend `search_expand` prenable : Gemini ne l'a
   pas utilisé pour « qu'est-ce que `register_builtins` enregistre ? »,
   que le graphe résolvait en un appel ([06](06-lacher-lagent-sur-notre-code.md) §3).
   Petit (`tools.rs` + `graph_tool.rs`), grande valeur.
2. **La résolution des relations contre la base** ([02](02-fichiers-en-temps-reel-deux-modes-git-et-histoire.md) §4).
   Le mapping global reste en mémoire à chaque analyse ; `edit` ré-ingère
   un fichier mais ne recalcule pas les relations inter-fichiers vers lui.
   Le graphe est sept fois plus petit qu'hier, c'est le moment.
3. **`GitRef`** comme troisième `FileSource`, avec `Commit` / `TOUCHES` et
   un horizon — l'histoire en un appel, et le mode recherche « basé sur un
   git » de Lucie.
4. **Le titre des entités simples dans l'index BM25** — une branche « titre »
   n'est possible aujourd'hui que sur une KB.
5. Retirer `special_ops` de `config.rs` ; relancer l'agent cloud avec les
   `enum` en place et une mission qui exige `search_expand`.

## Moyen terme — avant de livrer quoi que ce soit

- **`Catalog::search` devient un gabarit.** Le monolithe de 450 lignes en
  `search_default.mmd`, `KBConfig` réduite à des défauts de variables ;
  `fusion.rs`, `title_boost`, `content_boost` partent avec. Deux chemins à
  maintenir tant que ce n'est pas fait.
- **Lecteurs de documents** — pdf, docx, pptx, html, csv, sans lib lourde
  (les formats Office sont du ZIP + XML). L'OCR livré couvre les PDF scannés.
- **Le graphe de normalisation de tableurs** — la porte d'entrée de la
  moitié KB, avec les quatre corrections que la pratique impose à la spec de
  février (`../vision_roadmap_09_2026/03`), et le **rapport de validation à
  l'ingestion** (localisé, illustré, avec témoin).
- **`column=` sur `VectorSearchNode`** — une seconde colonne d'embedding à
  l'ingestion pour une branche « vecteur du titre ».
- **Un E2E de fusion inter-KB** (`source_resolved` est en place, testé par le
  monolithe seulement).
- **Le coût par question** (14 000–40 000 jetons) : outils qui rendent peu
  et juste, et un cache de préfixe qui tient (les `ToolDef` sont déjà triés
  pour ça).

## Long terme — la vision (`../vision_roadmap_09_2026/01`)

- **Un agent de code qui vit dans sa base** : il la gère, il construit des
  backends avec la même technologie ; le chaos contrôlé. Ce soir il lit,
  cherche, édite et ré-indexe notre propre code — c'est la première boucle
  fermée.
- **Les deux moitiés** : documents et code d'un côté (fait avancer ce soir),
  catalogues de l'autre (voitures, produits, médicaments, textes de loi —
  la raison d'être des KB, pas encore rouverte).
- **Les produits** : l'agent embarqué (`npm install`, parole locale — TTS/STT
  sur burn, bloqués par le Zipformer sur wgpu), la bibliothèque Rust,
  le cloud multi-tenant, et un jour le substrat navigateur.
- **Aucun Cypher pour les agents, jamais** : ce qui manque à l'abstraction
  s'ajoute à l'abstraction.
- **Ce que l'agent n'arrive pas à faire est la feuille de route.** Ce soir
  ça a donné les `enum`, le « dernier pas », `return_fields`, le « vouliez-
  vous dire » — quatre améliorations en trois questions et deux missions.
  Continuer à le lâcher, régulièrement, sur des tâches réelles.
