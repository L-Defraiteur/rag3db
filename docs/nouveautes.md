# Nouveautés

L'inverse d'une feuille de route : ce qui est **atteint**, en une ligne par grande
étape, jamais par commit. Une ligne entre quand une capacité nouvelle est sur
master, prouvée, et qu'on peut la montrer. Les titres sont faits pour accrocher,
le reste de la ligne dit en une phrase ce que ça change ; le hash dit où.

## Octobre 2026

- **25 oct. — Le juge juge aussi les outils.** À chaque run, un second rapport
  par outil appelé : un défaut typé par genre (paramètre manquant ou mal nommé,
  retour bavard, hors sujet, incohérent…) et par gravité, avec sa preuve exacte et
  la forme corrigée, rangé dans la fiche de l'outil ; sur Gemini, sept défauts
  nommés sur un outil volontairement faux. bdf5929c7.
- **25 oct. — Un point de reprise ne fige plus les lecteurs.** Une lecture entre,
  lit le dernier validé et rend la main pendant qu'il s'écrit ; les blocs neufs se
  bâtissent hors des verrous et s'échangent sous eux. 66994dd43.
- **25 oct. — Le juge de session.** Un agent relit chaque tour du chat par son
  journal, rejoue ce qui lit, vérifie le résultat, se compare aux sessions
  précédentes de la même tâche et range sa fiche de bilan dans la mémoire ; sur
  Gemini, il nomme la description d'outil qui a trompé l'agent. 4b64a98e5.
- **25 oct. — Le chat code dans une tâche.** Sur un dépôt étranger ouvert dans
  l'IDE, le chat lit, cherche et édite le code sur la branche de la tâche, jamais
  ailleurs, et mène à sa relecture ; Gemini le fait en 13 s. ae127d634.
- **25 oct. — Un dossier qui dit de quoi il parle.** Chaque fiche porte la version
  de son format, et le dossier un `manifeste.yaml` : une base reconstruite après un
  changement de format relit les anciens fichiers par leurs migrations, ou dit
  lesquels elle ne sait pas lire. bea6c6a98.
- **25 oct. — La recherche de l'IDE montre ce que l'hôte sait.** Chaque résultat
  avec son score et ce que chaque signal en a dit, l'état de l'index en tête, les
  replis en pied, et une case pour faire relire. b0e5451d0.
- **25 oct. — Une recherche ne peut plus tuer un écrivain.** Le plein texte (lucivy
  4.3.1) ne panique plus sous validations concurrentes, et une recherche en échec
  ne fait plus perdre d'écriture ni geler une validation ; prouvé 80 fois sur 80
  sous charge et par le témoin réel à quatre lecteurs. 277f8b11d.
- **25 oct. — La mémoire dans des fichiers qu'on lit.** Chaque fiche est un fichier
  Markdown (son état, ses ancres, ses apports) ; si la base se perd, elle revient
  entière de son dossier. d72bcf5e0.
- **25 oct. — Le plan d'un fichier dans l'IDE.** La fiche d'un fichier montre ses
  scopes imbriqués, rendus par l'outil `outline` en JSON (un Plan déclaré), chacun
  vers sa ligne et son symbole, avec ce que vaut cette lecture. cbb41d312.
- **25 oct. — La recherche dit ce que chaque signal a trouvé.** Chaque résultat porte
  le score et le rang du plein texte et des vecteurs, et `search_code` rend sa méta
  en JSON. 0427c2c2d.
- **25 oct. — Les grammaires d'aujourd'hui.** tree-sitter 0.27 : les enums du pont
  cxx rattachés à leur module, des fermetures devenues fonctions nommées, les
  méthodes de snappy à leur classe ; un recul connu dans fmt, en ticket. dfb706b07.
- **25 oct. — Les requêtes préparées ne s'accumulent plus.** Une préparation porte
  sa requête et la rend en mourant : quatre lecteurs d'un vrai dépôt tiennent dans
  7,1 Go au lieu de 30. fb4d83b65.
- **25 oct. — Un service revenu est repris sans rien redémarrer.** Un backend
  ouvert sans son service d'embarquement le retente, et se rouvre de lui-même dès
  qu'il répond : les vecteurs reprennent, l'avertissement dit toujours le vrai.
  2a5a38cb0.
- **25 oct. — Aucune écriture du plein texte ne se perd en silence.** Chaque index
  finit par compter ses documents contre les lignes de la base et dit l'écart, au
  journal et dans l'état. 4d1a4a0ee.
- **25 oct. — La recherche tient sans ses vecteurs.** Pendant un rattrapage,
  `search_code` répond par les mots et dit pourquoi le dense manque, au lieu de
  tomber en entier. d6abc051f.
- **25 oct. — Ce que la mémoire sait d'un fichier.** `cited_by` rend les fiches qui
  citent un chemin, un dossier, un symbole ou un source, avec leurs ancres et les
  apports qui le citent, lues par un Plan déclaré, sans Cypher. 665a4a97c.
- **25 oct. — Trier par ce qui arrive aux liens.** Un Plan déclaré s'ordonne par le
  max ou le min d'un champ des lignes liées (les fiches par la date de leur dernier
  apport), en Cypher comme en SQL. beda3d8f1.
- **25 oct. — Les colonnes gardent leur numéro pour toujours.** Une colonne
  supprimée laisse un trou à sa place au lieu de faire renuméroter les autres au
  point de reprise : plus rien ne bouge sous les plans liés, première marche vers
  un point de reprise qui ne fige personne (V2-1a, format 42). 4ae060070.
- **25 oct. — Le raisonnement se règle et se compte.** Chaque modèle déclare son
  budget de raisonnement (`models.llm.reasoning`, absent par défaut), et un run
  compte à part les jetons de raisonnement et ceux servis par le cache. 527a0356c.
- **25 oct. — Le chat de l'atelier est dans l'IDE.** Un agent qui lit tes gabarits
  et propose une modification depuis le navigateur, rien d'écrit avant Appliquer,
  jugé sur le résultat avec Gemini (« fais compter le compteur par cinq » : l'aperçu
  compte par cinq). a13b22417, 657e568bd, a454d6a91.
- **25 oct. — Le premier outil sans une ligne de Cypher.** `outline`, le plan d'un
  fichier, est un Plan déclaré en JSON, rendu dans les deux dialectes (rag3db et
  PostgreSQL) par l'IR du Plan. 89b5d5dbd, sur c1e6f7dd1.
- **25 oct. — Une base de fiches se reconstruit depuis ses fichiers.** Les fiches
  mémoire vivent en fichiers lisibles (en-tête YAML, bloc machine invisible), et la
  base perdue se refait depuis le dossier, à parité champ à champ. 80ea996c2.
- **25 oct. — Tout run s'arrête d'un clic.** Un seul chemin d'arrêt pour tout ce qui
  tourne dans l'hôte, et le bouton Arrêter dans le DAG vivant. 95c985c4c.
- **25 oct. — Un oracle juge nos relations.** clangd compare 300 relations C++ à la
  vérité du compilateur : 73 % justes, 5 % fausses sous le juge strict (218 et 14 sur 300), et on sait lesquelles. Rust,
  TypeScript, Python, Go suivent. Branche codeparsers/oracle-clangd.
- **25 oct. — lucivy ne se fige plus.** Le plein texte ne se bloque plus sous lecteurs
  et réindexation, et un shard sans écrivain ne perd plus d'écriture ; pris par une
  branche en attendant la publication. 44c790c20.
- **25 oct. — Le moteur sait quel moteur a écrit la base.** La version du format de
  stockage est posée à la main, l'en-tête et le journal nomment le bâti, et un
  moteur trop vieux refuse en le disant. f84433579.
- **19 oct. — Les lectures n'attendent plus la montre.** Pendant qu'un index
  s'écrit, chaque lecture voit l'état d'avant en moins de 100 ms (max 97 ms contre
  1 282). L4, L5, L6 : a5a6e63e6, b2ee0f8ce, 27f80e5ed.
- **19 oct. — Une relation douteuse se compte et se dit, elle ne se devine pas.**
  Plus de résolution par le fichier, le dossier ou le nom seul : la mesure sur une
  vraie base a refusé trois fois l'inverse. 9b22ef8d7, c2ef8a941.
- **19 oct. — Le catalogue dit tout.** Chaque chemin d'écriture publie sur le bus,
  en JSON, ce qu'il a réellement écrit. 60348d75a.
- **18 oct. — Le DAG vivant dans le navigateur.** Les nœuds s'allument pendant
  qu'un outil tourne, les composants React se rechargent à chaud, et le même IDE
  s'ouvre sur un dépôt étranger. C1, C2, lots 3 et 4, c15915b73.
- **18 oct. — L'hôte dort et se réveille.** L'hôte vit derrière une socket
  systemd : il s'arrête après 30 minutes d'inactivité et revient à la première
  connexion, avec un jeton stable. 
- **12 oct. — Un magasin de libs.** Chaque lib du moteur a son dossier, les
  binaires la trouvent par RUNPATH, et `courant` se pose seulement après le
  verdict.
- **11 oct. — Plusieurs écrivains par défaut.** Le moteur accepte plusieurs
  écrivains, avec des verrous nommés ; le second attend au lieu d'échouer.
  0a630b764.
