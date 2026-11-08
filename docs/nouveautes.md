# Nouveautés

L'inverse d'une feuille de route : ce qui est **atteint**, en une ligne par grande
étape, jamais par commit. Une ligne entre quand une capacité nouvelle est sur
master, prouvée, et qu'on peut la montrer. Les titres sont faits pour accrocher,
le reste de la ligne dit en une phrase ce que ça change ; le hash dit où.

## Novembre 2026

- **8 nov. — Une base rouverte retrouve toujours son index vectoriel.** Toute ouverture charge les
  extensions notées à côté de la base, même quand le journal est vide : une mort juste après un
  point de reprise laissait l'index attendre son extension. Une extension qui ne se charge pas ne
  passe plus inaperçue : la base s'ouvre, et SHOW_INDEXES le dit dans `extension_error`, avec le
  chemin et la cause.
- **8 nov. — Une phrase collée retrouve son fichier, et le surlignage ne demande plus rien à l'hôte.**
  Avec lucivy 4.3.2, une ligne de plus de 100 caractères copiée d'un fichier le ramène premier par
  le signal exact, et une requête entière garde son flou jusqu'à 900 caractères (mesuré : 464 ms à
  933) ; le surlignage des morceaux du seul vecteur lit les positions que le service bge-m3 rend,
  sans tokenizer ni modèle dans l'hôte (139 ms pour trois textes) ; et un second processus qui veut
  écrire une base déjà tenue est refusé en nommant l'écrivain en place (`80ca3d561`, `617de00da`,
  `0a8ff53d9`, `217dd10a1`).
- **8 nov. — Les fichiers d'annulation du dataflow ne s'accumulent plus dans `/tmp`.** À la fin d'un
  graphe, tous les fichiers `*.undo.json` de plus de 64 Ko sont supprimés avec le reste, au lieu d'être
  gardés par défaut (seules les migrations `migration-…` conservent leur annulation pour un retour
  arrière ultérieur) : la fuite de mémoire vive en `/tmp` est étanchée (`b860ea2dd`).
- **8 nov. — Un gros COPY de blobs tient dans un petit tampon.**
  Un seul COPY de 112 blobs d'1 Mio échouait sous 256 Mio de tampon (« buffer pool is full ») ; un
  COPY se borne désormais en octets, vide ses lignes au disque par lots sans attendre un groupe
  plein, les ajoute à la tête née du même COPY par copie-échange (jamais sous les pieds d'un lecteur),
  et écrit son journal par tranches : le COPY passe, forcé comme journalisé, annulé sans laisser une
  page (`dadbf0041`).
- **8 nov. — Un backend dont la racine a disparu le signale nommément.** Quand le dossier racine d'un
  workspace n'existe plus sur le disque (worktree supprimé), `daemon_status` porte « racine absente : <chemin> »
  et chaque outil touchant les fichiers, `index` et `estimate` compris, rend « la racine du dépôt a
  disparu (<chemin>) : rouvrez le backend sur une racine qui existe » plutôt qu'une erreur brute
  d'entrée-sortie (`b0a9ab6e5`, `1b9983b59`, `1de9c908c`).
- **7 nov. — Un appel dans une garde de `match` compte comme usage.** En Rust, `motif if
  verifie(x) =>` : la garde est du code, et son appel apparaît dans `usages` et `impact`. Avant,
  codeparsers prenait la garde pour une partie du motif et l'appel n'existait pas pour l'index
  (`vue_portee`, appelée seulement là, avait 0 usage) ; sur code-lourd, 19 appels de plus et
  aucune locale prise pour un appel. codeparsers 672a0d5.
- **7 nov. — Le flux de l'IDE ne perd plus un événement quand deux relais se croisent.** Un id
  tiré hors du verrou laissait un relais envoyer N+1 avant N, et N se perdait en silence : un
  jeton manquant en direct dans le chat (1 à 3 tours sur 8 sous un modèle rapide), une ligne de
  boîte de l'équipier. L'id se tire désormais sous le verrou des abonnés ; un témoin à quatre fils
  (8 000 publications, 7 577 reçues avant) le garde. Et la barre range chaque backend par la
  nature que son gabarit déclare (`kind`), son dépôt et sa branche lus dans son espace de
  travail. 23757c7d6.
- **7 nov. — Le chat tient l'ordre d'arrivée.** Une réponse de l'agent se lit comme elle
  s'est faite : mots, appel d'outil, mots, appel, réponse finale, pendant le tour comme après un
  rechargement ; chaque outil déplié dit son heure, sa durée et son résultat entier ; la liste
  ne saute plus pendant le flux, ni quand on déplie un outil ; le chat prend tout l'écran.
  3d09b32c2.
- **7 nov. — La barre parle en dépôts.** « Dépôts », « Mémoire », « Projets » : chaque dépôt
  attaché nommé par son dossier et sa branche, les worktrees de ses tâches par la leur ; sous
  chaque conversation, la jauge du contexte (le dernier appel sur le contexte du modèle déclaré),
  ses compteurs et les fiches écrites pendant la conversation ; les runs à rôle (l'archiviste…)
  dans les tâches de fond. Le mot « backend » ne paraît plus à l'écran du chat. 3d09b32c2.
- **7 nov. — Le journal de l'hôte dit l'heure et l'auteur.** Chaque ligne de `daemon.log`,
  celles du moteur comprises, commence par l'heure ISO à la milliseconde, et une ligne née d'un
  appel nomme son client, sa session et son dépôt : une rafale s'attribue. Le relais ne fige
  jamais l'hôte (sortie non bloquante, patience mesurée, lignes perdues comptées). 3d09b32c2.
- **7 nov. — Deux écrivains à la fois ne mêlent plus leurs nœuds sous un COPY de relations.**
  Une transaction qui créait des nœuds puis les reliait par un COPY pouvait, si une autre validait
  des nœuds dans la même table entre-temps, relier en silence les nœuds de l'autre (1000 → 1001 lu
  2000 → 2001) ; le COPY verse désormais les nœuds de la transaction dans leur table avant de
  résoudre la moindre clé, comme un COPY de nœuds le faisait déjà (`c4f8610b9`).
- **7 nov. — La recherche surligne ce que le vecteur a trouvé, filtre juste et ne s'étouffe plus sur un prompt collé.**
  Un morceau trouvé par le seul dense porte ses passages surlignés par bge-m3, calculés à
  l'affichage et nulle part ailleurs (0 appel à l'indexation, prouvé par un service qui compte) ;
  un filtre ne laisse plus passer dans le signal creux les morceaux d'un autre document ; et une
  requête de 200 mots répond en 80 ms au lieu de 26 s, en disant ce qu'elle a coupé (`c5a40d376`,
  `aaf475d4b`, `77d0b0676`).

- **7 nov. — Une lecture survit à l'échec d'une de ses instructions.** Dans une transaction en
  lecture seule, une instruction qui échoue (un index absent, un graphe projeté absent) ne fait plus
  refuser les suivantes : les signaux d'une recherche sous instantané ne tombent plus ensemble.
  Une interruption arrête aussi toute la requête, CREATE réécrit compris ; `17a094d12`.
- **7 nov. — Le Daemon Code Summoning dans le terminal.** Quand l'agent réfléchit, des runes
  tournent autour de ⛧ (« INVOCATION DU CODE… ») ; quand un outil lit, un symbole alchimique se
  transmute ; quand il écrit ou exécute, le Glitch tire « COMPILING CURSE » et ses frères. C'est
  la page de Lucie (ragforge, déc. 2025) refaite en cellules, l'animation choisie selon ce que
  l'outil a le droit de faire. Elle tient sous ssh : 60 à 158 octets par image, au plus 1,6 Ko/s,
  et se dégrade en 256 couleurs, en 16, sans couleur et en ASCII. La vue dit le dépôt, la branche
  et la tâche, et ne montre que son worktree. f66e72d31.
- **7 nov. — Le terminal de l'agent de code a sa vue.** `rag3weaver-tty`, sans option, ouvre une
  vue inline en ratatui et crossterm, la même sous Windows, macOS et Linux. Le texte de l'agent
  coule dans l'historique du terminal (défilement, copie, tmux), et seules trois lignes se
  redessinent en bas : la ligne en cours, l'état (réfléchit, agit, attend), la saisie. Entrée parle
  à l'équipier, Échap l'arrête. L'écran se redessine au plus 30 fois par seconde : 28 octets par
  token mesurés, bornés à 35, pour une connexion ssh. 1fc1e24bd.
- **7 nov. — L'équipier se suit depuis un terminal, même à travers ssh.** `rag3weaver-tty --plain`
  lance une demande à l'agent de code et écrit au fil de l'eau ses tokens, ses outils et sa boîte.
  Après une coupure, il reprend là où il en était, sans perte ni doublon (`Last-Event-ID`, prouvé
  par un flux coupé au milieu d'un run). Il tourne là où est l'hôte, ou sur une autre machine par
  un tunnel. C'est un crate à part, sans le moteur, et chaque outil dit maintenant ce qu'il a le
  droit de faire (`policy` dans `describe`). cfc5cc8e8.
- **7 nov. — Chercher sans avoir ouvert un dossier.** La recherche de l'IDE se tape dès
  l'accueil, sur chaque backend, sur tout l'index ; depuis l'arbre, le dossier courant la
  suit comme un filtre (le `scope` de `search_code`), effaçable d'un clic. Avant, la saisie
  n'apparaissait qu'une fois un dossier ouvert, et ce dossier ne filtrait rien. e2d5c9c88.
- **7 nov. — L'IDE entier est en React.** Après le cadre, le chat, le fichier et la
  recherche du 25 octobre : les tâches de l'agent et leur relecture avec l'équipier, les
  bilans de session par fiche, l'arbre, le symbole rendu section par section, le graphe, le
  DAG vivant, les réactions, les propositions, et l'édition dans la page (CodeMirror du
  magasin embarqué, Ctrl-S sur l'empreinte lue, le conflit montré par sa différence).
  L'ancienne page n'a plus de vue à elle. 43d7489ee, 08d10babf, 7897d2c96, a4ffea748.
- **7 nov. — Un lecteur ne voit plus jamais les lignes d'une écriture annulée.** Un bloc
  entièrement annulé perdait ses versions, ce qui voulait dire « tout validé » pour un lecteur
  qui le tenait encore : 10 + 2 048 relations d'un COPY annulé comptées, 4 fois sur 60. Les
  lignes annulées sont désormais marquées telles, au grain du bloc, nœuds et relations ; `42b85dc62`.
- **7 nov. — Un appel C++ ne voit que ce qu'il inclut.** La résolution suit les #include
  et la déclaration d'en-tête : 299 arêtes justes sur 300 au juge clangd, et une mention
  qui s'abstient se dit au lieu de passer pour devinée. 9fd37ab1c.
- **7 nov. — La mémoire parle à Claude Code.** Des hooks posés dans une seule session
  d'essai, par un script qui crée son worktree : à l'ouverture, qui tu es et ce que tu
  tenais ; à chaque tour, les fiches en rapport, ce que d'autres ont changé dans les
  fichiers que tu touches, le dernier verdict du juge ; après une compaction, le défi
  d'identité ; à 70 % du contexte réel, la suggestion de ranger. Jamais bloquant, muet
  sans hôte. d42787227.
- **7 nov. — Claude parle au trait Llm.** L'API Messages d'Anthropic derrière le même
  trait que Gemini et les modèles locaux, en HTTP brut, réflexion adaptative rejouée,
  refus nommé ; un seul point de branchement rend tout modèle au reste du runtime.
  L'équipier et le juge peuvent désormais tourner sur Claude comme sur Gemini.
  90df16ace.

## Octobre 2026

- **26 oct. — La recherche surligne ce qu'elle a trouvé.** Les positions rendues par
  lucivy arrivent jusqu'à la page, au caractère, dans l'extrait de chaque résultat ; et
  l'avertissement « rien de probant » se tait quand la phrase entière a été trouvée.
  11d350001.
- **26 oct. — Un graphe dans un graphe, une fois par élément.** Un manifeste cite un
  sous-graphe par fichier et l'appelle une fois ou par élément d'un flux, les résultats
  rendus dans l'ordre ; et un flux qui ne se lit pas en une seule valeur est refusé au
  lieu de se taire. 1bd1be619.
- **26 oct. — Créer l'index vectoriel ne fige plus les écritures.** Le CREATE pose
  l'index, attend seulement les écrivains déjà ouverts, puis bâtit pendant que les
  autres écrivent ; un index en construction ou incomplet se dit dans SHOW_INDEXES,
  et la recherche répond par les mots en attendant. Format 43. 97daa41ed.
- **25 oct. — L'IDE change de peau.** Le cadre et le chat passent en React, shadcn/ui
  et lucide : mode sombre, les backends en cartes avec leurs tâches, le chat de la
  tâche et de l'atelier sur la forme du deck builder avec les outils dépliés pendant
  le flux ; le bundle est bâti par le dépôt, embarqué et vérifié, jamais chargé
  d'ailleurs. Les autres vues suivent une à une. 81fdc2f40.
- **25 oct. — La recherche trouve ce qu'on a sous les yeux.** La requête entière part
  à lucivy telle quelle, en parallèle d'une recherche par mots, et l'exact passe
  devant la fusion : « Vue d'ensemble » rend sa page aux rangs 1, 2, 3 au lieu
  d'être absente des trente ; les questions en langue naturelle gardent leur
  3 sur 3. 159087582.
- **25 oct. — L'équipier.** Un agent qui accomplit une tâche seul, par pas, et parle à
  un superviseur par une boîte aux lettres ; un plan quand il le faut, le hors-plan en
  proposition, et le juge à chaque pas ; sur Gemini, la doc d'un dépôt de test en un
  pas, notée 10. 0ab04909a.
- **25 oct. — Un lecteur ouvert avant le point de reprise n'est plus attendu.** Il relit
  sa génération, gardée à côté jusqu'au point de reprise suivant ; un lecteur nouveau voit
  l'écriture. f8c0c8df0.
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
